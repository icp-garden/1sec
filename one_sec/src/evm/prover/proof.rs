use alloy_consensus::EthereumTxEnvelope;
use alloy_consensus::Receipt;
use alloy_consensus::TxEip4844Variant;
use alloy_eips::Decodable2718;
use alloy_primitives::FixedBytes;
use evm_rpc_types::Block;
use ic_canister_log::log;
use nybbles::Nibbles;

use crate::evm::reader::TxLogId;
use crate::flow::event::TxId;
use crate::flow::trace;
use crate::flow::trace::TraceEvent;
use crate::logs::DEBUG;
use crate::numeric::TxLogIndex;
use crate::{
    api::types::{EvmChain, TrieProof},
    evm::{
        state::{mutate_evm_state, read_evm_state},
        tx::{TxHash, TxReceipt, TxStatus},
        writer::increment_pending_receipt,
    },
    flow::state::FlowId,
    numeric::{nat256_to_u64, BlockNumber},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedProof {
    TxReceipt {
        id: FlowId,
        block_hash: TxHash,
        tx_receipt: TxReceipt,
    },
}

impl ValidatedProof {
    pub fn block_hash(&self) -> &TxHash {
        match self {
            ValidatedProof::TxReceipt { block_hash, .. } => block_hash,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confirmation {
    #[default]
    Unconfirmed,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedBlock {
    pub hash: TxHash,
    pub parent: TxHash,
    pub number: BlockNumber,
    pub transactions_root: TxHash,
    pub receipts_root: TxHash,
    pub confirmation: Confirmation,
    pub unconfirmed_proofs: Vec<ValidatedProof>,
}

impl TryFrom<Block> for ValidatedBlock {
    type Error = String;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        let number = nat256_to_u64(block.number.clone()).ok_or(format!(
            "BUG: block height doesn't fit into u64: {:?}",
            block.number.clone()
        ))?;
        let transactions_root = block.transactions_root.ok_or(format!(
            "BUG: empty transactions root for block: {}",
            block.hash
        ))?;

        Ok(Self {
            hash: TxHash(block.hash.into()),
            parent: TxHash(block.parent_hash.into()),
            number: BlockNumber::new(number),
            transactions_root: TxHash(transactions_root.into()),
            receipts_root: TxHash(block.receipts_root.into()),
            confirmation: Confirmation::Unconfirmed,
            unconfirmed_proofs: vec![],
        })
    }
}

pub fn validate_block_proof(
    block_hash: [u8; 32],
    block_header: Vec<u8>,
) -> Result<ValidatedBlock, String> {
    use alloy_rlp::Decodable;
    let mut buffer = &block_header[..];
    let header = alloy_consensus::Header::decode(&mut buffer).map_err(|err| err.to_string())?;
    let expected_hash = TxHash(block_hash);
    let actual_hash = TxHash(header.hash_slow().0);
    if expected_hash != actual_hash {
        return Err(format!(
            "mismatch in block hash: expected: {}, actual: {}",
            expected_hash, actual_hash
        ));
    }

    Ok(ValidatedBlock {
        hash: actual_hash,
        parent: TxHash(header.parent_hash.0),
        number: BlockNumber::new(header.number),
        transactions_root: TxHash(header.transactions_root.0),
        receipts_root: TxHash(header.receipts_root.0),
        confirmation: Confirmation::Unconfirmed,
        unconfirmed_proofs: vec![],
    })
}

pub fn validate_trie_proof(proof: TrieProof) -> Result<(), String> {
    let key = Nibbles::unpack(alloy_rlp::encode_fixed_size(&proof.index));

    let nodes: Vec<_> = proof
        .nodes
        .into_iter()
        .map(|node| alloy_primitives::Bytes::from(node.bytes))
        .collect();

    alloy_trie::proof::verify_proof(
        FixedBytes::new(proof.root_hash),
        key,
        Some(proof.value.bytes),
        nodes.iter(),
    )
    .map_err(|err| err.to_string())?;

    Ok(())
}

pub fn decode_tx(mut buf: &[u8]) -> Result<EthereumTxEnvelope<TxEip4844Variant>, String> {
    alloy_consensus::TxEnvelope::decode_2718(&mut buf).map_err(|err| err.to_string())
}

pub fn decode_receipt(mut buf: &[u8]) -> Result<Receipt, String> {
    let envelope =
        alloy_consensus::ReceiptEnvelope::decode_2718(&mut buf).map_err(|err| err.to_string())?;
    envelope
        .as_receipt()
        .cloned()
        .ok_or("cannot get receipt from envelope".into())
}

pub fn validate_tx_receipt_proof(
    chain: EvmChain,
    id: u64,
    block_hash: [u8; 32],
    tx_proof: TrieProof,
    receipt_proof: TrieProof,
) -> Result<(), String> {
    if tx_proof.index != receipt_proof.index {
        return Err("mismatching index in tx and receipt proofs".to_string());
    }

    let block_hash = TxHash(block_hash);

    let (transactions_root, receipts_root, block_number) = read_evm_state(chain, |s| {
        let block = s.prover.forest.blocks.get(&block_hash);
        block.map(|b| (b.transactions_root, b.receipts_root, b.number))
    })
    .ok_or(format!("proof refers to an unknown block: {}", block_hash))?;

    if transactions_root != TxHash(tx_proof.root_hash) {
        return Err(format!(
            "mismatching transactions root hashes: {} vs {}",
            transactions_root,
            TxHash(tx_proof.root_hash)
        ));
    }

    if receipts_root != TxHash(receipt_proof.root_hash) {
        return Err(format!(
            "mismatching receipts root hashes: {} vs {}",
            transactions_root,
            TxHash(receipt_proof.root_hash)
        ));
    }

    let tx = decode_tx(&tx_proof.value.bytes[..])?;
    let receipt = decode_receipt(&receipt_proof.value.bytes[..])?;

    validate_trie_proof(tx_proof)?;
    validate_trie_proof(receipt_proof)?;

    let tx_hash = TxHash((*tx.tx_hash()).into());

    let tx_receipt = TxReceipt {
        tx_hash,
        status: match receipt.status {
            alloy_consensus::Eip658Value::Eip658(true) => TxStatus::Success,
            alloy_consensus::Eip658Value::Eip658(false) => TxStatus::Failure,
            alloy_consensus::Eip658Value::PostState(_) => {
                return Err("unsupported status type".to_string());
            }
        },
        block_number,
    };

    let id = FlowId::new(id);

    if !read_evm_state(chain, |s| {
        s.writer
            .pending
            .get(&id)
            .map(|p| p.signed.iter().any(|x| x.tx_hash == tx_receipt.tx_hash))
            .unwrap_or(false)
    }) {
        return Err(format!(
            "Unknown tx hash {} for id {:?}",
            tx_receipt.tx_hash, id
        ));
    }

    log!(
        DEBUG,
        "[{:?}]: received proof: {} at {}",
        chain,
        tx_hash,
        block_number
    );

    let validated_proof = ValidatedProof::TxReceipt {
        id,
        block_hash,
        tx_receipt,
    };

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
            Some(block_number),
        );
    }

    mutate_evm_state(chain, |s| {
        s.prover
            .forest
            .add_validated_proofs(block_hash, vec![validated_proof])
    });

    Ok(())
}
