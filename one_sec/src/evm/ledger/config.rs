use ic_ethereum_types::Address;

use crate::{
    api::types::Token,
    config::OperatingMode,
    numeric::{Amount, GasAmount, Wei},
};

/// The configuration parameters of the EVM ledger state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The token of the ledger.
    pub token: Token,
    /// The operating mode of the ledger.
    pub operating_mode: OperatingMode,
    /// The decimals in the ERC20 contract of the token.
    pub decimals: u8,
    /// The address of the ERC20 contract of the token.
    pub erc20_address: Address,
    /// The limit on the cost of the transaction.
    pub max_tx_cost: Wei,
    /// The gas limit for executing an unlock/mint transaction.
    pub gas_limit_for_unlock_or_mint: GasAmount,
    /// The gas limit for executing an lock/burn transaction.
    pub gas_limit_for_lock_or_burn: GasAmount,
    /// The gas limit for executing an approve transaction.
    pub gas_limit_for_approve: GasAmount,
    /// The address of the helper contract that emits the log events.
    /// Note: in case of [OperatingMode::Minter], this address is the same as
    /// the address of the ERC20 contract.
    pub logger_address: Address,
    /// The topics of the log event corresponding to the lock/burn transaction.
    pub logger_topics: [[u8; 32]; 4],
    /// The initial balance of the ledger.
    pub initial_balance: Amount,
}
