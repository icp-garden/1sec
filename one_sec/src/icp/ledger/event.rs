//! This module defines events of the ICP ledger state machine.
use minicbor::{Decode, Encode};

use crate::{
    api::types::Token,
    config::OperatingMode,
    event::EventType,
    flow::{event::Operation, state::FlowId},
    icp::{self, ledger::state::Status, IcpAccount},
    numeric::{Amount, BlockIndex, Timestamp},
};

use super::state::{Request, State};

/// An event of the ICP ledger state machine.
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
        account: IcpAccount,
        #[n(3)]
        amount: Amount,
        #[n(4)]
        collected_fee: Amount,
        #[n(5)]
        ledger_fee: Amount,
    },
    /// The previously started operation has succeeded.
    #[n(1)]
    Succeeded {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        op: Operation,
        #[n(2)]
        tx: BlockIndex,
    },
    /// The previously started operation has failed.
    #[n(2)]
    Failed {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        op: Operation,
        #[n(2)]
        err: String,
    },

    /// Transferred the collected fees to the recipient.
    #[n(3)]
    TransferredFee {
        #[n(0)]
        amount: Amount,
        #[n(1)]
        ledger_fee: Amount,
    },
}

impl Event {
    pub fn wrap(self, token: Token) -> EventType {
        EventType::Icp(icp::Event::Ledger { token, event: self })
    }
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, time: Timestamp) {
    match event {
        Event::Started {
            id,
            op,
            account,
            amount,
            collected_fee,
            ledger_fee,
        } => {
            on_started(
                state,
                id,
                op,
                account,
                amount,
                collected_fee,
                ledger_fee,
                time,
            );
        }
        Event::Succeeded { id, op, tx } => {
            on_succeeded(state, id, op, tx);
        }
        Event::Failed { id, op, err } => {
            on_failed(state, id, op, err);
        }
        Event::TransferredFee { amount, ledger_fee } => {
            on_transferred_fee(state, amount, ledger_fee);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn on_started(
    state: &mut State,
    id: FlowId,
    op: Operation,
    account: IcpAccount,
    amount: Amount,
    collected_fee: Amount,
    ledger_fee: Amount,
    time: Timestamp,
) {
    let request = Request {
        id,
        op,
        account,
        amount,
        collected_fee,
        ledger_fee,
        status: Status::Pending,
        created_at: time,
    };
    let overwritten = match op {
        Operation::Lock | Operation::Burn => state.lock_or_burn.insert(id, request),
        Operation::Unlock | Operation::Mint => state.unlock_or_mint.insert(id, request),
    };

    assert!(
        overwritten.is_none(),
        "BUG: duplicate ledger request: {}",
        id
    );

    let success = state.pending.insert(id);
    assert!(success, "BUG: duplicate pending request: {}", id);
}

fn on_succeeded(state: &mut State, id: FlowId, op: Operation, tx: BlockIndex) {
    let success = state.pending.remove(&id);
    assert!(success, "BUG: missing pending flow: {}", id);

    let request = match op {
        Operation::Lock | Operation::Burn => state
            .lock_or_burn
            .get_mut(&id)
            .unwrap_or_else(|| unreachable!("BUG: missing request: {}", id)),

        Operation::Unlock | Operation::Mint => state
            .unlock_or_mint
            .get_mut(&id)
            .unwrap_or_else(|| unreachable!("BUG: missing request: {}", id)),
    };

    assert_eq!(request.status, Status::Pending);
    request.status = Status::Succeeded { tx };

    let op = request.op;

    // `config.operating_mode` will never change, so it is okay to depend on it here.
    let mode = state.config.operating_mode;

    let add_to_balance = match (mode, op) {
        (OperatingMode::Minter, Operation::Burn) => false,
        (OperatingMode::Minter, Operation::Mint) => true,
        (OperatingMode::Locker, Operation::Lock) => true,
        (OperatingMode::Locker, Operation::Unlock) => false,
        _ => {
            unreachable!("BUG: invalid combination: {:?}", (mode, op));
        }
    };

    if add_to_balance {
        state.balance = state
            .balance
            .add(request.amount, "BUG: overflow in balance += amount");
    } else {
        state.balance = state
            .balance
            .sub(request.amount, "BUG: underflow in balance -= amount");
    }

    state.balance = state.balance.sub(
        request.ledger_fee,
        "BUG: underflow in balance -= ledger_fee",
    );

    state.fees = state.fees.add(
        request.collected_fee,
        "BUG: overflow in fees += collected_fee",
    );
}

fn on_failed(state: &mut State, id: FlowId, op: Operation, err: String) {
    let success = state.pending.remove(&id);
    assert!(success, "BUG: missing pending flow: {}", id);

    let request = match op {
        Operation::Lock | Operation::Burn => state
            .lock_or_burn
            .get_mut(&id)
            .unwrap_or_else(|| unreachable!("BUG: missing request: {}", id)),

        Operation::Unlock | Operation::Mint => state
            .unlock_or_mint
            .get_mut(&id)
            .unwrap_or_else(|| unreachable!("BUG: missing request: {}", id)),
    };
    assert_eq!(request.status, Status::Pending);
    request.status = Status::Failed { err };
}

fn on_transferred_fee(state: &mut State, amount: Amount, ledger_fee: Amount) {
    state.fees = state
        .fees
        .sub(amount, "BUG: underflow in fees -= amount")
        .sub(ledger_fee, "BUG: underflow in fees -= ledger_fee");

    // `config.operating_mode` will never change, so it is okay to depend on it here.
    match state.config.operating_mode {
        OperatingMode::Minter => {
            // The fee was minted, so the balance increases.
            state.balance = state
                .balance
                .add(amount, "BUG: overflow in balance += amount")
                .sub(ledger_fee, "BUG: underflow in balance -= ledger_fee");
        }
        OperatingMode::Locker => {
            // The fee was unlocked, so the balance decreases.
            state.balance = state
                .balance
                .sub(amount, "BUG: overflow in balance += amount")
                .sub(ledger_fee, "BUG: underflow in balance -= ledger_fee");
        }
    }
}
