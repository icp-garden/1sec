use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    api::types::Token,
    evm::{
        fee::{DailyFeeEstimate, FeeEstimate},
        tx::{SignedEip1559TransactionRequest, TxHash, TxReceipt},
        TxFee,
    },
    flow::{event::Operation, state::FlowId},
    numeric::{Timestamp, TxNonce},
};

use super::{event::TxInput, Config};

/// The state of the EVM writer state machine.
#[derive(Debug)]
pub struct State {
    /// Pending transaction requests.
    pub pending: BTreeMap<FlowId, TxRequest>,
    /// Receipts of all the executed transactions.
    pub done: BTreeMap<FlowId, TxReceipt>,
    /// Hashes of all the executed transactions.
    pub done_tx: BTreeSet<TxHash>,
    /// The next transaction nonce.
    pub next_nonce: TxNonce,
    /// The configuration parameters (immutable).
    pub config: Config,
    /// The most recently fetched fee estimate.
    /// Note: this state is ephemeral (cleared on upgrade).
    pub fetched_fee: Option<FeeEstimate>,
    /// The most recent fee submitted by a relayer.
    /// Note: this state is ephemeral (cleared on upgrade).
    pub relayed_fee: Option<FeeEstimate>,
    /// The daily average of the fetched fee estimates.
    /// Note: this state is ephemeral (cleared on upgrade).
    pub daily_average_fee: DailyFeeEstimate,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            pending: Default::default(),
            done: Default::default(),
            done_tx: Default::default(),
            fetched_fee: Default::default(),
            relayed_fee: Default::default(),
            daily_average_fee: Default::default(),
            next_nonce: config.initial_nonce,
            config,
        }
    }

    pub fn latest_fee(&self) -> Option<TxFee> {
        match (self.fetched_fee.as_ref(), self.relayed_fee.as_ref()) {
            (Some(a), Some(b)) => {
                if a.block_number < b.block_number {
                    Some(b.fee.clone())
                } else {
                    Some(a.fee.clone())
                }
            }
            (Some(a), None) => Some(a.fee.clone()),
            (None, Some(b)) => Some(b.fee.clone()),
            (None, None) => None,
        }
    }

    pub fn average_fee(&self) -> Option<TxFee> {
        self.daily_average_fee.average()
    }

    pub fn relay_fee(&mut self, fee: FeeEstimate) {
        match self.relayed_fee.as_ref() {
            Some(existing) => {
                if existing.block_number < fee.block_number {
                    self.relayed_fee = Some(fee)
                }
            }
            None => self.relayed_fee = Some(fee),
        }
    }
}

/// A request to send a transaction with the given input.
#[derive(Debug, Clone)]
pub struct TxRequest {
    /// The flow for which this request is made.
    pub id: FlowId,
    /// The token corresponding to this request.
    pub token: Token,
    /// The operation of this request: either unlock or mint.
    pub op: Operation,
    /// The transaction input that is used to created new transactions.
    pub tx_input: TxInput,
    /// The transaction nonce reserved for this request.
    pub nonce: TxNonce,
    /// A list of hashes of the signed transactions with timestamps.
    pub signed: VecDeque<SignedTx>,
    /// A list of transactions that are being sent.
    /// Note: this state is ephemeral (cleared on upgrade).
    pub sending: VecDeque<SendingTx>,
}

/// A transaction hash with a timestamp of when the transaction was signed.
#[derive(Debug, Clone)]
pub struct SignedTx {
    pub tx_hash: TxHash,
    pub sign_time: Timestamp,
}

/// A transaction that is being sent.
#[derive(Debug, Clone)]
pub struct SendingTx {
    /// The signed transaction.
    pub tx: SignedEip1559TransactionRequest,

    /// The time of signing.
    pub sign_time: Timestamp,

    /// The time of the last attempt to send.
    pub send_time: Option<Timestamp>,

    /// This counter is incremented if a transaction receipt is fetched for the
    /// transaction but the block is not safe yet.
    /// It is also incremented if a relayer submits a receipt proof
    /// for this transaction, but the proof is not confirmed yet.
    ///
    /// It is used to avoid resending the transaction that has likely been
    /// executed and only requires waiting for a safe block / confirmation.
    pub pending_receipts: usize,
}
