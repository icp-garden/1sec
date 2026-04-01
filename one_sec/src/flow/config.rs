//! This module defines the configuration parameters of the flow state machine.
use crate::{
    api::types::{EvmChain, Token},
    numeric::{Amount, Percent},
};

use super::event::Direction;

/// Configuration parameters of the flow state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The maximum number of pending flows at any time.
    pub max_concurrent_flows: usize,

    /// Configuration parameters for specific source and destination tokens.
    pub flows: Vec<FlowConfig>,
}

/// Configuration parameters of a bridging transfer.
#[derive(Debug, Clone)]
pub struct FlowConfig {
    /// The direction of the transfer.
    pub direction: Direction,
    /// The token on the ICP side.
    pub icp_token: Token,
    /// The EVM chain.
    pub evm_chain: EvmChain,
    /// The token on the EVM side.
    pub evm_token: Token,
    /// The minimum amount of the source token.
    pub min_amount: Amount,
    /// The maximum amount of the source token.
    pub max_amount: Amount,
    /// The fee charged by the canister.
    pub fee: Percent,
}
