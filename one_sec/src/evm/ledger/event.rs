use ic_ethereum_types::Address;
use minicbor::{Decode, Encode};

use crate::{
    api::types::{EvmChain, Token},
    config::OperatingMode,
    event::EventType,
    evm::{self, ledger::state::Status, reader::TxLogId},
    flow::{event::Operation, state::FlowId},
    numeric::{Amount, Timestamp},
};

use super::state::{Request, State};

/// An event of the EVM ledger state machine.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    /// Started the given operation for the given flow.
    #[n(0)]
    Started {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        op: Operation,
        #[n(2)]
        account: Address,
        #[n(3)]
        amount: Amount,
    },

    /// The previously started operation has succeeded.
    #[n(1)]
    Succeeded {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        tx: TxLogId,
    },

    /// The previously started operation has failed.
    #[n(2)]
    Failed {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        tx: TxLogId,
        #[n(2)]
        err: String,
    },
}

impl Event {
    pub fn wrap(self, chain: EvmChain, token: Token) -> EventType {
        EventType::Evm {
            chain,
            event: evm::Event::Ledger { token, event: self },
        }
    }
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, _time: Timestamp) {
    match event {
        Event::Started {
            id,
            op,
            account,
            amount,
        } => on_started(state, id, op, account, amount),
        Event::Succeeded { id, tx } => on_succeeded(state, id, tx),
        Event::Failed { id, tx, err } => on_failed(state, id, tx, err),
    }
}

fn on_started(state: &mut State, id: FlowId, op: Operation, account: Address, amount: Amount) {
    let request = Request {
        id,
        op,
        account,
        amount,
        status: Status::Pending,
    };

    // `config.operating_mode` will never change, so it is okay to depend on it here.
    match (state.config.operating_mode, request.op) {
        (OperatingMode::Minter, Operation::Burn) | (OperatingMode::Locker, Operation::Unlock) => {
            assert!(
                state.balance() >= amount,
                "BUG: insufficient balance for operation {:?}/{:?}: {} vs {}",
                state.config.token,
                request.op,
                state.balance(),
                amount,
            );
            state.pending_balance_sub = state.pending_balance_sub.add(
                request.amount,
                "BUG: overflow in pending_balance_sub += amount",
            );
        }

        (OperatingMode::Minter, Operation::Mint) | (OperatingMode::Locker, Operation::Lock) => {
            state.pending_balance_add = state.pending_balance_add.add(
                request.amount,
                "BUG: overflow in pending_balance_add += amount",
            );
        }
        _ => {
            unreachable!(
                "BUG: on_started: impossible combination of ledger and op: {:?} {:?}",
                state.config.operating_mode, request.op
            )
        }
    }

    let success = state.pending.insert(id);
    assert!(success, "BUG: duplicate pending ledger request: {}", id);

    let overwritten = state.requests.insert(id, request);
    assert!(
        overwritten.is_none(),
        "BUG: duplicate ledger request: {}",
        id
    );
}

fn on_succeeded(state: &mut State, id: FlowId, tx: TxLogId) {
    let request = state
        .requests
        .get_mut(&id)
        .unwrap_or_else(|| unreachable!("BUG: missing request: {}", id));

    assert_eq!(request.status, Status::Pending);
    request.status = Status::Succeeded { tx };

    let success = state.pending.remove(&id);
    assert!(success, "BUG: missing pending requests: {}", id);

    // `config.operating_mode` will never change, so it is okay to depend on it here.
    match (state.config.operating_mode, request.op) {
        (OperatingMode::Minter, Operation::Burn) | (OperatingMode::Locker, Operation::Unlock) => {
            if state.positive_balance >= request.amount {
                state.positive_balance = state.positive_balance.sub(
                    request.amount,
                    "BUG: underflow in positive_balance -= amount",
                );
            } else {
                let remaining = request.amount.sub(
                    state.positive_balance,
                    "BUG: underflow in amount -= positive_balance",
                );
                state.positive_balance = Amount::ZERO;
                state.negative_balance = state
                    .negative_balance
                    .add(remaining, "BUG: overflow in negative_balance += remaining");
            }

            state.pending_balance_sub = state.pending_balance_sub.sub(
                request.amount,
                "BUG: underflow in pending_balance_sub -= amount",
            );
        }

        (OperatingMode::Minter, Operation::Mint) | (OperatingMode::Locker, Operation::Lock) => {
            if state.negative_balance >= request.amount {
                state.negative_balance = state.negative_balance.sub(
                    request.amount,
                    "BUG: underflow in negative_balance -= amount",
                );
            } else {
                let remaining = request.amount.sub(
                    state.negative_balance,
                    "BUG: underflow in amount -= negative_balance",
                );
                state.negative_balance = Amount::ZERO;
                state.positive_balance = state
                    .positive_balance
                    .add(remaining, "BUG: overflow in positive_balance += remaining");
            }

            state.pending_balance_add = state.pending_balance_add.sub(
                request.amount,
                "BUG: underflow in pending_balance_add -= amount",
            );
        }
        _ => {
            unreachable!(
                "BUG: impossible combination of ledger and op: {:?} {:?}",
                state.config.operating_mode, request.op
            )
        }
    }
}

fn on_failed(state: &mut State, id: FlowId, tx: TxLogId, err: String) {
    let request = state
        .requests
        .get_mut(&id)
        .unwrap_or_else(|| unreachable!("BUG: on_failed: missing request: {}", id));

    assert_eq!(request.status, Status::Pending);
    request.status = Status::Failed { tx, err };

    let success = state.pending.remove(&id);
    assert!(success, "BUG: missing pending requests: {}", id);

    // `config.operating_mode` will never change, so it is okay to depend on it here.
    match (state.config.operating_mode, request.op) {
        (OperatingMode::Minter, Operation::Burn) | (OperatingMode::Locker, Operation::Unlock) => {
            state.pending_balance_sub = state.pending_balance_sub.sub(
                request.amount,
                "BUG: on_failed: underflow in pending_balance_sub -= amount",
            );
        }

        (OperatingMode::Minter, Operation::Mint) | (OperatingMode::Locker, Operation::Lock) => {
            state.pending_balance_add = state.pending_balance_add.sub(
                request.amount,
                "BUG: on_failed: underflow in pending_balance_add -= amount",
            );
        }
        _ => {
            unreachable!(
                "BUG: on_failed: impossible combination of ledger and op: {:?} {:?}",
                state.config.operating_mode, request.op
            )
        }
    }
}
