use ic_ethereum_types::Address;
use minicbor::{Decode, Encode};

use crate::{
    api::types::EvmChain,
    evm::tx::TxHash,
    numeric::{BlockNumber, Timestamp, TxLogIndex},
};

use super::State;

/// An event of the reader state machine.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    /// An event log has been fetched.
    #[n(0)]
    FetchedTxLog {
        #[n(0)]
        block_number: BlockNumber,
        #[n(1)]
        tx_log_id: TxLogId,
    },

    /// All event logs of the given block have been fetched.
    #[n(1)]
    FetchedBlock(#[n(0)] BlockNumber),
}

impl Event {
    pub fn wrap(self, chain: EvmChain) -> crate::event::EventType {
        crate::event::EventType::Evm {
            chain,
            event: crate::evm::Event::Reader(self),
        }
    }
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, _time: Timestamp) {
    match event {
        Event::FetchedTxLog {
            block_number,
            tx_log_id,
        } => {
            on_fetched_tx_log(state, block_number, tx_log_id);
        }
        Event::FetchedBlock(block) => {
            on_fetched_block(state, block);
        }
    }
}

/// A partially parsed transaction log with raw topics and data.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
pub struct TxLog {
    #[n(0)]
    pub id: TxLogId,
    #[n(1)]
    pub block: BlockNumber,
    #[n(2)]
    pub contract: Address,
    #[cbor(n(3), with = "minicbor::bytes")]
    pub topic: [u8; 32],
    #[cbor(n(4), with = "minicbor::bytes")]
    pub data: Vec<u8>,
}

/// The id of an event log consisting of the transaction hash and the index of
/// the log within the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub struct TxLogId {
    #[n(0)]
    pub tx_hash: TxHash,
    #[n(1)]
    pub index: TxLogIndex,
}

fn on_fetched_tx_log(state: &mut State, block_number: BlockNumber, tx_log_id: TxLogId) {
    let last_fully_fetched_block = state.last_fully_fetched_block.unwrap_or(BlockNumber::ZERO);

    assert!(
        block_number > last_fully_fetched_block,
        "BUG: attempt to ingest a withdrawal of an old block: {} > {}",
        block_number,
        last_fully_fetched_block
    );

    let success = state.done.insert(tx_log_id);
    assert!(success, "BUG: duplicate tx log entry: {:?}", tx_log_id);
}

fn on_fetched_block(state: &mut State, block: BlockNumber) {
    let last_fully_fetched_block = state.last_fully_fetched_block.unwrap_or(BlockNumber::ZERO);

    assert!(
        block > last_fully_fetched_block,
        "BUG: attempt to ingest a block older than the last fully ingested block: {} > {}",
        block,
        last_fully_fetched_block
    );

    state.last_fully_fetched_block = Some(block);

    while state.unconfirmed_blocks.first().cloned().unwrap_or(block) < block {
        state.unconfirmed_blocks.pop_first();
    }
}
