use ic_ethereum_types::Address;
use minicbor::{Decode, Encode};
use std::collections::VecDeque;

use crate::{
    api::types::{EvmChain, Token},
    evm::{
        self,
        tx::{TxHash, TxReceipt},
        writer::state::TxRequest,
    },
    flow::{event::Operation, state::FlowId},
    numeric::{GasAmount, Timestamp, Wei},
};

use super::{state::SignedTx, State};

/// An event of the writer state machine.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    /// Received a request to send a transaction with the given input
    /// and started processing that request.
    #[n(0)]
    Started {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        token: Token,
        #[n(2)]
        op: Operation,
        #[n(3)]
        tx_input: TxInput,
    },

    /// Created and signed a new transaction corresponding to the given request.
    /// This event is needed to remember hashes of all transactions that were
    /// signed by the canister.
    ///
    /// Since events are persistent, the state machine can poll for all
    /// transaction receipts even after a canister upgrade.
    #[n(1)]
    SignedTx {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        tx_hash: TxHash,
    },

    /// One of the transactions of the given request has been executed.
    #[n(2)]
    Finished {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        tx_receipt: TxReceipt,
    },
}

impl Event {
    pub fn wrap(self, chain: EvmChain) -> crate::event::EventType {
        crate::event::EventType::Evm {
            chain,
            event: evm::Event::Writer(self),
        }
    }
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, time: Timestamp) {
    match event {
        Event::Started {
            id,
            token,
            op,
            tx_input,
        } => {
            on_started(state, id, token, op, tx_input);
        }
        Event::SignedTx { id, tx_hash } => on_signed_tx(state, id, tx_hash, time),
        Event::Finished { id, tx_receipt } => on_finished(state, id, tx_receipt),
    }
}

/// Transaction input that contains all information needed to create a new
/// transaction (except for the current transaction fee estimate).
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct TxInput {
    #[n(0)]
    pub contract: Address,
    #[n(1)]
    pub calldata: Vec<u8>,
    #[n(2)]
    pub gas_limit: GasAmount,
    #[n(3)]
    pub cost_limit: Wei,
}

fn on_started(state: &mut State, id: FlowId, token: Token, op: Operation, tx_input: TxInput) {
    let nonce = state.next_nonce;
    state.next_nonce = state
        .next_nonce
        .increment("BUG: overflow in record_input: next_nonce++");

    let overwritten = state.pending.insert(
        id,
        TxRequest {
            id,
            token,
            op,
            tx_input,
            nonce,
            signed: VecDeque::new(),
            sending: VecDeque::new(),
        },
    );
    assert!(overwritten.is_none());
}

fn on_signed_tx(state: &mut State, id: FlowId, tx_hash: TxHash, time: Timestamp) {
    // It is possible that there is no entry corresponding to `id` in
    // `pending` if another transaction has completed and finalized the write.
    // In that case, it is safe to ignore this event.

    state.pending.entry(id).and_modify(|pending| {
        pending.signed.push_back(SignedTx {
            tx_hash,
            sign_time: time,
        });
    });
}

fn on_finished(state: &mut State, id: FlowId, tx_receipt: TxReceipt) {
    // Since we received the receipt, the corresponding write should still be
    // in the pending list. Otherwise, it would mean that we received another
    // valid receipt for the same write (nonce), which is impossible unless
    // there is a fork in the EVM blockchain (or RPC node is malicious).
    let pending = state
        .pending
        .remove(&id)
        .expect("BUG: cannot find pending write for tx receipt");

    let tx_hash = tx_receipt.tx_hash;

    let maybe_tx = pending.signed.iter().find(|p| p.tx_hash == tx_hash);

    // Transactions are never removed from `pending`, so we should
    // always be able to find the pending transaction by its hash.
    let _tx = maybe_tx.expect("BUG: cannot find pending tx by its hash");

    let success = state.done_tx.insert(tx_hash);
    assert!(success, "BUG: duplicate done tx: {}, {}", id, tx_hash);

    let overwritten = state.done.insert(id, tx_receipt);
    assert!(overwritten.is_none(), "BUG: duplicate done flow: {}", id);
}
