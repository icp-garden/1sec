//! This state machine interacts with an ERC20 [Token] contract on EVM.
//!
//! There is an instance of this state machine per EVM [Token].
//!
//! Depending on [Token] the state machine operates in one of the two modes:
//! - [OperatingMode::Locker]
//! - [OperatingMode::Minter]
//!
//! In the locking mode, there is an additional helper EVM contract for locking
//! the ERC20 token like `USDC` and emitting log events.
//! The contract is defined in `contracts/evm/Locker.sol`.
//!
//! The minting mode does not requires the helper contract because the custom
//! ERC20 contract emits the necessary log events. The contract is defined in
//! `contracts/evm/Token.sol`.
//!
//! The [flow] state machine uses this state machine to:
//! - lock and unlock tokens or
//! - mint and burn tokens.
//!
//! ## Entry points
//! - [record_lock()]: called by [evm::ledger::process_tx_log()] upon detection
//!   of locked tokens on the EVM side.
//! - [record_burn()]: called by [evm::ledger::process_tx_log()] upon detection
//!   of burned tokens on the EVM side.
//! - [start_unlock()]: called by [flow::endpoint::icp_to_evm()] after
//!   successfully burning tokens on the ICP side.
//! - [start_mint()]: called by [flow::endpoint::icp_to_evm()] after
//!   successfully locking tokens on the ICP side.
//!
//! ## Lock and burn
//!
//! The lock and burn operations are synchronous in the context of this state
//! machine because it learns about these operations post-factum -- after
//! fetching events logs from the EVM contract.
//!
//! In other words, [record_lock()] and [record_burn()] simply send
//! [Event::Started] and [Event::Succeeded], which means that other tasks will never
//! observe lock and mint operations in [State::pending].
//!
//! ## Unlock and mint
//!
//! The unlock and mint require executing a transfer transaction of the ERC20
//! contract on the EVM chain.
//!
//! Example flow of unlocking `USDC`:
//!
//! - [start_unlock()] sends [Event::Started] with [Operation::Unlock].
//!   That remembers the current request in [State::requests] and adds the
//!   flow id to [State::pending]. It also sends [Event::Started] to
//!   [evm::writer] such that the writer starts sending the transaction.
//!   Finally, it sends events to [flow] to notify it that the unlocking step
//!   has started.
//!
//! - [evm::writer] sends the transaction to the EVM chain using the EVM RPC and
//!   possibly off-chain relayers.
//!
//! - When the transaction executes and [evm::writer] gets the transaction
//!   receipt, then this state machine receives [Event::Succeeded] or
//!   [Event::Failed] depending on the receipt status. It also sends the
//!   corresponding events to [flow] to notify it that the unlocking has finished.
//!
//! ## Locker vs Minter
//!
//! Lock is similar to burn and unlock is similar to mint.
//!
//! There are two differences between the operating modes:
//! - balance accounting: balances move in opposite directions for a locker and
//!   a minter because the locker keeps track of the balance of the canister
//!   whereas the minter keeps track of the circulating supply.
//! - helper contract: the locker needs a helper contract in addition to the
//!   ERC20 contract whereas the minter needs only the ERC20 contract.
#[cfg(doc)]
use crate::{api::types::Token, config::OperatingMode, evm, flow, flow::event::Operation};

pub use config::Config;
pub use event::{apply_event, Event};
pub use parser::encode_icp_account;
pub use state::State;
pub use task::{
    estimate_tx_cost, process_tx_log, record_burn, record_lock, start_mint, start_unlock,
};
pub use tx::{call_burn_or_lock_tx, call_tx_with_address_and_amount};

mod config;
mod event;
mod parser;
mod state;
mod task;
mod tx;

#[cfg(test)]
mod tests;
