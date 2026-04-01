use alloy::providers::ProviderBuilder;
use alloy::providers::{Provider, RootProvider};
use alloy::rpc::types::TransactionReceipt;
use alloy_consensus::ReceiptEnvelope;
use alloy_eips::BlockId;
use alloy_eips::BlockNumberOrTag;
use alloy_eips::Encodable2718;
use alloy_eips::RpcBlockHash;
use alloy_rlp::BytesMut;
use alloy_rlp::Encodable;
use async_trait::async_trait;
use candid::Nat;
use evm_rpc_types::Nat256;
use eyre::eyre;
use eyre::OptionExt;
use ic_agent::export::reqwest::Url;
use one_sec::api::types::RelayProof;
use one_sec::evm::writer::{fee_history_args, get_fee_from_history};
use one_sec::evm::TxHash;
use num_traits::cast::ToPrimitive;

use crate::merkle::{build_proof, check_proof};
use crate::TaskMetadata;
use crate::TxIndex;
use crate::Worker;

pub struct EthWorker {
    provider: RootProvider<alloy::network::Ethereum>,
}

impl EthWorker {
    pub fn new(rpc_url: Url) -> Self {
        let provider = ProviderBuilder::new()
            .disable_recommended_fillers()
            .network::<alloy::network::Ethereum>()
            .on_http(rpc_url);
        Self { provider }
    }
}

#[async_trait]
impl Worker for EthWorker {
    async fn send_tx(&self, raw_tx: &[u8]) -> Result<(), eyre::Error> {
        let _ = self.provider.send_raw_transaction(raw_tx).await?;
        Ok(())
    }

    async fn check_status(
        &self,
        tx_hash: TxHash,
    ) -> Result<Option<(BlockNumberOrTag, TxIndex)>, eyre::Error> {
        let receipt = self
            .provider
            .get_transaction_receipt(tx_hash.0.into())
            .await?;
        match receipt {
            None => Ok(None),
            Some(receipt) => {
                let block_number = receipt.block_number.unwrap();
                let tx_index = receipt.transaction_index.unwrap();
                Ok(Some((
                    BlockNumberOrTag::Number(block_number),
                    tx_index as usize,
                )))
            }
        }
    }

    async fn build_tx_proof(
        &self,
        block_number: BlockNumberOrTag,
        tx_index: usize,
        task_metadata: &TaskMetadata,
    ) -> Result<Vec<RelayProof>, eyre::Error> {
        let block = self
            .provider
            .get_block_by_number(block_number)
            .full()
            .await?
            .ok_or_eyre(eyre!("Block not found: {} / {}", block_number, tx_index))?;

        let block_hash = block.header.hash;

        let txs: Vec<_> = block
            .transactions
            .as_transactions()
            .unwrap()
            .iter()
            .cloned()
            .map(|tx| tx.inner.clone_inner())
            .collect();

        let mut receipts = vec![];

        for _ in 0..10 {
            if let Some(r) = self
                .provider
                .get_block_receipts(BlockId::Hash(RpcBlockHash::from_hash(block_hash, None)))
                .await?
            {
                receipts = r;
                break;
            }
        }

        if receipts.is_empty() {
            return Err(eyre!("No block receipts for {}", block_hash));
        }

        let receipts = encodable_receipts(receipts);

        let tx_proof = build_proof(tx_index, block.header.transactions_root, txs, |x, buf| {
            x.encode_2718(buf)
        })?;

        let receipt_proof =
            build_proof(tx_index, block.header.receipts_root, receipts, |x, buf| {
                x.encode_2718(buf)
            })?;

        let mut buffer = BytesMut::new();
        block.header.inner.encode(&mut buffer);
        let block_header = buffer.to_vec();

        let block_proof = RelayProof::EvmBlockHeader {
            block_hash: block_hash.0,
            block_header,
            hint_fee_per_gas: None,
            hint_priority_fee_per_gas: None,
        };

        let tx_receipt_proof = RelayProof::EvmTransactionReceipt {
            id: task_metadata.id,
            block_hash: block_hash.0,
            tx: tx_proof,
            receipt: receipt_proof,
        };

        Ok(vec![block_proof, tx_receipt_proof])
    }

    async fn build_block_proof(
        &self,
        block_number: BlockNumberOrTag,
    ) -> Result<(u64, RelayProof), eyre::Error> {
        let block = self
            .provider
            .get_block_by_number(block_number)
            .full()
            .await?
            .ok_or_eyre(eyre!("Block not found: {}", block_number))?;

        let hint = match block_number {
            BlockNumberOrTag::Latest => {
                let block_number = block.header.number;
                let args =
                    fee_history_args(evm_rpc_types::BlockTag::Number(Nat256::from(block_number)));

                let block_count = Nat::from(args.block_count).0.to_u64().unwrap_or_default();
                let reward_percentiles: Vec<_> = args
                    .reward_percentiles
                    .unwrap_or_default()
                    .into_iter()
                    .map(|x| x as f64)
                    .collect();

                let fee_history = self
                    .provider
                    .get_fee_history(
                        block_count,
                        BlockNumberOrTag::Number(block_number),
                        &reward_percentiles,
                    )
                    .await?;

                let fee_history = evm_rpc_types::FeeHistory {
                    oldest_block: Nat256::from(fee_history.oldest_block),
                    base_fee_per_gas: fee_history
                        .base_fee_per_gas
                        .into_iter()
                        .map(Nat256::from)
                        .collect(),
                    gas_used_ratio: fee_history.gas_used_ratio,
                    reward: fee_history
                        .reward
                        .unwrap_or_default()
                        .into_iter()
                        .map(|v| v.into_iter().map(Nat256::from).collect())
                        .collect(),
                };
                let fee = get_fee_from_history(fee_history).map_err(|err| eyre!(err))?;
                Some((
                    Nat::from(fee.max_fee_per_gas)
                        .0
                        .to_u64()
                        .unwrap_or_default(),
                    Nat::from(fee.max_priority_fee_per_gas)
                        .0
                        .to_u64()
                        .unwrap_or_default(),
                ))
            }
            _ => None,
        };

        let block_number = block.header.number;
        let block_hash = block.header.hash;

        let mut buffer = BytesMut::new();
        block.header.inner.encode(&mut buffer);
        let block_header = buffer.to_vec();

        let block_proof = RelayProof::EvmBlockHeader {
            block_hash: block_hash.0,
            block_header,
            hint_fee_per_gas: hint.map(|x| x.0),
            hint_priority_fee_per_gas: hint.map(|x| x.1),
        };
        Ok((block_number, block_proof))
    }

    async fn self_check(&self) -> Result<(), eyre::Error> {
        let proof = self
            .build_tx_proof(
                BlockNumberOrTag::Number(22540689),
                10,
                &TaskMetadata { id: 0 },
            )
            .await?;
        for p in proof {
            check_proof(p, true).map_err(|err| eyre!(err))?;
        }
        Ok(())
    }
}

fn encodable_receipts(receipts: Vec<TransactionReceipt>) -> Vec<ReceiptEnvelope> {
    receipts
        .iter()
        .cloned()
        .map(|r| r.inner.into_primitives_receipt())
        .collect()
}
