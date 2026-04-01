use candid::{CandidType, Nat};
use evm_rpc_types::{
    BlockTag, FeeHistory, FeeHistoryArgs, HttpOutcallError, Nat256, RpcError,
    SendRawTransactionStatus,
};
use ic_canister_log::log;
use ic_cdk::api::call::RejectionCode;
use ic_management_canister_types_private::DerivationPath;
use ic_secp256k1::PublicKey;
use num_traits::ToPrimitive;
use serde::Deserialize;
use std::collections::VecDeque;

use crate::{
    api::types::EvmChain,
    event::process_event,
    evm::{
        self,
        evm_rpc::{build_evm_rpc_client, consensus, map_multi_rpc, pick_any_ok, ConsensusResult},
        fee::{FeeEstimate, TxFee},
        ledger,
        prover::{endpoint::notify_latest_block, ValidatedProof},
        reader::TxLogId,
        state::{mutate_evm_state, read_evm_state},
        tx::{
            wrap_signature, AccessList, Eip1559TransactionRequest, SignedEip1559TransactionRequest,
            TxHash, TxReceipt, TxStatus,
        },
    },
    flow::{
        self,
        event::TxId,
        state::FlowId,
        trace::{self, TraceEvent},
    },
    logs::{DEBUG, ERROR, INFO},
    metrics::CanisterCall,
    numeric::{BlockNumber, Timestamp, TxLogIndex, Wei, WeiPerGas},
    state::{mutate_state, read_state},
    task::{schedule_after, schedule_now, timestamp_ms},
};

use super::{
    config::Config,
    state::{SendingTx, SignedTx, TxRequest},
    Event,
};

/// A task of the writer state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    /// A task that creates and signed new transactions for the new pending
    /// requests or the pending requests that haven't made progress for a long
    /// time.
    NewTx,

    /// A task that (re-)sends signed transactions.
    SendTx,

    /// A task that polls for transaction receipts of the signed transactions.
    PollTx,

    /// A task that updates the current transaction fee estimate.
    FetchFeeEstimate,
}

impl Task {
    pub async fn run(self, chain: EvmChain) -> Result<(), String> {
        match self {
            Task::NewTx => new_tx_task(chain).await,
            Task::SendTx => send_tx_task(chain).await,
            Task::PollTx => poll_tx_task(chain).await,
            Task::FetchFeeEstimate => fetch_fee_estimate_task(chain).await,
        }
    }

    pub fn wrap(self, chain: EvmChain) -> crate::task::TaskType {
        crate::task::TaskType::Evm {
            chain,
            task: crate::evm::Task::Writer(self),
        }
    }

    pub fn get_all_tasks(chain: EvmChain) -> Vec<crate::task::TaskType> {
        vec![
            Task::NewTx.wrap(chain),
            Task::SendTx.wrap(chain),
            Task::PollTx.wrap(chain),
            Task::FetchFeeEstimate.wrap(chain),
        ]
    }
}

fn should_try_new_tx(now: Timestamp, pending: &TxRequest, config: &Config) -> bool {
    let pending_receipts = pending.sending.iter().any(|x| x.pending_receipts > 0);
    if pending_receipts {
        // This transaction has receipts that are being confirmed. Wait until
        // the pending receipts are resolved (either confirmed or rejected).
        return false;
    }

    let Some(last_tx) = pending.signed.back() else {
        // If there are no signed transaction, then we should definitely try
        // create a new one.
        return true;
    };

    let resubmit_delay = Timestamp::new(config.tx_resubmit_delay.as_millis() as u64);

    let retry_time = last_tx.sign_time.add(
        resubmit_delay,
        "BUG: overflow in should_try_new_tx: retry_time",
    );
    retry_time <= now
}

pub async fn new_tx_task(chain: EvmChain) -> Result<(), String> {
    // The task and panic guards are on the caller side.

    let start = timestamp_ms();

    let chain_id = read_evm_state(chain, |s| s.chain_id);
    let config = read_evm_state(chain, |s| s.writer.config.clone());
    let ecdsa_key_name = read_state(|s| s.icp.config.ecdsa_key_name.clone());
    let ecdsa_public_key = read_state(|s| s.icp.ecdsa_public_key.clone())
        .ok_or("ECDSA public key is not initialized yet")?;

    let batch: Vec<_> = read_evm_state(chain, |s| {
        let now = timestamp_ms();
        s.writer
            .pending
            .values()
            .filter_map(|p| {
                if should_try_new_tx(now, p, &config) {
                    Some((p.id, p.tx_input.clone(), max_fee(&p.sending), p.nonce))
                } else {
                    None
                }
            })
            .take(config.tx_sign_batch)
            .collect()
    });

    let mut awaiting = vec![];

    for (id, input, max_pending_fee, nonce) in batch {
        let Some(fee) = read_evm_state(chain, |s| s.writer.latest_fee()) else {
            return Err("Fee estimate is too old".into());
        };

        let next_fee = fee.higher(&max_pending_fee).bump(config.tx_fee_bump);
        log!(
            DEBUG,
            "[{:?}]: new tx with fee {:?}, next fee {:?}",
            chain,
            fee,
            next_fee
        );
        let cost = next_fee.cost(input.gas_limit, config.tx_fee_margin);

        if cost > input.cost_limit {
            return Err(format!(
                "Cannot increase fee for operation {}: fee={:?}",
                id, next_fee
            ));
        }

        let request = Eip1559TransactionRequest {
            chain_id,
            nonce,
            max_priority_fee_per_gas: next_fee.max_priority_fee_per_gas,
            max_fee_per_gas: next_fee.max_fee_per_gas,
            gas_limit: input.gas_limit,
            destination: input.contract,
            amount: Wei::ZERO,
            data: input.calldata.clone(),
            access_list: AccessList::default(),
        };

        let future = do_sign_tx(
            chain,
            id,
            request,
            ecdsa_key_name.clone(),
            ecdsa_public_key.clone(),
        );

        awaiting.push(future);
    }

    if !awaiting.is_empty() {
        let count = awaiting.len();
        let results = futures::future::join_all(awaiting).await;
        for result in results {
            if let Err(err) = result {
                log!(DEBUG, "[{:?}]: new_tx_task: {}", chain, err,);
            }
        }
        schedule_after(
            config.tx_sign_to_send_delay,
            Task::SendTx.wrap(chain),
            "new tx".into(),
        );
        let end = timestamp_ms();
        log!(
            DEBUG,
            "[{:?}]: new_tx_task: {} tx in {} ms",
            chain,
            count,
            end.into_inner() - start.into_inner()
        );
    }

    // Re-schedule this task either immediately or after a delay.
    let any_pending_tx: bool = read_evm_state(chain, |s| {
        let now = timestamp_ms();
        s.writer
            .pending
            .values()
            .any(|d| should_try_new_tx(now, d, &config))
    });
    if any_pending_tx {
        schedule_now(Task::NewTx.wrap(chain), "deposits without tx".into());
    } else {
        let any_pending_deposits = read_evm_state(chain, |s| !s.writer.pending.is_empty());
        if any_pending_deposits {
            schedule_after(
                config.tx_resubmit_delay,
                Task::NewTx.wrap(chain),
                "pending deposits".into(),
            );
        }
    }
    Ok(())
}

async fn do_sign_tx(
    chain: EvmChain,
    id: FlowId,
    request: Eip1559TransactionRequest,
    ecdsa_key_name: String,
    ecdsa_public_key: PublicKey,
) -> Result<(), String> {
    if !read_evm_state(chain, |s| s.writer.pending.contains_key(&id)) {
        return Ok(());
    }

    let hash = request.hash();

    let raw_signature =
        crate::management::sign_with_ecdsa(ecdsa_key_name, DerivationPath::new(vec![]), hash)
            .await
            .map_err(|e| format!("failed to sign tx: {}", e))?;

    // It is possible that there is no entry corresponding to `id` in
    // `pending` if another transaction has completed and finalized the write
    // or if there was a canister upgrade in the meantime.
    // In that cases, it is safe to ignore the missing entry.
    if !read_evm_state(chain, |s| s.writer.pending.contains_key(&id)) {
        return Ok(());
    }

    let signature = wrap_signature(raw_signature, hash, ecdsa_public_key)?;
    let tx = SignedEip1559TransactionRequest::new(request, signature);
    let tx_hash = tx.hash();
    let tx_id = TxId::Evm(TxLogId {
        tx_hash: tx.hash(),
        index: TxLogIndex::ZERO,
    });
    process_event(Event::SignedTx { id, tx_hash }.wrap(chain));
    trace::ok(id, TraceEvent::SignTx, tx_id, None);
    mutate_evm_state(chain, |s| {
        s.writer.pending.entry(id).and_modify(|pending| {
            pending.sending.push_back(SendingTx {
                tx,
                sign_time: timestamp_ms(),
                send_time: None,
                pending_receipts: 0,
            });
        });
    });

    Ok(())
}

pub async fn send_tx_task(chain: EvmChain) -> Result<(), String> {
    // The task and panic guards are on the caller side.

    let config = read_evm_state(chain, |s| s.writer.config.clone());
    let evm_rpc = read_evm_state(chain, |s| s.evm_rpc.clone());
    let evm_rpc_client = build_evm_rpc_client(&evm_rpc, 1);

    let to_send: Vec<_> = read_evm_state(chain, |s| {
        s.writer
            .pending
            .values()
            .filter_map(|d| {
                let pending_receipts = d.sending.iter().any(|x| x.pending_receipts > 0);
                if pending_receipts {
                    return None;
                }
                let last_tx = d.sending.iter().last();
                if let Some(tx) = last_tx {
                    if tx.send_time.is_none() {
                        return Some((d.id, tx.clone()));
                    }
                }
                None
            })
            .collect()
    });

    for (id, sending) in to_send.into_iter() {
        let done = read_evm_state(chain, |s| match s.writer.pending.get(&id) {
            Some(w) => w.sending.iter().any(|x| x.pending_receipts > 0),
            None => true,
        });
        if done {
            continue;
        }

        let tx_hash = sending.tx.hash();
        let tx_id = TxId::Evm(TxLogId {
            tx_hash,
            index: TxLogIndex::ZERO,
        });

        let cc = CanisterCall::new(
            evm_rpc.evm_rpc_canister_id,
            "eth_send_raw_transaction",
            evm_rpc.evm_rpc_canister_cycles,
        );
        let result = evm_rpc_client
            .eth_send_raw_transaction(sending.tx.raw_transaction_hex())
            .await;

        let reduced_result = pick_any_ok(result);
        match reduced_result {
            Ok(SendRawTransactionStatus::InsufficientFunds)
            | Ok(SendRawTransactionStatus::NonceTooHigh)
            | Ok(SendRawTransactionStatus::NonceTooLow) => {
                let err = format!("{:?}", reduced_result);
                cc.returned_err(&err);
                trace::err(id, TraceEvent::SendTx, tx_id, None, &err);
                // Note that we exit the task here because it doesn't help to
                // continue with later transactions because they will have
                // a higher nonce.
                return Err(format!(
                    "failed to send transaction with hash={}: {:?}",
                    sending.tx.hash(),
                    reduced_result
                ));
            }

            Ok(SendRawTransactionStatus::Ok(_)) => {
                cc.returned_ok();
                mutate_evm_state(chain, |s| {
                    // It is possible that there is no entry corresponding to `id` in
                    // `pending` if another transaction has completed and finalized the write
                    // or if there was a canister upgrade in the meantime.
                    // In such cases, it is safe to ignore the missing entry.
                    s.writer.pending.entry(id).and_modify(|pending| {
                        let maybe_tx = pending.sending.iter_mut().find(|p| p.tx.hash() == tx_hash);
                        // Transactions are never removed from `pending`, so we should
                        // always be able to find the pending transaction by its hash.
                        let tx = maybe_tx.expect("BUG: cannot find pending tx by its hash");
                        tx.send_time = Some(timestamp_ms());
                    });
                });
                trace::ok(id, TraceEvent::SendTx, tx_id, None);
                schedule_after(
                    config.tx_send_to_poll_delay,
                    Task::PollTx.wrap(chain),
                    "send tx".into(),
                );
            }

            Err(RpcError::HttpOutcallError(HttpOutcallError::IcError {
                code: RejectionCode::SysTransient,
                message,
            })) if message.contains("No consensus") => {
                cc.returned_err(&message);
                // At this point we don't know if sending the transaction
                // succeeded or not. Optimistically mark the transaction as
                // sent. If it actually has failed, then it will be retried
                // later on anyways.
                let tx_hash = sending.tx.hash();
                mutate_evm_state(chain, |s| {
                    s.writer.pending.entry(id).and_modify(|pending| {
                        let maybe_tx = pending.sending.iter_mut().find(|p| p.tx.hash() == tx_hash);
                        // Transactions are never removed from `pending`, so we should
                        // always be able to find the pending transaction by its hash.
                        let tx = maybe_tx.expect("BUG: cannot find pending tx by its hash");
                        tx.send_time = Some(timestamp_ms());
                    });
                });
                trace::ok(id, TraceEvent::SendTx, tx_id, None);
                schedule_after(
                    config.tx_send_to_poll_delay,
                    Task::PollTx.wrap(chain),
                    "maybe send tx".into(),
                );
            }

            Err(..) => {
                let err = format!("{:?}", reduced_result);
                cc.returned_err(&err);
                trace::err(id, TraceEvent::SendTx, tx_id, None, &err);
                // Note that we exit the task here because it doesn't help to
                // continue with later transactions because they will have
                // a higher nonce.
                return Err(format!(
                    "failed to send transaction with hash={}: {:?}",
                    sending.tx.hash(),
                    reduced_result
                ));
            }
        }
    }

    // Re-schedule this task.
    let any_tx_to_send: bool = read_evm_state(chain, |s| {
        s.writer.pending.values().any(|d| {
            let last_tx = d.sending.iter().last();
            match last_tx {
                Some(tx) => tx.send_time.is_none(),
                None => false,
            }
        })
    });
    if any_tx_to_send {
        schedule_after(
            config.tx_resend_delay,
            Task::SendTx.wrap(chain),
            "recurring: more tx to send".into(),
        );
    }

    let any_pending: bool = read_evm_state(chain, |s| !s.writer.pending.is_empty());
    if any_pending {
        schedule_after(
            config.tx_sign_to_poll_delay,
            evm::prover::Task::FetchLatestBlock.wrap(chain),
            "send tx".into(),
        );
        schedule_after(
            config.tx_sign_to_poll_delay,
            Task::PollTx.wrap(chain),
            "send tx".into(),
        );
    }

    Ok(())
}

fn should_poll_tx(signed: &SignedTx, now: Timestamp, config: &super::Config) -> bool {
    let poll_delay = Timestamp::new(config.tx_sign_to_poll_delay.as_millis() as u64);
    // A signed transaction could have been sent even if it is not marked as
    // sent, so we poll after some delay.
    let deadline = signed
        .sign_time
        .add(poll_delay, "BUG: overflow in should_poll_tx: deadline");
    deadline <= now
}

pub async fn poll_tx_task(chain: EvmChain) -> Result<(), String> {
    // The task and panic guards are on the caller side.
    let config = read_evm_state(chain, |s| s.writer.config.clone());
    let evm_rpc = read_evm_state(chain, |s| s.evm_rpc.clone());
    let evm_rpc_client = build_evm_rpc_client(&evm_rpc, evm_rpc.consensus_threshold);

    let to_poll: Vec<_> = read_evm_state(chain, |s| {
        let now = timestamp_ms();
        s.writer
            .pending
            .values()
            .flat_map(|d| {
                let filtered = d.signed.iter().filter(|s| should_poll_tx(s, now, &config));
                // Reverse the iterator to ensure that the recently added
                // transactions are polled first.
                filtered.map(|s| (d.id, s.tx_hash)).rev()
            })
            .collect()
    });

    for (id, tx_hash) in to_poll {
        let done = read_evm_state(chain, |s| !s.writer.pending.contains_key(&id));
        if done {
            continue;
        }

        let safe = read_evm_state(chain, |s| {
            s.prover
                .head
                .safe
                .as_ref()
                .map(|x| x.block_number)
                .unwrap_or(BlockNumber::ZERO)
        });

        let cc = CanisterCall::new(
            evm_rpc.evm_rpc_canister_id,
            "eth_get_transaction_receipt",
            evm_rpc.evm_rpc_canister_cycles,
        );
        let result = evm_rpc_client
            .eth_get_transaction_receipt(tx_hash.to_string())
            .await;

        let result = map_multi_rpc(result, |r| match r {
            Ok(r) => Ok(r.map(TxReceipt::try_from)),
            Err(err) => Err(err),
        });

        match consensus(result, evm_rpc.consensus_threshold) {
            ConsensusResult::Consensus(Ok(Some(Ok(tx_receipt)))) => {
                cc.returned_ok();
                if tx_receipt.block_number <= safe {
                    process_tx_receipt(chain, id, tx_receipt);
                } else {
                    let pending_receipts = increment_pending_receipt(chain, id, tx_hash);
                    if pending_receipts == 1 {
                        let tx = TxLogId {
                            tx_hash,
                            index: TxLogIndex::ZERO,
                        };
                        trace::ok(
                            id,
                            TraceEvent::PendingConfirmTx,
                            TxId::Evm(tx),
                            Some(tx_receipt.block_number),
                        );
                        notify_latest_block(chain, tx_receipt.block_number, None, None);
                    }
                }
            }
            ConsensusResult::Consensus(Ok(Some(Err(err)))) => {
                cc.returned_err(&err);
                // We couldn't parse the transaction status because it
                // was invalid. This case should not happen in practice.
                log!(
                    ERROR,
                    "BUG: [{:?}]: failed to parse transaction receipt hash={}: {}",
                    chain,
                    tx_hash.to_string(),
                    err
                );
            }
            ConsensusResult::Consensus(Ok(None)) => {
                let err = "no receipt yet";
                cc.returned_err(err);
            }
            ConsensusResult::Consensus(Err(err)) => {
                let err_msg = err.to_string();
                cc.returned_err(&err_msg);
                log!(
                    INFO,
                    "[{:?}]: RPC error when polling for receipt for hash={}: {}",
                    chain,
                    tx_hash,
                    err
                );
            }
            ConsensusResult::NoConsensus(items) => {
                let err_msg = format!(
                    "failed to reach consensus: consensus={}, threshold={}",
                    items.len(),
                    evm_rpc.consensus_threshold
                );
                cc.returned_err(&err_msg);
                log!(
                    INFO,
                    "[{:?}]: RPC error when polling for receipt for hash={}: {}",
                    chain,
                    tx_hash,
                    err_msg
                );
            }
        }
    }

    // Re-schedule this task.
    let any_pending: bool = read_evm_state(chain, |s| !s.writer.pending.is_empty());
    if any_pending {
        schedule_after(
            config.tx_sign_to_poll_delay,
            evm::prover::Task::FetchLatestBlock.wrap(chain),
            "poll tx".into(),
        );
        schedule_after(
            config.tx_sign_to_poll_delay,
            Task::PollTx.wrap(chain),
            "recurring".into(),
        );
    }

    Ok(())
}

pub fn apply_confirmed_proofs(chain: EvmChain, proofs: Vec<ValidatedProof>) {
    for proof in proofs {
        match proof {
            ValidatedProof::TxReceipt {
                id,
                block_hash: _,
                tx_receipt,
            } => {
                log!(
                    DEBUG,
                    "[{:?}]: applying confirmed proof {}",
                    chain,
                    tx_receipt.tx_hash
                );
                process_tx_receipt(chain, id, tx_receipt);
            }
        }
    }
}

pub fn process_tx_receipt(chain: EvmChain, id: FlowId, tx_receipt: TxReceipt) {
    let Some((token, op, signed)) = read_evm_state(chain, |s| {
        s.writer
            .pending
            .get(&id)
            .map(|r| (r.token, r.op, r.signed.clone()))
    }) else {
        // This can happen if the transaction has already been finalized.
        return;
    };

    let tx_hash = tx_receipt.tx_hash;
    if !signed.iter().any(|s| s.tx_hash == tx_hash) {
        log!(
            ERROR,
            "BUG: [{:?}]: cannot find signed tx for receipt: id={}, receipt: {}, signed: {:?}",
            chain,
            id,
            tx_hash,
            signed
        );
        return;
    }

    let status = tx_receipt.status;
    let tx = TxLogId {
        tx_hash,
        index: TxLogIndex::ZERO,
    };

    let block_number = tx_receipt.block_number;

    let event = Event::Finished { id, tx_receipt };
    process_event(event.wrap(chain));

    match status {
        TxStatus::Success => {
            process_event(ledger::Event::Succeeded { id, tx }.wrap(chain, token));
            process_event(
                flow::Event::SucceededStep {
                    id,
                    chain: chain.into(),
                    op,
                    tx: TxId::Evm(tx),
                }
                .wrap(),
            );
            trace::ok(id, TraceEvent::ConfirmTx, TxId::Evm(tx), Some(block_number));
        }
        TxStatus::Failure => {
            let err = "EVM transaction returned an error while executing";
            process_event(
                ledger::Event::Failed {
                    id,
                    tx,
                    err: err.into(),
                }
                .wrap(chain, token),
            );
            process_event(
                flow::Event::FailedStep {
                    id,
                    chain: chain.into(),
                    op,
                    tx: Some(TxId::Evm(tx)),
                    err: err.into(),
                }
                .wrap(),
            );
            trace::err(
                id,
                TraceEvent::ConfirmTx,
                TxId::Evm(tx),
                Some(block_number),
                err,
            );
        }
    }
}

/// Returns the largest EVM fees within the given list of pending transactions.
fn max_fee(pending: &VecDeque<SendingTx>) -> TxFee {
    let max_fee_per_gas = pending
        .iter()
        .max_by_key(|p| p.tx.transaction().max_fee_per_gas)
        .map(|p| p.tx.transaction().max_fee_per_gas)
        .unwrap_or(WeiPerGas::ZERO);

    let max_priority_fee_per_gas = pending
        .iter()
        .max_by_key(|p| p.tx.transaction().max_priority_fee_per_gas)
        .map(|p| p.tx.transaction().max_priority_fee_per_gas)
        .unwrap_or(WeiPerGas::ZERO);

    TxFee {
        max_fee_per_gas,
        max_priority_fee_per_gas,
    }
}

pub fn increment_pending_receipt(chain: EvmChain, id: FlowId, tx_hash: TxHash) -> usize {
    mutate_evm_state(chain, |s| {
        let mut result = 0;
        s.writer.pending.entry(id).and_modify(|w| {
            for sending in w.sending.iter_mut() {
                if sending.tx.hash() == tx_hash {
                    sending.pending_receipts += 1;
                    result = sending.pending_receipts;
                }
            }
        });
        result
    })
}

pub async fn fetch_fee_estimate_task(chain: EvmChain) -> Result<(), String> {
    let evm_rpc = read_evm_state(chain, |s| s.evm_rpc.clone());
    let evm_rpc_client = build_evm_rpc_client(&evm_rpc, 1);

    let last_known_block = read_evm_state(chain, |s| {
        s.reader
            .last_fully_fetched_block
            .unwrap_or(BlockNumber::ZERO)
            .max(s.prover.forest.last_confirmed_height())
    });

    let newest_block = if last_known_block > BlockNumber::ZERO {
        BlockTag::Number(last_known_block.into_inner().into())
    } else {
        BlockTag::Latest
    };

    let cc = CanisterCall::new(
        evm_rpc.evm_rpc_canister_id,
        "eth_fee_history",
        evm_rpc.evm_rpc_canister_cycles,
    );

    let args = fee_history_args(newest_block);

    let block_count = args.block_count.clone();

    let result = evm_rpc_client.eth_fee_history(args).await;

    let fee_history = pick_any_ok(result).map_err(|err| err.to_string())?;

    let block_number = if last_known_block > BlockNumber::ZERO {
        last_known_block
    } else {
        BlockNumber::new(
            (Nat::from(fee_history.oldest_block.clone()) + Nat::from(block_count))
                .0
                .to_u64()
                .unwrap_or_default(),
        )
    };

    let tx_fee = match get_fee_from_history(fee_history) {
        Ok(fee) => {
            cc.returned_ok();
            fee
        }
        Err(err) => {
            cc.returned_err(&err);
            return Err(err);
        }
    };

    mutate_state(|s| {
        let evm_state = s
            .evm
            .get_mut(&chain)
            .unwrap_or_else(|| unreachable!("BUG: cannot find evm state for {:?}", chain));
        let fee = FeeEstimate {
            fee: tx_fee,
            block_number,
            last_updated: timestamp_ms(),
        };
        evm_state.writer.daily_average_fee.add(&fee);
        evm_state.writer.fetched_fee = Some(fee);
    });

    let delay = read_evm_state(chain, |s| s.writer.config.fetch_fee_estimate_delay);
    schedule_after(
        delay,
        Task::FetchFeeEstimate.wrap(chain),
        "recurring".into(),
    );
    Ok(())
}

pub fn fee_history_args(block_number: BlockTag) -> FeeHistoryArgs {
    FeeHistoryArgs {
        block_count: Nat256::from(5_u8),
        newest_block: block_number,
        reward_percentiles: Some(vec![20]),
    }
}

pub fn get_fee_from_history(fee_history: FeeHistory) -> Result<TxFee, String> {
    let base_fee = WeiPerGas::try_from(
        fee_history
            .base_fee_per_gas
            .last()
            .cloned()
            .ok_or("received empty base_fee_per_gas in fee history")?,
    )
    .map_err(|err| format!("BUG: update_fee_estimate_task: {}", err))?;

    let mut rewards: Vec<_> = fee_history
        .reward
        .iter()
        .flatten()
        .cloned()
        .map(WeiPerGas::try_from)
        .collect();
    if rewards.is_empty() {
        let err = "received empty rewards in fee history".to_string();
        return Err(err);
    }

    rewards.sort();

    let max_priority_fee_per_gas = rewards[rewards.len() / 2]
        .clone()
        .map_err(|err| format!("BUG: update_fee_estimate_task: rewards: {}", err))?;

    let max_fee_per_gas = base_fee
        .checked_add(max_priority_fee_per_gas)
        .ok_or("Overflow in base_fee + max_priority_fee")?;

    Ok(TxFee {
        max_fee_per_gas,
        max_priority_fee_per_gas,
    })
}

/// Returns how many other pending flows are ahead of this flow in the queue for
/// signing the transaction.
pub fn queue_position(chain: EvmChain, id: FlowId) -> Option<u64> {
    read_evm_state(chain, |s| {
        let request = s.writer.pending.get(&id)?;
        if !request.signed.is_empty() {
            return None;
        }
        Some(
            s.writer
                .pending
                .iter()
                .filter(|(_id, req)| req.signed.is_empty())
                .take_while(|(i, _req)| *i < &id)
                .count() as u64,
        )
    })
}
