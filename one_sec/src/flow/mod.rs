//! This state machine keeps track of bridging between EVM and ICP.
//!
//! When a user's bridging request is received and validated, the state machine
//! gets an [input event](Event::Input) with details about the source and
//! destination assets and the address of the recipient.
//!
//! To handle the event the state machine creates a new [Flow] entry
//! and [prepares an execution plan](prepare_steps) for the flow.
//!
//! The plan always consists of two [steps](Step):
//! - the first step is either [Operation::Burn] or [Operation::Lock] on the source chain,
//! - the second step is correspondingly [Operation::Unlock] or [Operation::Mint] on the destination chain.
//!
//! Both steps are initially marked as [Progress::Planned].
//!
//! After that different components of the canister work to complete the steps
//! and send the following events to this state machine:
//! [Event::StartedStep], [Event::SucceededStep], and
//! [Event::FailedStep].
//!
//! To make it concrete, let's walk through some example flows.
//!
//! ## Flow example 1: `USDC` on EVM to `USDC` on ICP
//!
//! 1. The user calls `approve(amount, locker)` on the standard `USDC` contract.
//! 2. The user calls `lock(amount, to_principal, to_subaccount)` on the locker helper
//!    contract.
//! 3. The locker transfers `USDC` to the canister's EVM address and emits a
//!    `LockedOne(from, amount, to_principal, to_subaccount)` log event.
//! 4. The user can optionally call the [transfer] endpoint of the canister to
//!    notify about incoming transfer and schedule tasks to fetch the EVM logs
//!    earlier. Otherwise, the canister schedules the tasks with longer delays.
//! 5. [evm::prover] periodically fetches the new latest and
//!    safe block headers using EVM RPC. Eventually, its safe block number
//!    reaches or goes above the block that contains the user's transactions.
//! 6. [evm::reader] periodically fetches logs events from its last
//!    fetched block to the safe block number of [evm::prover].
//!    Eventually, it fetches the `LockedOne` log event.
//! 7. [evm::reader] looks up a [Token] corresponding to the address of the log
//!    event emitter contract.
//! 8. [evm::reader] calls [evm::ledger::process_tx_log()] passing the chain,
//!    the token, and the log event.
//! 9. [evm::ledger::process_tx_log()] parses and validates the log event.
//! 10. If the log event is invalid then it sends [Event::InvalidInput] to the
//!     flow state machine.
//! 11. Otherwise, it creates a new flow by sending [Event::Input] the flow
//!     state machine.
//! 12. The flow has two planned steps: a lock and a mint.
//! 13. [evm::ledger::process_tx_log()] calls [evm::ledger::record_lock()] to send:
//!     - [Event::StartedStep] with [Operation::Lock].
//!     - [Event::SucceededStep] with [Operation::Lock].
//!     - [evm::ledger::Event::Started] with [Operation::Lock].
//!     - [evm::ledger::Event::Succeeded] with [Operation::Lock].
//! 14. [evm::ledger::process_tx_log()] calls [advance_flow_to_next_step] to
//!     start the next step of the flow, which in turn calls [icp::ledger::start_mint()].
//! 15. [icp::ledger::start_mint()] sends:
//!     - [Event::StartedStep] with [Operation::Mint].
//!     - [icp::ledger::Event::Started] with [Operation::Mint].
//! 16. Eventually, a task of [icp::ledger] calls the `transfer()` endpoint of
//!     the `USDC` ledger to mint the tokens to the recipient. That sends either
//!     [Event::SucceededStep] or [Event::FailedStep] to the flow state machine
//!     and finalizes the flow.
//! 17. It also sends an event to [icp::ledger] itself update the balance after minting.
//! 18. The user can learn about the outcome by calling the [transfer] endpoint
//!     to get the transfer id and then the [get_transfer] query.
//!
//! ## Flow example 2: `USDC` on ICP to `USDC` on EVM
//! 1. The user calls the `icrc2_approve()` endpoint of the `USDC` ledger
//!    specifying the canister as the spender.
//! 2. The user calls the [transfer()] endpoint of the canister.
//! 3. After inspecting the source and destination assets, that calls
//!    [icp_to_evm()] of the flow state machine.
//! 4. [icp_to_evm()] further validates the user input, computes fees and the
//!    amount of `USDC` to unlock.
//! 5. [icp_to_evm()] calls the `transferFrom()` endpoint of the `USDC` ledger.
//! 6. If the call fails, then it simply returns the error to the user without
//!    any changes in the state machine.
//! 7. Otherwise, it creates a new flow by sending [Event::Input] the flow
//!    state machine.
//! 8. The flow has two planned steps: a burn and an unlock.
//! 9.  [icp_to_evm()] calls [icp::ledger::record_burn()] to send:
//!     - [Event::StartedStep] with [Operation::Burn].
//!     - [Event::SucceededStep] with [Operation::Burn].
//!     - [icp::ledger::Event::Started] with [Operation::Burn].
//!     - [icp::ledger::Event::Succeeded] with [Operation::Burn].
//! 10. Afterwards, it calls [advance_flow_to_next_step] to
//!     start the next step of the flow, which in turn calls [evm::ledger::start_unlock()].
//! 11. [evm::ledger::start_unlock()] sends:
//!     - [Event::StartedStep] with [Operation::Unlock].
//!     - [evm::ledger::Event::Started] with [Operation::Unlock].
//!     - [evm::writer::Event::Started] with [Operation::Unlock] and the
//!       corresponding transaction request for sending.
//! 12. [evm::writer] marks the incoming transaction request as pending and
//!     starts processing it:
//!     1. It reserves the transaction nonce for this transaction request.
//!     2. In a periodic task, it creates and signs a new transaction based on
//!        the current fee estimates and sends [evm::writer::Event::SignedTx] to
//!        itself.
//!     3. In a periodic task, it sends the transaction to EVM RPC.
//!     4. In a periodic task, it fetches the transaction receipt from EVM RPC.
//!     5. If the receipt exists and its block number is at or below the safe
//!        block number from [evm::prover], then it finalizes the transaction
//!        request by calling [evm::writer::process_tx_receipt()].
//!     6. Otherwise, it repeats steps 2-5 with an increased transaction fee.
//!     7. Note: steps 3-5 can be accelerated with the help of off-chain
//!        relayers (see: [evm::prover]).
//! 13. [evm::writer::process_tx_receipt()] sends:
//!     - [evm::writer::Event::Finished]
//!     - [evm::ledger::Event::Succeeded] or [evm::ledger::Event::Failed] depending on the
//!       transaction receipt status.
//!     - [Event::SucceededStep] or [Event::FailedStep] depending on the
//!       transaction receipt status.
//! 14. The user can learn about the outcome by calling the [get_transfer] query
//!     with the transfer id returned by the initial [transfer()] call.
//!
#[cfg(doc)]
use crate::{
    api::{queries::get_transfer, types::Token, updates::transfer},
    evm, icp,
};
#[cfg(doc)]
use endpoint::{advance_flow_to_next_step, icp_to_evm};
#[cfg(doc)]
use event::{prepare_steps, Input, Operation};
#[cfg(doc)]
use state::{Flow, Progress, Step};

pub use event::{apply_event, Event};
pub use state::State;

pub mod config;
pub mod endpoint;
pub mod event;
pub mod state;
pub mod trace;

#[cfg(test)]
mod tests;
