use alloy::primitives::{FixedBytes, B256};
use alloy_trie::{
    proof::{verify_proof, ProofNodes, ProofRetainer},
    root::adjust_index_for_rlp,
    HashBuilder, Nibbles, EMPTY_ROOT_HASH,
};
use eyre::eyre;
use one_sec::{
    api::types::{RelayProof, TrieProof, RLP},
    evm::prover::{decode_receipt, decode_tx, validate_block_proof, validate_trie_proof},
};

pub fn build_proof<T, E>(
    index: usize,
    expected_root: FixedBytes<32>,
    leaves: Vec<T>,
    encode: E,
) -> Result<TrieProof, eyre::Error>
where
    E: Fn(&T, &mut Vec<u8>),
{
    if index >= leaves.len() {
        return Err(eyre!(
            "index out of bounds: index is {} but leaves len is {}",
            index,
            leaves.len()
        ));
    }

    let key = nibbles_from_index(index);

    let mut value = vec![];
    encode(&leaves[index], &mut value);

    let (root, nodes) = ordered_trie_root_with_encoder(&leaves, index, encode);

    if root != expected_root {
        return Err(eyre!(
            "Root mismatch: {} vs {} for {}",
            expected_root,
            root,
            index
        ));
    }

    let nodes: Vec<_> = nodes
        .matching_nodes_sorted(&key)
        .into_iter()
        .map(|(_key, bytes)| bytes)
        .collect();

    verify_proof(expected_root, key.clone(), Some(value.clone()), &nodes)?;

    let proof = TrieProof {
        root_hash: root.0,
        index: index as u64,
        value: RLP { bytes: value },
        nodes: nodes
            .into_iter()
            .map(|n| RLP { bytes: n.to_vec() })
            .collect(),
    };

    Ok(proof)
}

/// Compute a trie root of the collection of items with a custom encoder.
fn ordered_trie_root_with_encoder<T, E>(items: &[T], proof: usize, encode: E) -> (B256, ProofNodes)
where
    E: Fn(&T, &mut Vec<u8>),
{
    if items.is_empty() {
        return (EMPTY_ROOT_HASH, ProofNodes::default());
    }

    let mut value_buffer = Vec::new();

    let targets = vec![nibbles_from_index(proof)];

    let retainer = ProofRetainer::new(targets);

    let mut hb = HashBuilder::default().with_proof_retainer(retainer);
    let items_len = items.len();
    for i in 0..items_len {
        let index = adjust_index_for_rlp(i, items_len);
        let key = nibbles_from_index(index);
        value_buffer.clear();
        encode(&items[index], &mut value_buffer);
        hb.add_leaf(key, &value_buffer);
    }

    (hb.root(), hb.take_proof_nodes())
}

fn nibbles_from_index(index: usize) -> Nibbles {
    Nibbles::unpack(alloy_rlp::encode_fixed_size(&index))
}

pub fn check_proof(proof: RelayProof, decode: bool) -> Result<(), String> {
    match proof {
        RelayProof::EvmTransactionReceipt {
            id: _,
            block_hash: _,
            tx,
            receipt,
        } => {
            if decode {
                decode_tx(&tx.value.bytes[..])?;
                decode_receipt(&receipt.value.bytes[..])?;
            }
            validate_trie_proof(tx)?;
            validate_trie_proof(receipt)?;
        }
        RelayProof::EvmBlockHeader {
            block_hash,
            block_header,
            hint_fee_per_gas: _,
            hint_priority_fee_per_gas: _,
        } => {
            validate_block_proof(block_hash, block_header)?;
        }
        RelayProof::EvmBlockWithTxLogs { .. } => {
            // Nothing to do.
        }
    }
    Ok(())
}
