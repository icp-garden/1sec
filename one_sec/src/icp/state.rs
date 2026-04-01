//! This module defines the state of the ICP state machine.
use ic_secp256k1::PublicKey;
use std::collections::BTreeMap;

use crate::{
    api::types::Token,
    numeric::{Amount, Wei},
};

use super::{ledger, Config};

/// The exchange rate of ETH to a token.
#[derive(Debug, Clone)]
pub struct ExchangeRate {
    /// The amount of ETH (in Wei) per token.
    pub eth_per_token: Wei,
}

impl ExchangeRate {
    pub fn eth_to_token(&self, eth: Wei) -> Option<Amount> {
        let result = eth.checked_div_floor(self.eth_per_token.into_inner())?;
        Some(result.change_units())
    }
}

/// The state of the ICP state machine.
#[derive(Debug)]
pub struct State {
    /// The state of a ledger.
    pub ledger: BTreeMap<Token, ledger::State>,
    /// The ECDSA public key (immutable after it is set).
    /// Note: this state is ephemeral (cleared on upgrade).
    pub ecdsa_public_key: Option<PublicKey>,
    /// TODO
    pub chain_code: Option<Vec<u8>>,
    /// The exchange rate of ETH to a token.
    /// Note: this state is ephemeral (cleared on upgrade).
    pub exchange_rate: BTreeMap<Token, ExchangeRate>,
    /// The configuration parameters of the state machine (immutable).
    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            ledger: config
                .ledger
                .iter()
                .map(|c| (c.token, crate::icp::ledger::State::new(c.clone())))
                .collect(),
            ecdsa_public_key: None,
            chain_code: None,
            exchange_rate: Default::default(),
            config,
        }
    }
}
