use std::collections::{BTreeMap, BTreeSet};

use crate::{evm::tx::TxHash, numeric::BlockNumber};

use super::proof::{Confirmation, ValidatedBlock, ValidatedProof};

#[derive(Debug, Default)]
pub struct State {
    pub blocks: BTreeMap<TxHash, ValidatedBlock>,
    pub children: BTreeMap<TxHash, Vec<TxHash>>,
    pub blocks_by_height: BTreeMap<BlockNumber, Vec<TxHash>>,
    pub confirmed_proofs: Vec<ValidatedProof>,
    pub confirmed_heights: BTreeSet<BlockNumber>,
}

impl State {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_validated_block(&mut self, mut block: ValidatedBlock) {
        let hash = block.hash;
        let parent = block.parent;
        let height = block.number;
        if self.blocks.contains_key(&hash) {
            return;
        }

        // We are going to set the block's confirmation using the
        // `propagate_confirmation` function to also update confirmations of its
        // ancestors, so we need to clear the block's confirmations here.
        // Otherwise, the function is going to bail out without doing anything.
        let initial_confirmation = std::mem::take(&mut block.confirmation);

        self.blocks.insert(hash, block);

        // Pick the most confirmed candidate among the children of this block.
        let children = self.children.get(&hash).cloned().unwrap_or_default();
        let confirmation = initial_confirmation.max(
            children
                .iter()
                .flat_map(|hash| self.blocks.get(hash).map(|block| &block.confirmation))
                .cloned()
                .max()
                .unwrap_or_default(),
        );

        if confirmation == Confirmation::Confirmed {
            self.propagate_confirmation(hash);
        }

        self.children.entry(parent).or_default().push(hash);
        self.blocks_by_height.entry(height).or_default().push(hash);
    }

    pub fn add_validated_proofs(&mut self, hash: TxHash, proofs: Vec<ValidatedProof>) {
        let block = self.blocks.get_mut(&hash).unwrap();
        for proof in proofs.iter() {
            assert_eq!(&hash, proof.block_hash());
        }
        if block.confirmation == Confirmation::Confirmed {
            self.confirmed_proofs.extend(proofs);
        } else {
            block.unconfirmed_proofs.extend(proofs);
        }
    }

    pub fn add_confirmation(&mut self, hash: TxHash) {
        let block = self.blocks.get(&hash).unwrap();
        self.confirmed_heights.insert(block.number);
        if block.confirmation == Confirmation::Unconfirmed {
            self.propagate_confirmation(hash);
        }
    }

    pub fn first_unconfirmed_height_after(&self, start: BlockNumber) -> Option<BlockNumber> {
        let start = start.max(self.last_confirmed_height()).add(
            BlockNumber::ONE,
            "BUG: first_unconfirmed_height_after: overflow in start + 1",
        );

        self.blocks_by_height.range(start..).next().map(|x| *x.0)
    }

    pub fn last_confirmed_height(&self) -> BlockNumber {
        self.confirmed_heights
            .last()
            .cloned()
            .unwrap_or(BlockNumber::ZERO)
    }

    fn propagate_confirmation(&mut self, mut hash: TxHash) {
        while let Some(block) = self.blocks.get_mut(&hash) {
            if block.confirmation == Confirmation::Confirmed {
                break;
            }

            let proofs = std::mem::take(&mut block.unconfirmed_proofs);
            if !proofs.is_empty() {
                self.confirmed_proofs.extend(proofs.into_iter());
            }

            block.confirmation = Confirmation::Confirmed;
            hash = block.parent;
        }
    }
}
