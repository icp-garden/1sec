//! This module defines the configuration parameters of the EVM state machine.
use crate::api::types::EvmChain;

use super::{evm_rpc, forwarder, ledger, prover, reader, writer};

/// The configuration parameters of the EVM state machine.
#[derive(Debug, Clone)]
pub struct Config {
    pub chain: EvmChain,
    pub chain_id: u64,
    pub reader: reader::Config,
    pub writer: writer::Config,
    pub prover: prover::Config,
    pub forwarder: forwarder::Config,
    pub ledger: Vec<ledger::Config>,
    pub evm_rpc: evm_rpc::Config,
}
