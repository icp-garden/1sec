//! This module defines the state of the ICP state machine.
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    api::types::Token,
    flow::{event::Operation, state::FlowId},
    icp::IcpAccount,
    numeric::{Amount, BlockIndex, Timestamp},
    state::read_state,
};

use super::config::Config;

/// The state of the ICP ledger state machine.
#[derive(Debug)]
pub struct State {
    /// The token balance.
    /// Its meaning depends on the operating mode:
    /// - locker: token balance of the canister in the ICRC2 ledger.
    /// - minter: the circulating supply of the token.
    pub balance: Amount,

    /// The total amount of fees that have been collected since the last
    /// transfer of the fees.
    pub fees: Amount,

    /// The pending requests.
    /// Invariant: only unlock/mint requests are observable in tasks.
    /// The lock/burn requests are started and immediately finished.
    pub pending: BTreeSet<FlowId>,

    /// Flows that have pending transfer calls.
    pub quarantine: Vec<FlowId>,

    /// All received lock/burn requests.
    pub lock_or_burn: BTreeMap<FlowId, Request>,

    /// All received unlock/mint requests.
    pub unlock_or_mint: BTreeMap<FlowId, Request>,

    /// The configuration parameters (immutable).
    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            balance: config.initial_balance,
            fees: Amount::ZERO,
            pending: Default::default(),
            quarantine: vec![],
            lock_or_burn: Default::default(),
            unlock_or_mint: Default::default(),
            config,
        }
    }
}

/// A request to execute a token operation: burn/mint/lock/unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: FlowId,
    pub op: Operation,
    pub account: IcpAccount,
    pub amount: Amount,
    pub collected_fee: Amount,
    pub ledger_fee: Amount,
    pub status: Status,
    pub created_at: Timestamp,
}

/// The status of a request.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pending,
    Succeeded { tx: BlockIndex },
    Failed { err: String },
}

/// Passes the current state of the ledger to the given function.
pub fn read_ledger_state<F, R>(token: Token, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    read_state(|s| {
        f(s.icp
            .ledger
            .get(&token)
            .unwrap_or_else(|| unreachable!("BUG: failed to lookup ledger for {:?}", token)))
    })
}
