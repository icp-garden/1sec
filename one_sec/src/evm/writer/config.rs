use std::time::Duration;

use crate::{
    evm::fee::TxFee,
    numeric::{Percent, TxNonce},
};

/// The configuration parameters of the EVM writer state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The initial transaction nonce to start from.
    pub initial_nonce: TxNonce,

    /// The initial transaction fee estimate.
    pub initial_fee_estimate: TxFee,

    /// If a transaction doesn't make progress, then its fee is increased by
    /// this many percent.
    pub tx_fee_bump: Percent,

    /// The margin over the transaction cost to amortize fee spikes.
    pub tx_fee_margin: Percent,

    /// The maximum size of a batch of transactions to sign in parallel.
    pub tx_sign_batch: usize,

    /// If a transaction doesn't make progress for this duration, then
    /// a new transaction with a higher fee is created.
    pub tx_resubmit_delay: Duration,

    /// The time between sending a transaction and retrying again.
    pub tx_resend_delay: Duration,

    /// The time between signing a transaction and sending it.
    pub tx_sign_to_send_delay: Duration,

    /// The time between signing a transaction and polling for its receipt.
    pub tx_sign_to_poll_delay: Duration,

    /// The time between sending a transaction and polling for its receipt.
    pub tx_send_to_poll_delay: Duration,

    /// The time between fetching fee estimates.
    pub fetch_fee_estimate_delay: Duration,
}
