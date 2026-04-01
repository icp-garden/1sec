//! This state machine interacts with an EVM chain.
//!
//! There is one instance of this state machine per EVM chain.
//!
//! It has the following components:
//! - [ledger]: one state machine per EVM [Token]
//!   - maintains the token balance.
//!   - supports lock, unlock, mint, and burn operations.
//!   - uses [writer] to send transactions to the ERC20 and locker contracts.
//!   - uses [reader] to read the EVM log events from the contracts.
//! - [prover]: fetches the latest/safe block headers and validates proofs from
//!   off-chain relayers.
//! - [writer]: a state machine that maintains the transaction nonce and sends
//!   transactions.
//! - [reader]: a state machine that fetches EVM log events from specified
//!   contracts.
//! - [evm_rpc]: config and helpers for calling the EVM RPC canister.
#[cfg(doc)]
use crate::api::types::Token;

pub use config::Config;
pub use event::{apply_event, Event};
pub use fee::TxFee;
pub use state::{mutate_evm_state, read_evm_state, State};
pub use task::Task;
pub use tx::{derive_address_from_public_key, TxHash};

pub mod evm_rpc;
pub mod forwarder;
pub mod ledger;
pub mod prover;
pub mod reader;
pub mod writer;

mod config;
mod event;
mod fee;
mod rlp_encode;
mod state;
mod task;
mod tx;
