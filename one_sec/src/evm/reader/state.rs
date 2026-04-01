use evm_rpc_types::{Hex20, Hex32};
use ic_ethereum_types::Address;
use std::collections::{BTreeMap, BTreeSet};

use crate::{api::types::Token, numeric::BlockNumber};

use super::{event::TxLogId, Config};

/// The state of the reader state machine.
#[derive(Debug)]
pub struct State {
    /// The state machine subscribes to event logs from these contracts and
    /// topics (immutable after initialization).
    pub subscription_to_token: BTreeMap<Address, Token>,
    pub subscription_topics: BTreeSet<[u8; 32]>,

    /// The height of the most recent block for which all event logs have been
    /// fetched.
    pub last_fully_fetched_block: Option<BlockNumber>,

    /// Block numbers that were reported by relayers as containing event logs.
    pub unconfirmed_blocks: BTreeSet<BlockNumber>,

    /// The set of all event logs that have been fetched.
    /// It is used to avoid processing the same event log twice.
    pub done: BTreeSet<TxLogId>,

    /// The configuration parameters (immutable).
    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            last_fully_fetched_block: None,
            unconfirmed_blocks: Default::default(),
            done: Default::default(),
            subscription_to_token: Default::default(),
            subscription_topics: Default::default(),
            config,
        }
    }

    pub fn subscribe(&mut self, token: Token, contract: Address, topic: [u8; 32]) {
        let previous = self.subscription_to_token.insert(contract, token);
        if let Some(p) = previous {
            assert_eq!(p, token);
        }
        self.subscription_topics.insert(topic);
    }

    pub fn token(&self, contract: &Address) -> Option<Token> {
        self.subscription_to_token.get(contract).cloned()
    }

    pub fn contracts_and_topics(&self) -> (Vec<Hex20>, Vec<Vec<Hex32>>) {
        let contracts: Vec<_> = self
            .subscription_to_token
            .keys()
            .map(|k| Hex20::from(k.into_bytes()))
            .collect();
        let topics: Vec<_> = self
            .subscription_topics
            .iter()
            .cloned()
            .map(Hex32::from)
            .collect();

        (contracts, vec![topics])
    }

    pub fn add_unconfirmed_block(&mut self, block_number: BlockNumber) {
        if block_number > self.last_fully_fetched_block.unwrap_or_default() {
            self.unconfirmed_blocks.insert(block_number);
        }
    }

    pub fn first_unconfirmed_height_after(&self, start: BlockNumber) -> Option<BlockNumber> {
        let start = start.add(
            BlockNumber::ONE,
            "BUG: reader: first_unconfirmed_height_after: overflow in start + 1",
        );
        self.unconfirmed_blocks.range(start..).next().cloned()
    }
}

#[derive(Debug, Clone)]
pub struct Subscription {
    pub token: Token,
    pub contract: Address,
    pub topic: [u8; 32],
}
