//! This module defines the configuration parameters of the ICP state machine.
use std::collections::BTreeSet;

use candid::Principal;

use super::ledger;

/// Configuration parameters of the ICP state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The name of the ECDSA key used in the management canister calls.
    pub ecdsa_key_name: String,
    /// The exchange rate canister.
    pub xrc_canister_id: Principal,
    /// The configuration parameters of the ledger.
    pub ledger: Vec<ledger::Config>,
    /// If the cycles balance of the canister drops below this threshold, then
    /// it will skip execution of tasks and endpoints for safety.
    pub min_cycles_balance: u128,
    /// Principals that are allowed to use the relayer endpoints.
    pub relayers: Vec<Principal>,
    /// Principals that have a fee rebate to bridge tokens.
    pub market_makers: BTreeSet<Principal>,
}
