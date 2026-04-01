//! This state machine fetches event logs from EVM contracts.
//!
//! The approach of fetching logs roughly follows the ckETH/ckUSDC
//! implementation:
//! - [State::last_fully_fetched_block] separates processed blocks from
//!   unprocessed blocks. A block is processed when all its event logs are
//!   fetched fetched and processed.
//! - [Task::FetchTxLogs] periodically fetches events logs in blocks between
//!   [State::last_fully_fetched_block] to [evm::prover::head::State::safe]
//!   using EVM RPC.
//! - If there are too many logs in the block range and they don't fit into an
//!   HTTP outcall response, then the range of blocks is halved and fetching is
//!   retried.
//!
//! ## Subscription
//!
//! Each [evm::ledger] adds itself to [State::subscriptions] providing the EVM
//! contract address that emits the event logs and also providing the event log
//! topic.
//!
//! ## Processing a fetched event log
//!
//! For each event log [Task::FetchTxLogs] looks up the target [evm::ledger] and
//! calls its [evm::ledger::process_tx_log()], which records lock/burn of the
//! EVM token and starts mint/unlock of the corresponding ICP token.
//!
#[cfg(doc)]
use crate::{api::types::Token, config::OperatingMode, evm, flow, flow::event::Operation};

pub use config::Config;
pub use event::{apply_event, Event, TxLog, TxLogId};
pub use state::{State, Subscription};
pub use task::{
    schedule_tx_logs_task_if_unconfirmed_blocks, schedule_tx_logs_task_sooner_if_too_many_blocks,
    Task,
};

mod config;
mod event;
mod state;
mod task;

#[cfg(test)]
mod tests;
