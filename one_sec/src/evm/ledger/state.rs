use ic_ethereum_types::Address;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    api::types::{EvmChain, Token},
    config::OperatingMode,
    evm::{reader::TxLogId, state::read_evm_state},
    flow::{event::Operation, state::FlowId},
    numeric::Amount,
};

use super::config::Config;

/// The state of the EVM ledger state machine.
#[derive(Debug)]
pub struct State {
    /// The token balance as a sum of negative and positive balances.
    /// Its meaning depends on the operating mode:
    /// - locker: token balance of the canister in the ERC20 contract.
    /// - minter: the circulating supply of the token.
    ///
    /// The current balance after accounting for the pending requests
    /// can be computed as:
    /// - `positive_balance + pending_balance_add - negative_balance - pending_balance_sub`.
    pub positive_balance: Amount,
    pub negative_balance: Amount,

    /// The sum of all pending requests that are going to increase `balance` when
    /// completed.
    pub pending_balance_add: Amount,

    /// The sum of all pending requests that are going to decrease `balance` when
    /// completed.
    pub pending_balance_sub: Amount,

    /// All pending requests.
    pub pending: BTreeSet<FlowId>,

    /// All received requests.
    pub requests: BTreeMap<FlowId, Request>,

    /// The configuration parameters (immutable).
    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            positive_balance: config.initial_balance,
            negative_balance: Amount::ZERO,
            pending_balance_add: Amount::ZERO,
            pending_balance_sub: Amount::ZERO,
            pending: Default::default(),
            requests: Default::default(),
            config,
        }
    }

    pub fn maybe_balance(&self) -> Option<Amount> {
        self.positive_balance
            .checked_add(self.pending_balance_add)?
            .checked_sub(self.negative_balance)?
            .checked_sub(self.pending_balance_sub)
    }

    pub fn balance(&self) -> Amount {
        self.maybe_balance().unwrap_or_else(|| {
            unreachable!(
                "BUG: underflow in balance: {} {} {} {}",
                self.positive_balance,
                self.negative_balance,
                self.pending_balance_add,
                self.pending_balance_sub
            )
        })
    }

    pub fn available(&self) -> Option<Amount> {
        match self.config.operating_mode {
            OperatingMode::Minter => None,
            OperatingMode::Locker => Some(self.balance()),
        }
    }
}

/// A request to execute a token operation: burn/mint/lock/unlock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: FlowId,
    pub op: Operation,
    pub account: Address,
    pub amount: Amount,
    pub status: Status,
}

/// The status of a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Pending,
    Succeeded { tx: TxLogId },
    Failed { tx: TxLogId, err: String },
}

/// Passes the current state of the ledger to the given function.
pub fn read_ledger_state<F, R>(chain: EvmChain, token: Token, f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    read_evm_state(chain, |s| {
        f(s.ledger.get(&token).unwrap_or_else(|| {
            unreachable!("BUG: failed to lookup evm ledger for {:?}", (chain, token))
        }))
    })
}
