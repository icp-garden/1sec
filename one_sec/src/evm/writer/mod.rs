//! This state machine sends transactions to EVM chain.
//!
//! It gets transaction requests from [evm::ledger].
//!
//! The approach of sending transactions roughly follows the ckETH/ckUSDC
//! implementation:
//! - [State::next_nonce] keeps track of the transaction nonce.
//! - When a new transaction request comes, then transaction nonce is reserved
//!   for it and cannot be used for another transaction request.
//! - All transaction data of the request comes from [evm::ledger] except for
//!   the transaction fee, which is set by the writer.
//! - The writer bumps the fee if the transaction doesn't get executed for a
//!   long time.
//!
//! ## Transaction request flow
//!
//! 1. [evm::ledger] sends [Event::Started] to this state machine, which adds the request
//!    into [State::pending] and reserves [State::next_nonce] for it.
//! 2. [Task::NewTx] creates a new EVM transactions from the transaction data
//!    and (possibly bumped) transaction fee.
//! 3. [Task::NewTx] calls ECDSA to sign the newly created transaction.
//! 4. [Task::NewTx] sends an [Event::SignedTx] event once the transaction is
//!    successfully signed. As a result, the transaction hash is added to the
//!    [TxRequest::signed] list.
//! 5. [Task::NewTx] adds the signed transaction to the [TxRequest::sending]
//!    list, but without an event.
//! 4. [Task::SendTx] sends the transactions in [TxRequest::sending].
//! 5. [Task::PollTx] fetches the transaction receipt for each transaction in [TxRequest::signed].
//!    If the block number of the receipt is at or below
//!    [evm::prover::head::State::safe] meaning that it is safe, then the transaction
//!    request is finalized in [process_tx_receipt()]:
//!    - send [Event::Finished] to this state machine to marked the transaction
//!      request as done.
//!    - send [evm::ledger::Event::Succeeded] or [evm::ledger::Event::Failed] depending on the
//!      transaction status.
//!    - send [flow::event::Event::SucceededStep] or [flow::event::Event::FailedStep]
//!      depending on the transaction status.
//!
//! ## Fee estimate
//!
//! Fetching the transaction fee estimation works to similar to that of
//! ckETH/ckUSDC. [Task::UpdateFeeEstimate] periodically fetches
//! `eth_fee_history` using EVM RPC and computes `max_fee_per_gas` /
//! `max_priority_fee_per_gas`.
//!
#[cfg(doc)]
use crate::{api::types::Token, config::OperatingMode, evm, flow, flow::event::Operation};

pub use config::Config;
pub use event::{apply_event, Event, TxInput};
pub use state::{State, TxRequest};
pub use task::{
    apply_confirmed_proofs, fee_history_args, get_fee_from_history, increment_pending_receipt,
    process_tx_receipt, queue_position, Task,
};

mod config;
mod event;
mod state;
mod task;

#[cfg(test)]
mod tests;
