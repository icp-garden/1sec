use std::fmt::Display;

use candid::CandidType;
use evm_rpc_types::{BlockTag, HttpOutcallError, RpcError};
use ic_canister_log::log;
use serde::Deserialize;

use crate::{
    api::types::EvmChain,
    evm::{
        evm_rpc::{build_evm_rpc_client, consensus, map_multi_rpc, ConsensusResult},
        reader::{
            schedule_tx_logs_task_if_unconfirmed_blocks,
            schedule_tx_logs_task_sooner_if_too_many_blocks,
        },
        state::{mutate_evm_state, read_evm_state},
        writer, TxHash,
    },
    flow::state::FlowId,
    logs::DEBUG,
    metrics::CanisterCall,
    numeric::{BlockNumber, Timestamp},
    task::{schedule_after, schedule_now, timestamp_ms},
};

use super::{
    head::{estimated_time_for_n_blocks, Head},
    proof::ValidatedBlock,
    ValidatedProof,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    FetchLatestBlock,
    FetchSafeBlock,
}

impl Task {
    pub async fn run(self, chain: EvmChain) -> Result<(), String> {
        match self {
            Self::FetchLatestBlock => fetch_latest_block_task(chain).await,
            Self::FetchSafeBlock => fetch_safe_block_task(chain).await,
        }
    }

    pub fn get_all_tasks(chain: EvmChain) -> Vec<crate::task::TaskType> {
        vec![
            Task::FetchLatestBlock.wrap(chain),
            Task::FetchSafeBlock.wrap(chain),
        ]
    }

    pub fn wrap(self, chain: EvmChain) -> crate::task::TaskType {
        crate::task::TaskType::Evm {
            chain,
            task: crate::evm::Task::Prover(self),
        }
    }
}

#[derive(Debug)]
enum FetchBlockError {
    RpcError(RpcError),
    Validation(String),
    Consensus(String),
}

impl Display for FetchBlockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchBlockError::RpcError(rpc_error) => rpc_error.fmt(f),
            FetchBlockError::Validation(err) => err.fmt(f),
            FetchBlockError::Consensus(err) => err.fmt(f),
        }
    }
}

pub async fn fetch_latest_block_task(chain: EvmChain) -> Result<(), String> {
    let threshold = read_evm_state(chain, |s| s.evm_rpc.consensus_threshold);

    let latest = read_evm_state(chain, |s| s.prover.head.latest.clone());
    let hint = read_evm_state(chain, |s| s.prover.head.hint.clone());

    let previous = latest
        .as_ref()
        .map(|x| x.block_number)
        .unwrap_or(BlockNumber::ZERO);

    let next_latest = estimate_next_block_number(chain, latest.clone());
    let next_hint = estimate_next_block_number(chain, hint.clone());

    log!(
        DEBUG,
        "[{:?}]: fetching latest: fetching latest={:?}, hint={:?}",
        chain,
        next_latest,
        next_hint,
    );

    let next = match (next_latest, next_hint) {
        (Some(next_latest), Some(next_hint)) => match (latest, hint) {
            (Some(latest), Some(hint)) => {
                if latest.block_number > hint.block_number {
                    next_latest
                } else {
                    next_hint
                }
            }
            _ => next_latest.max(next_hint),
        },
        (Some(next_latest), None) => next_latest,
        (None, Some(next_hint)) => next_hint,
        (None, None) => {
            let block = fetch_block(chain, BlockTag::Latest, 1)
                .await
                .map_err(|err| err.to_string())?;
            block.number.max(previous.add(
                BlockNumber::ONE,
                "BUG: fetch_latest_block_task: overflow in previous+1",
            ))
        }
    };

    match fetch_block(chain, BlockTag::Number(next.into_inner().into()), threshold).await {
        Ok(block) => {
            log!(DEBUG, "[{:?}]: fetched latest: {}", chain, block.number);
            let new_head = Head {
                block_number: block.number,
                block_hash: block.hash,
                fetch_time: timestamp_ms(),
            };
            handle_fetched_latest_block(chain, new_head);
            schedule_next_fetch_latest_block_task(chain);
            if next_safe_block_to_fetch(chain).is_some() {
                schedule_now(
                    Task::FetchSafeBlock.wrap(chain),
                    "fetched new latest".into(),
                );
            }
            schedule_tx_logs_task_if_unconfirmed_blocks(chain);
            Ok(())
        }
        Err(FetchBlockError::RpcError(RpcError::JsonRpcError(rpc_err))) => {
            if rpc_err.message.contains("block number") && rpc_err.message.contains("not found") {
                handle_missing_latest_block(chain);
            }
            schedule_now(Task::FetchLatestBlock.wrap(chain), "missed block".into());
            Ok(())
        }
        Err(FetchBlockError::RpcError(RpcError::HttpOutcallError(
            HttpOutcallError::InvalidHttpJsonRpcResponse {
                status,
                body: _,
                parsing_error: Some(msg),
            },
        ))) => {
            if status == 200 && msg.contains("invalid type: null") {
                handle_missing_latest_block(chain);
            }
            schedule_now(Task::FetchLatestBlock.wrap(chain), "missed block".into());
            Ok(())
        }
        Err(FetchBlockError::Consensus(err)) => {
            log!(
                DEBUG,
                "[{:?}]: consensus failed when fetching {}: {}",
                chain,
                next.into_inner(),
                err
            );
            handle_missing_latest_block(chain);
            schedule_now(
                Task::FetchLatestBlock.wrap(chain),
                "missed block: consensus".into(),
            );
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}

pub async fn fetch_safe_block_task(chain: EvmChain) -> Result<(), String> {
    let threshold = read_evm_state(chain, |s| s.evm_rpc.consensus_threshold);
    if let Some(next) = next_safe_block_to_fetch(chain) {
        log!(DEBUG, "[{:?}]: fetching safe: {}", chain, next);
        let block = fetch_block(chain, BlockTag::Number(next.into_inner().into()), threshold)
            .await
            .map_err(|err| err.to_string())?;
        log!(DEBUG, "[{:?}]: fetched safe: {}", chain, next);
        handle_fetched_safe_block(chain, block);
        if next_safe_block_to_fetch(chain).is_some() {
            schedule_now(Task::FetchSafeBlock.wrap(chain), "recurring".into());
        }
        schedule_tx_logs_task_sooner_if_too_many_blocks(chain);
    }
    Ok(())
}

async fn fetch_block(
    chain: EvmChain,
    block: BlockTag,
    threshold: usize,
) -> Result<ValidatedBlock, FetchBlockError> {
    let evm_rpc = read_evm_state(chain, |s| s.evm_rpc.clone());
    let evm_rpc_client = build_evm_rpc_client(&evm_rpc, threshold);

    let cc = CanisterCall::new(
        evm_rpc.evm_rpc_canister_id,
        "eth_get_block_by_number",
        evm_rpc.evm_rpc_canister_cycles,
    );

    let result = evm_rpc_client.eth_get_block_by_number(block.clone()).await;

    let result = map_multi_rpc(result, |r| match r {
        Ok(block) => Ok(ValidatedBlock::try_from(block)),
        Err(err) => Err(err),
    });

    match consensus(result, threshold) {
        ConsensusResult::Consensus(Ok(Ok(block))) => {
            cc.returned_ok();
            Ok(block)
        }
        ConsensusResult::Consensus(Ok(Err(err))) => {
            cc.returned_err(&err);
            Err(FetchBlockError::Validation(err))
        }
        ConsensusResult::Consensus(Err(err)) => {
            cc.returned_err(err.to_string());
            Err(FetchBlockError::RpcError(err))
        }
        ConsensusResult::NoConsensus(items) => {
            let err = format!(
                "prover: failed to fetch block {:?}: consensus={}, threshold={}",
                block,
                items.len(),
                threshold,
            );
            cc.returned_err(&err);
            Err(FetchBlockError::Consensus(err))
        }
    }
}

fn handle_fetched_latest_block(chain: EvmChain, latest: Head) {
    let min_ms = read_evm_state(chain, |s| s.prover.head.config.block_time_min).as_millis() as u64;
    let avg_ms = read_evm_state(chain, |s| s.prover.head.config.block_time_avg).as_millis() as u64;
    let old_time = read_evm_state(chain, |s| s.prover.head.block_time_ms);
    let percent = read_evm_state(chain, |s| s.prover.head.config.block_time_after_hit);
    let new_time = (old_time as f64 * percent.as_f64()).round() as u64;

    mutate_evm_state(chain, |s| {
        // Reduce the time to the average time for subsequent blocks.
        s.prover.head.block_time_ms = new_time.max(min_ms).min(avg_ms);
        s.prover.head.latest = Some(latest);
    });

    prune_stale_blocks(chain);
}

fn handle_missing_latest_block(chain: EvmChain) {
    let min_ms = read_evm_state(chain, |s| s.prover.head.config.block_time_min).as_millis() as u64;
    let max_ms = read_evm_state(chain, |s| s.prover.head.config.block_time_max).as_millis() as u64;
    let old_time = read_evm_state(chain, |s| s.prover.head.block_time_ms);
    let percent = read_evm_state(chain, |s| s.prover.head.config.block_time_after_miss);
    let new_time = (old_time as f64 * percent.as_f64()).round() as u64;

    mutate_evm_state(chain, |s| {
        s.prover.head.block_time_ms = new_time.max(min_ms).min(max_ms)
    });
}

fn handle_fetched_safe_block(chain: EvmChain, block: ValidatedBlock) {
    let safe = Head {
        block_number: block.number,
        block_hash: block.hash,
        fetch_time: timestamp_ms(),
    };
    mutate_evm_state(chain, |s| s.prover.head.safe = Some(safe));
    confirm_block(chain, block);
    apply_confirmed_proofs(chain);
    prune_stale_blocks(chain);
}

fn confirm_block(chain: EvmChain, block: ValidatedBlock) {
    mutate_evm_state(chain, |s| {
        let hash = block.hash;
        if !s.prover.forest.blocks.contains_key(&hash) {
            s.prover.forest.add_validated_block(block);
        }
        s.prover.forest.add_confirmation(hash)
    });
}

pub fn apply_confirmed_proofs(chain: EvmChain) {
    let proofs = mutate_evm_state(chain, |s| {
        std::mem::take(&mut s.prover.forest.confirmed_proofs)
    });
    writer::apply_confirmed_proofs(chain, proofs);
}

pub fn estimate_next_block_number(chain: EvmChain, latest: Option<Head>) -> Option<BlockNumber> {
    let latest = latest?;
    let block_time_ms = read_evm_state(chain, |s| s.prover.head.block_time_ms);
    let now = timestamp_ms();
    let elapsed_ms = now
        .checked_sub(latest.fetch_time)
        .unwrap_or(Timestamp::ZERO)
        .into_inner();
    let elapsed_blocks = BlockNumber::new(elapsed_ms / block_time_ms);
    latest
        .block_number
        .checked_add(elapsed_blocks.max(BlockNumber::ONE))
}

pub fn next_useful_latest(chain: EvmChain) -> Option<BlockNumber> {
    let safety_margin = read_evm_state(chain, |s| s.prover.head.config.safety_margin);
    let latest = read_evm_state(chain, |s| {
        s.prover.head.latest.as_ref().map(|x| x.block_number)
    })?;

    let safe = latest.checked_sub(safety_margin)?;

    let unconfirmed = read_evm_state(chain, |s| {
        let a = s.prover.forest.first_unconfirmed_height_after(safe);
        let b = s.reader.first_unconfirmed_height_after(safe);
        match (a, b) {
            (None, None) => None,
            (None, Some(b)) => Some(b),
            (Some(a), None) => Some(a),
            (Some(a), Some(b)) => Some(a.min(b)),
        }
    })?;

    unconfirmed.checked_add(safety_margin)
}

fn blocks_until_first_unconfirmed_becomes_safe(chain: EvmChain) -> Option<BlockNumber> {
    if read_evm_state(chain, |s| {
        s.writer.pending.is_empty() && s.reader.unconfirmed_blocks.is_empty()
    }) {
        return None;
    }

    let latest = read_evm_state(chain, |s| {
        s.prover.head.latest.as_ref().map(|x| x.block_number)
    })?;

    let blocks = next_useful_latest(chain)?.checked_sub(latest)?;

    if blocks > BlockNumber::ZERO {
        Some(blocks)
    } else {
        None
    }
}

fn schedule_next_fetch_latest_block_task(chain: EvmChain) {
    let safety_margin = read_evm_state(chain, |s| s.prover.head.config.safety_margin);
    let blocks = blocks_until_first_unconfirmed_becomes_safe(chain).unwrap_or(safety_margin.mul(
        10_u64,
        "BUG: last_unconfirmed_height_between: overflow in safety_margin * 10",
    ));
    let delay = estimated_time_for_n_blocks(chain, blocks);
    log!(
        DEBUG,
        "[{:?}]: schedule fetch latest: delay={}s",
        chain,
        delay.as_secs()
    );
    schedule_after(
        delay,
        Task::FetchLatestBlock.wrap(chain),
        format!("recurring: {}", blocks),
    );
}

fn next_safe_block_to_fetch(chain: EvmChain) -> Option<BlockNumber> {
    let safe_to_latest_blocks = read_evm_state(chain, |s| s.prover.head.config.safety_margin);

    let latest = read_evm_state(chain, |s| {
        s.prover.head.latest.as_ref().map(|x| x.block_number)
    })?;

    let old_safe = read_evm_state(chain, |s| {
        s.prover
            .head
            .safe
            .as_ref()
            .map(|x| x.block_number)
            .unwrap_or(BlockNumber::ZERO)
    });

    let new_safe = latest.checked_sub(safe_to_latest_blocks)?;

    if old_safe >= new_safe {
        return None;
    }

    Some(new_safe)
}

/// Returns flow ids of proofs attaches to the block with the given hash.
fn proof_ids(chain: EvmChain, hash: &TxHash) -> Vec<FlowId> {
    read_evm_state(chain, |s| {
        s.prover
            .forest
            .blocks
            .get(hash)
            .map(|b| {
                b.unconfirmed_proofs
                    .iter()
                    .map(|p| match p {
                        ValidatedProof::TxReceipt { id, .. } => *id,
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Returns hashes of blocks with the lowest block number.
fn oldest_blocks(chain: EvmChain) -> Option<Vec<TxHash>> {
    read_evm_state(chain, |s| {
        s.prover
            .forest
            .blocks_by_height
            .first_key_value()
            .map(|(_, v)| v.clone())
    })
}

/// Removes old blocks that either do not have any proofs attached or whose
/// proofs are not pending anymore in [evm::writer].
pub fn prune_stale_blocks(chain: EvmChain) {
    while let Some(hashes) = oldest_blocks(chain) {
        for hash in hashes.iter() {
            let ids = proof_ids(chain, hash);
            let any_pending = read_evm_state(chain, |s| {
                ids.iter().any(|id| s.writer.pending.contains_key(id))
            });
            if any_pending {
                return;
            }
        }

        mutate_evm_state(chain, |s| s.prover.forest.blocks_by_height.pop_first());

        for hash in hashes {
            mutate_evm_state(chain, |s| s.prover.forest.blocks.remove(&hash));
        }
    }
}
