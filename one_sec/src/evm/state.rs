//! This module defines the state of the EVM state machine.
use std::collections::BTreeMap;

use crate::{
    api::types::{EvmChain, Token},
    evm::forwarder,
    state::{mutate_state, read_state},
};

use super::{config::Config, evm_rpc, ledger, prover, reader, writer};

/// The state of the EVM state machine.
#[derive(Debug)]
pub struct State {
    /// The EVM chain (immutable).
    pub chain: EvmChain,
    /// The EVM chain id (immutable).
    pub chain_id: u64,
    /// The state of the reader state machine.
    pub reader: reader::State,
    /// The state of the writer state machine.
    pub writer: writer::State,
    /// The state of the prover state machine.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub prover: prover::State,
    /// The state of the forwarder state machine.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub forwarder: forwarder::State,
    /// The state of a ledger state machine.
    pub ledger: BTreeMap<Token, ledger::State>,
    /// The configuration related to the EVM RPC canister (immutable).
    pub evm_rpc: evm_rpc::Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut state = Self {
            chain: config.chain,
            chain_id: config.chain_id,
            reader: reader::State::new(config.reader),
            writer: writer::State::new(config.writer),
            prover: prover::State::new(config.prover),
            forwarder: forwarder::State::new(config.forwarder),
            ledger: config
                .ledger
                .into_iter()
                .map(|c| (c.token, ledger::State::new(c)))
                .collect(),
            evm_rpc: config.evm_rpc,
        };

        let subscriptions: Vec<_> = state
            .ledger
            .iter()
            .flat_map(|(token, s)| {
                s.config
                    .logger_topics
                    .iter()
                    .map(|topic| (*token, s.config.logger_address, *topic))
            })
            .collect();

        for (token, contract, topic) in subscriptions {
            state.reader.subscribe(token, contract, topic);
        }

        state
    }
}

/// Read (part of) the current EVM state using `f`.
///
/// Panics if there is no state.
pub fn read_evm_state<F, R>(chain: EvmChain, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    read_state(|s| {
        let evm_state = s
            .evm
            .get(&chain)
            .unwrap_or_else(|| unreachable!("BUG: cannot find evm state: {:?}", chain));
        f(evm_state)
    })
}

/// Updates the state using the given function.
///
/// Panics if there is no state.
pub fn mutate_evm_state<F, R>(chain: EvmChain, f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    mutate_state(|s| {
        let evm_state = s
            .evm
            .get_mut(&chain)
            .unwrap_or_else(|| unreachable!("BUG: cannot find evm state: {:?}", chain));
        f(evm_state)
    })
}
