use crate::{
    api::types::{EvmChain, RelayProof, RelayTask, RLP},
    evm::{
        fee::FeeEstimate,
        prover::head::Head,
        state::{mutate_evm_state, read_evm_state},
        TxFee,
    },
    numeric::{BlockNumber, WeiPerGas},
    task::{schedule_after, schedule_now, timestamp_ms},
};

use super::{
    head::estimated_time_to_safe_block,
    proof::{validate_block_proof, validate_tx_receipt_proof},
    task::{next_useful_latest, prune_stale_blocks},
    Task,
};

/// Returns a list of blocks to fetch as `RelayTask::FetchEvmBlock`
fn get_missing_blocks(chain: EvmChain) -> Vec<RelayTask> {
    let safety_margin = read_evm_state(chain, |s| s.prover.head.config.safety_margin).into_inner();

    let max_blocks = safety_margin;
    let min_blocks = safety_margin / 2;

    fn confirmed(chain: EvmChain, b: &BlockNumber) -> bool {
        read_evm_state(chain, |s| s.prover.forest.confirmed_heights.contains(b))
    }
    fn known(chain: EvmChain, b: &BlockNumber) -> bool {
        read_evm_state(chain, |s| s.prover.forest.blocks_by_height.contains_key(b))
    }

    let blocks: Vec<_> = read_evm_state(chain, |s| {
        s.prover
            .forest
            .blocks_by_height
            .iter()
            .map(|x| *x.0)
            .collect()
    });

    let safe = read_evm_state(chain, |s| {
        s.prover
            .head
            .safe
            .as_ref()
            .map(|x| x.block_number)
            .unwrap_or_default()
            .into_inner()
    });

    let first = blocks.first().cloned().unwrap_or_default().into_inner();

    let mut result = vec![];

    read_evm_state(chain, |s| {
        if let Some(block) = s.reader.unconfirmed_blocks.first() {
            if block.into_inner() + safety_margin > safe {
                result.push(RelayTask::FetchEvmBlock {
                    block_number: block.into_inner() + safety_margin,
                });
            }
        }
        if let Some(block) = s.reader.unconfirmed_blocks.last() {
            if block.into_inner() + safety_margin > safe {
                result.push(RelayTask::FetchEvmBlock {
                    block_number: block.into_inner() + safety_margin,
                });
            }
        }
        if first + safety_margin > safe {
            result.push(RelayTask::FetchEvmBlock {
                block_number: first + safety_margin,
            });
        }
    });

    for block in blocks {
        if !confirmed(chain, &block) {
            let start = block.into_inner() + 1;
            let end = safe.max(first + min_blocks);

            for i in start..end {
                let b = BlockNumber::new(i);
                if confirmed(chain, &b) || known(chain, &b) {
                    break;
                }
                result.push(RelayTask::FetchEvmBlock { block_number: i });
                if result.len() >= max_blocks as usize {
                    return result;
                }
            }
        }
    }
    result
}

/// Return a list of transactions to send as `RelayTask::SendEvmTransaction`.
fn get_pending_transactions(chain: EvmChain) -> Vec<RelayTask> {
    read_evm_state(chain, |s| {
        s.writer
            .pending
            .iter()
            .flat_map(|(id, write)| {
                write.sending.iter().map(|p| RelayTask::SendEvmTransaction {
                    id: id.into_inner(),
                    tx: RLP { bytes: p.tx.rlp() },
                })
            })
            .collect()
    })
}

pub fn get_relay_tasks(chain: EvmChain) -> Vec<RelayTask> {
    let mut result = get_missing_blocks(chain);
    result.extend(get_pending_transactions(chain));
    result
}

pub fn submit_relay_proofs(chain: EvmChain, proofs: Vec<RelayProof>) -> Result<(), String> {
    let (blocks, proofs): (Vec<_>, Vec<_>) = proofs.into_iter().partition(|proof| match proof {
        RelayProof::EvmBlockHeader { .. } | RelayProof::EvmBlockWithTxLogs { .. } => true,
        RelayProof::EvmTransactionReceipt { .. } => false,
    });

    for block in blocks {
        match block {
            RelayProof::EvmBlockHeader {
                block_hash: hash,
                block_header,
                hint_fee_per_gas,
                hint_priority_fee_per_gas,
            } => {
                let block = validate_block_proof(hash, block_header)?;
                notify_latest_block(
                    chain,
                    block.number,
                    hint_fee_per_gas,
                    hint_priority_fee_per_gas,
                );
                mutate_evm_state(chain, |s| s.prover.forest.add_validated_block(block));
            }
            RelayProof::EvmBlockWithTxLogs { block_number } => mutate_evm_state(chain, |s| {
                s.reader
                    .add_unconfirmed_block(BlockNumber::new(block_number));
            }),
            RelayProof::EvmTransactionReceipt { .. } => {
                // Nothing to do.
            }
        }
    }

    let mut any_tx_proof = false;

    for proof in proofs {
        match proof {
            RelayProof::EvmTransactionReceipt {
                id,
                block_hash,
                tx,
                receipt,
            } => {
                validate_tx_receipt_proof(chain, id, block_hash, tx, receipt)?;
                any_tx_proof = true;
            }
            RelayProof::EvmBlockHeader { .. } | RelayProof::EvmBlockWithTxLogs { .. } => {
                // Nothing to do.
            }
        }
    }

    prune_stale_blocks(chain);

    if any_tx_proof {
        schedule_after(
            estimated_time_to_safe_block(chain),
            Task::FetchLatestBlock.wrap(chain),
            "submit relay proofs".into(),
        );
    }
    Ok(())
}

pub fn notify_latest_block(
    chain: EvmChain,
    hint_block_number: BlockNumber,
    hint_max_fee_per_gas: Option<u64>,
    hint_max_priority_fee_per_gas: Option<u64>,
) {
    mutate_evm_state(chain, |s| {
        let hint_time = timestamp_ms();

        let hint = Head {
            block_number: hint_block_number,
            block_hash: Default::default(),
            fetch_time: hint_time,
        };

        match s.prover.head.hint.as_ref() {
            Some(old) => {
                if hint_block_number > old.block_number {
                    s.prover.head.hint = Some(hint)
                }
            }
            None => {
                s.prover.head.hint = Some(hint);
            }
        }
        if let (Some(a), Some(b)) = (hint_max_fee_per_gas, hint_max_priority_fee_per_gas) {
            s.writer.relay_fee(FeeEstimate {
                fee: TxFee {
                    max_fee_per_gas: WeiPerGas::new(a as u128),
                    max_priority_fee_per_gas: WeiPerGas::new(b as u128),
                },
                block_number: hint_block_number,
                last_updated: hint_time,
            })
        }
    });

    let latest = read_evm_state(chain, |s| {
        s.prover.head.latest.as_ref().map(|x| x.block_number)
    });
    let next = next_useful_latest(chain);
    let hint = read_evm_state(chain, |s| {
        s.prover.head.hint.as_ref().map(|x| x.block_number)
    });

    if let (Some(latest), Some(next), Some(hint)) = (latest, next, hint) {
        if latest < next && next <= hint {
            schedule_now(
                Task::FetchLatestBlock.wrap(chain),
                "relayer hint".to_string(),
            );
        }
    }
}
