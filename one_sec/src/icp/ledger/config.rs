//! This module defines the configuration parameters of the ICP ledger state machine.
use candid::Principal;
use std::time::Duration;

use crate::{api::types::Token, config::OperatingMode, numeric::Amount};

/// The configuration parameters of the ICP ledger state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The token of the ledger.
    pub token: Token,
    /// The operating mode of the ledger.
    pub operating_mode: OperatingMode,
    /// The decimals in the ICRC2 ledger canister.
    pub decimals: u8,
    /// The ICRC2 ledger canister.
    pub canister: Principal,
    /// The associated index canister.
    pub index_canister: Option<Principal>,
    /// Indicates whether the ledger supports account identifiers or not.
    /// This is generally true only for the ICP token ledger.
    pub supports_account_id: bool,
    /// The initial balance of the ledger.
    pub initial_balance: Amount,
    /// The receiver of the collected fees.
    pub fee_receiver: Principal,
    /// The collected fees will be transferred to the receiver if they exceed
    /// this threshold.
    pub fee_threshold: Amount,
    /// How many transfers to perform in parallel
    pub transfer_batch: usize,
    /// The transfer fee that the canister charges.
    pub transfer_fee: Amount,
    /// A delay between transfer tasks when there are pending transfer.
    pub transfer_task_busy_delay: Duration,
    /// A delay between transfer tasks when there are no pending transfers.
    pub transfer_task_idle_delay: Duration,
    /// A delay between transfer fee tasks.
    pub transfer_fee_task_delay: Duration,
}
