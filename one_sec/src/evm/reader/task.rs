use candid::CandidType;
use evm_rpc_types::{BlockTag, GetLogsArgs, LogEntry};
use ic_canister_log::log;
use ic_ethereum_types::Address;
use serde::Deserialize;
use std::collections::VecDeque;

use crate::{
    api::types::EvmChain,
    event::process_event,
    evm::{
        self,
        evm_rpc::{build_evm_rpc_client, consensus, is_response_too_large, ConsensusResult},
        prover,
        reader::Event,
        state::read_evm_state,
        tx::TxHash,
    },
    logs::{DEBUG, ERROR},
    metrics::CanisterCall,
    numeric::{BlockNumber, TxLogIndex},
    task::{schedule_after, schedule_now},
};

use super::event::{TxLog, TxLogId};

/// A task of the reader state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    // A task that fetches new event logs.
    FetchTxLogs,
}

impl Task {
    pub async fn run(self, chain: EvmChain) -> Result<(), String> {
        match self {
            Self::FetchTxLogs => fetch_tx_logs_task(chain).await,
        }
    }

    pub fn get_all_tasks(chain: EvmChain) -> Vec<crate::task::TaskType> {
        vec![Task::FetchTxLogs.wrap(chain)]
    }

    pub fn wrap(self, chain: EvmChain) -> crate::task::TaskType {
        crate::task::TaskType::Evm {
            chain,
            task: crate::evm::Task::Reader(Task::FetchTxLogs),
        }
    }
}

pub async fn fetch_tx_logs_task(chain: EvmChain) -> Result<(), String> {
    // The task and panic guards are on the caller side.

    let config = read_evm_state(chain, |s| s.reader.config.clone());
    let evm_rpc = read_evm_state(chain, |s| s.evm_rpc.clone());
    let evm_rpc_client = build_evm_rpc_client(&evm_rpc, evm_rpc.consensus_threshold);

    let safety_margin = read_evm_state(chain, |s| s.prover.head.config.safety_margin);

    // Use `prover.head.latest - safety_margin` instead of `prover.head.safe` to
    // reduce latency by skipping `prover::Task::FetchSafeBlock`.
    // It is safe because fetching logs doesn't not depend on the hash of the
    // safe block, it depends only on its number.
    let safe_block = match read_evm_state(chain, |s| {
        s.prover
            .head
            .latest
            .as_ref()
            .and_then(|x| x.block_number.checked_sub(safety_margin))
    }) {
        Some(safe) => safe,
        None => {
            schedule_now(
                prover::Task::FetchLatestBlock.wrap(chain),
                "fetch tx logs: no latest block".into(),
            );
            schedule_after(
                config.fetch_tx_logs_task_delay,
                Task::FetchTxLogs.wrap(chain),
                "recurring: no safe block".into(),
            );
            return Ok(());
        }
    };

    let last_fully_fetched_block = read_evm_state(chain, |s| s.reader.last_fully_fetched_block)
        .unwrap_or(
            config
                .initial_block
                .map(|x| x.sub(BlockNumber::ONE, "BUG: underflow in initial_block - 1"))
                .unwrap_or(
                    safe_block
                        .checked_sub(BlockNumber::new(
                            config.num_blocks_to_fetch_initially as u64,
                        ))
                        .unwrap_or(BlockNumber::ZERO),
                ),
        );

    let from = last_fully_fetched_block
        .increment("BUG: overflow fetch_logs_task: last_fully_ingested_block++");
    let to = safe_block.increment("BUG: overflow in fetch_log_task: safe_block++");

    if from > to {
        return Err(format!(
            "BUG: fetch_logs_task: last ingested block is unsafe: {} > {}",
            from, to
        ));
    }

    if from == to {
        schedule_now(
            prover::Task::FetchLatestBlock.wrap(chain),
            "fetch tx logs: from==to".into(),
        );
        schedule_after(
            config.fetch_tx_logs_task_delay,
            Task::FetchTxLogs.wrap(chain),
            "recurring: from==to".into(),
        );
        return Ok(());
    }

    // This queue contains block ranges to be fetched.
    // Each range is defined by `[from..to)`, where
    // - `from` is the first block to be fetched in the range.
    // - `to` is one greater than the last block to be fetched.
    // In other words, `first` is inclusive and `to` is not inclusive bound.
    // Invariant: `from < to` for all entries in the queue.
    let mut to_fetch = VecDeque::new();

    if from < to {
        to_fetch.push_back((from, to));
    }

    while let Some((from, to)) = to_fetch.pop_front() {
        log!(DEBUG, "[{:?}]: fetch tx logs [{}, {})", chain, from, to);
        let count = to.sub(from, "BUG: invalid block range in fetch_log_task");
        assert_ne!(count, BlockNumber::ZERO);

        let max_count = BlockNumber::new(config.max_num_blocks_to_fetch_per_call as u64);
        if count > max_count {
            let split = from.add(
                max_count,
                "BUG: fetch_tx_logs_task: overflow in from + max_count",
            );
            to_fetch.push_front((split, to));
            to_fetch.push_front((from, split));
            continue;
        }

        let to_minus_one = to.decrement("BUG: underflow in fetch_logs_task: to--");

        let (addresses, topics) = read_evm_state(chain, |s| s.reader.contracts_and_topics());

        let cc = CanisterCall::new(
            evm_rpc.evm_rpc_canister_id,
            "eth_get_logs",
            evm_rpc.evm_rpc_canister_cycles,
        );

        let result = evm_rpc_client
            .eth_get_logs(GetLogsArgs {
                from_block: Some(BlockTag::Number(from.into_inner().into())),
                to_block: Some(BlockTag::Number(to_minus_one.into_inner().into())),
                addresses,
                topics: Some(topics),
            })
            .await;

        match consensus(result, evm_rpc.consensus_threshold) {
            ConsensusResult::Consensus(Ok(log_entries)) => {
                let parsed: Vec<_> = log_entries
                    .into_iter()
                    .map(parse)
                    .collect::<Result<_, _>>()?;
                ensure_all_unique(chain, parsed.iter().map(|x| x.id).collect())?;
                process_tx_logs(chain, to_minus_one, parsed);
                cc.returned_ok();
            }
            ConsensusResult::Consensus(Err(err)) => {
                if is_response_too_large(&err) {
                    cc.returned_err("response too large");
                    if count >= BlockNumber::new(2) {
                        let (left, right) = split_block_range_in_half(from, to);
                        // The order of pushes is important here!
                        to_fetch.push_front(right);
                        to_fetch.push_front(left);
                    } else {
                        return Err(format!(
                            "BUG: response is too large in a single block: {}",
                            from
                        ));
                    }
                } else {
                    cc.returned_err(err.to_string());
                    return Err(format!(
                        "Failed to fetch logs of block range [{}, {}): {}",
                        from, to, err
                    ));
                }
            }
            ConsensusResult::NoConsensus(_items) => {
                let err = format!(
                    "failed to reach consensus with threshold: {}",
                    evm_rpc.consensus_threshold
                );
                cc.returned_err(&err);
                return Err(err);
            }
        }
    }

    schedule_after(
        config.fetch_tx_logs_task_delay,
        Task::FetchTxLogs.wrap(chain),
        "recurring".into(),
    );

    schedule_tx_logs_task_sooner_if_too_many_blocks(chain);

    schedule_tx_logs_task_if_unconfirmed_blocks(chain);

    Ok(())
}

/// Schedule the `Task::FetchTxLogs` task sooner if it has fallen too far
/// behind the safe block.
pub fn schedule_tx_logs_task_sooner_if_too_many_blocks(chain: EvmChain) {
    let safe_block = read_evm_state(chain, |s| s.prover.head.safe.clone());
    let last_block = read_evm_state(chain, |s| s.reader.last_fully_fetched_block);
    let max_blocks = read_evm_state(chain, |s| s.reader.config.max_num_blocks_to_fetch_per_call);

    if let Some(safe_block) = safe_block {
        if let Some(last_block) = last_block {
            let todo = safe_block
                .block_number
                .checked_sub(last_block)
                .unwrap_or_default();
            if todo.into_inner() as usize > max_blocks / 2 {
                schedule_now(Task::FetchTxLogs.wrap(chain), "too many blocks".into());
            }
        }
    }
}

/// Schedule the `Task::FetchTxLogs` task now if there are unconfirmed blocks
/// with transaction logs that can be confirmed with the currently fetched
/// latest block.
pub fn schedule_tx_logs_task_if_unconfirmed_blocks(chain: EvmChain) -> Option<()> {
    let safety_margin = read_evm_state(chain, |s| s.prover.head.config.safety_margin);
    let unconfirmed = read_evm_state(chain, |s| s.reader.unconfirmed_blocks.first().cloned())?;
    let next = unconfirmed.checked_add(safety_margin)?;
    let latest = read_evm_state(chain, |s| {
        s.prover.head.latest.as_ref().map(|x| x.block_number)
    })?;
    if next <= latest {
        schedule_now(Task::FetchTxLogs.wrap(chain), "unconfirmed tx logs".into());
    }
    Some(())
}

/// Processes the fetched event logs and sends events to other state machines.
pub fn process_tx_logs(chain: EvmChain, last: BlockNumber, tx_logs: Vec<TxLog>) {
    for tx_log in tx_logs {
        process_event(
            Event::FetchedTxLog {
                block_number: tx_log.block,
                tx_log_id: tx_log.id,
            }
            .wrap(chain),
        );
        let token = read_evm_state(chain, |s| s.reader.token(&tx_log.contract));
        match token {
            Some(token) => match evm::ledger::process_tx_log(chain, token, tx_log) {
                Ok(()) => {}
                Err(err) => {
                    log!(ERROR, "BUG: [{:?}]: process_tx_log failed: {}", chain, err);
                }
            },
            None => {
                log!(
                    ERROR,
                    "BUG: [{:?}]: missing token for log: {:?}",
                    chain,
                    tx_log
                );
            }
        }
    }

    process_event(Event::FetchedBlock(last).wrap(chain));
}

/// Splits the given block range `[from..to)` into two halves.
/// Precondition: there must be at least two blocks in the range,
/// i.e. `to - from >= 2`
fn split_block_range_in_half(
    from: BlockNumber,
    to: BlockNumber,
) -> ((BlockNumber, BlockNumber), (BlockNumber, BlockNumber)) {
    let count = to.sub(from, "BUG: underflow in split_block_range_in_half");
    assert!(
        count >= BlockNumber::new(2),
        "BUG: split_block_range_in_half is called on an unsplittable range"
    );
    let half = count.div_by_two();
    let middle = from.add(half, "BUG: overflow in split_block_range_in_half");
    ((from, middle), (middle, to))
}

/// Partially parses the event log entry.
fn parse(entry: LogEntry) -> Result<TxLog, String> {
    let block: BlockNumber = entry
        .block_number
        .ok_or_else(|| "BUG: empty block number in log entry".to_string())?
        .try_into()
        .map_err(|err| format!("BUG: block number is too large: {}", err))?;

    let tx_hash = TxHash(
        entry
            .transaction_hash
            .ok_or_else(|| "BUG: empty transaction hash in log entry".to_string())?
            .into(),
    );
    let index: TxLogIndex = entry
        .log_index
        .ok_or_else(|| "BUG: empty log index in log entry".to_string())?
        .try_into()
        .map_err(|err| format!("BUG: log index is too large: {}", err))?;

    let id = TxLogId { tx_hash, index };

    if entry.removed {
        return Err(format!("BUG: log entry is removed: {:?}", id));
    }

    if entry.topics.len() != 1 {
        return Err(format!(
            "BUG: unexpected number of topics {} in log entry: {:?}",
            entry.topics.len(),
            id
        ));
    }

    Ok(TxLog {
        id,
        block,
        contract: Address::new(entry.address.into()),
        topic: entry.topics[0].clone().into(),
        data: entry.data.into(),
    })
}

/// Checks that there are not duplicates in the given vector of event log ids.
pub fn ensure_all_unique(chain: EvmChain, mut ids: Vec<TxLogId>) -> Result<(), String> {
    ids.sort();

    read_evm_state(chain, |s| {
        for id in ids.iter() {
            if s.reader.done.contains(id) {
                return Err(format!("BUG: duplicate log entry {:?}", id));
            }
        }
        Ok(())
    })?;
    for i in 1..ids.len() {
        if ids[i - 1] == ids[i] {
            return Err(format!("BUG: duplicate log entry {:?}", ids[i]));
        }
    }
    Ok(())
}
