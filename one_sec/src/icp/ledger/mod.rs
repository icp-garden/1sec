//! This state machine interacts with an ICRC-2 ledger.
//!
//! There is an instance of this state machine per ICP [Token].
//!
//! Depending on [Token] the state machine operates in one of the two modes:
//! - [OperatingMode::Locker]
//! - [OperatingMode::Minter]
//!
//! The [flow] state machine uses this state machine to:
//! - lock and unlock tokens or
//! - mint and burn tokens.
//!
//! ## Entry points
//! - [record_lock()]: called by [flow::endpoint::icp_to_evm] upon
//!   a successfully transfer of tokens from the user's account to the
//!   canister's account.
//! - [record_burn()]: called by [flow::endpoint::icp_to_evm] upon
//!   a successfully burn of tokens from the user's account (by transferring them to
//!   the canister's account as the canister is the minter).
//! - [start_unlock()]: called by [evm::ledger::process_tx_log] upon detection
//!   of burned tokens on the EVM side.
//! - [start_mint()]: called by [evm::ledger::process_tx_log] upon detection
//!   of locked tokens on the EVM side.
//!
//! ## Lock and burn
//!
//! The lock and burn operations are synchronous in the context of this state
//! machine because it learns about these operations post-factum -- after they
//! have already been performed by [flow::endpoint::icp_to_evm].
//!
//! In other words, [record_lock()] and [record_burn()] simply send
//! [Event::Started] and [Event::Succeeded], which means that other tasks will never
//! observe lock and mint operations in [State::pending].
//!
//! ## Unlock and mint
//!
//! The unlock and mint require calling the `transfer()` endpoint of ledger canister.
//!
//! Example flow of unlocking `ICP`:
//!
//! - [start_unlock()] sends [Event::Started] with [Operation::Unlock].
//!   That remembers the current request in [State::requests] and adds the
//!   flow id to [State::pending]. It also sends events to [flow] to notify it
//!   that the unlocking step has started.
//!
//! - [Task::Transfer] iterates over all pending requests and makes a call to
//!   the `transfer()` endpoint of the ledger canister. It transfer the
//!   specified amount of `ICP` from the canister to the specified recipient.
//!
//! - When the transfer completes, the task sends [Event::Succeeded] or
//!   [Event::Failed] depending on the outcome. It also sends the corresponding
//!   events to [flow] to notify it that the unlocking has finished.
//!
//! Note that currently transfer calls happen sequentially one by one.
//! If this becomes a performance bottleneck, then the code will change to make
//! multiple concurrent calls.
//!
//! ## Locker vs Minter
//!
//! Lock is similar to burn and unlock is similar to mint.
//!
//! There are only two differences between the operating modes:
//! - balance accounting: balances move in opposite directions for a locker and
//!   a minter because the locker keeps track of the balance of the canister
//!   whereas the minter keeps track of the circulating supply.
//! - transfer fee: the minter must specify zero fees in transfer calls.
//!
//! ## Quarantine
//!
//! Protection against crashes and panics during or after the call:
//! 1. Just before making the call, the flow id is stored in [State::quarantine].
//!    Note that this commits the quarantine into Wasm memory.
//! 2. [State::quarantine] is emptied after processing the call and fully
//!    updating the state machine.
//!
//! This gives an invariant that [State::quarantine] should always be empty
//! before the next call unless Step 2 did not execute or was reverted.
//! If [State::quarantine] is not empty, then that flow marked as failed such
//! that there is no attempt to make a second transfer for it.
//!
#[cfg(doc)]
use crate::{api::types::Token, config::OperatingMode, evm, flow, flow::event::Operation};

pub use config::Config;
pub use event::{apply_event, Event};
pub use state::{read_ledger_state, State};
pub use task::{
    as_block_index, queue_position, record_burn, record_lock, start_mint, start_unlock, Task,
};

pub mod config;
pub mod event;
pub mod state;
pub mod task;

#[cfg(test)]
mod tests;
