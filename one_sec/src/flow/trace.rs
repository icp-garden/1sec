//! Tracing improves UX by providing details about execution of a flow.
use candid::CandidType;
use ic_canister_log::log;
use serde::Deserialize;

use crate::{
    api::types::{self, Chain, EvmTx, IcpTx, Tx},
    icp,
    logs::ERROR,
    numeric::BlockNumber,
    state::{mutate_state, read_state},
    task::timestamp_ms,
};

use super::{event::TxId, state::FlowId};

const MAX_TRACES: usize = 1000;

/// A tracing event that happened during execution of a step of a bridging
/// transfer.
#[allow(clippy::enum_variant_names)]
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum TraceEvent {
    FetchTx,
    SignTx,
    SendTx,
    PendingConfirmTx,
    ConfirmTx,
}

/// A tracing event with additional information.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct TraceEntry {
    /// The chain where the event has happened.
    pub chain: Option<Chain>,
    /// The event itself.
    pub event: Option<TraceEvent>,
    /// The start timestamp.
    pub start: u64,
    /// The end timestamp.
    pub end: Option<u64>,
    /// The transaction that was being processed during the event.
    pub tx: Option<Tx>,
    /// The block number corresponding to `tx` if known.
    pub block_number: Option<u64>,
    /// The result of the event.
    pub result: Option<Result<(), String>>,
}

/// Append-only log of tracing events.
#[derive(CandidType, Clone, Debug, Default, Deserialize)]
pub struct Trace {
    pub entries: Vec<TraceEntry>,
}

/// Finds the trace of the given flow.
pub fn lookup(id: FlowId) -> Trace {
    read_state(|s| s.flow.traces.get(&id).cloned().unwrap_or_default())
}

/// Records a successful trace event for the given flow.
pub fn ok(id: FlowId, event: TraceEvent, tx: TxId, block_number: Option<BlockNumber>) {
    match entry(id, tx, block_number, event, Ok(())) {
        Some(entry) => {
            add(id, entry);
        }
        None => {
            log!(ERROR, "BUG: failed to build trace entry for {}", id);
        }
    }
}

/// Records a failed trace event for the given flow.
pub fn err<T: AsRef<str>>(
    id: FlowId,
    event: TraceEvent,
    tx: TxId,
    block_number: Option<BlockNumber>,
    err: T,
) {
    match entry(id, tx, block_number, event, Err(err.as_ref().into())) {
        Some(entry) => {
            add(id, entry);
        }
        None => {
            log!(ERROR, "BUG: failed to build trace entry for {}", id);
        }
    }
}

fn entry(
    id: FlowId,
    tx: TxId,
    block_number: Option<BlockNumber>,
    event: TraceEvent,
    result: Result<(), String>,
) -> Option<TraceEntry> {
    let now = timestamp_ms();
    let input = read_state(|s| s.flow.flow.get(&id).as_ref().map(|f| f.input.clone()))?;
    let ledger = icp::ledger::read_ledger_state(input.icp_token, |s| s.config.canister);

    let chain = match &tx {
        TxId::Icp(..) => Chain::ICP,
        TxId::Evm(..) => input.evm_chain.into(),
    };

    let tx = match tx {
        TxId::Icp(block_index) => types::Tx::Icp(IcpTx {
            ledger,
            block_index: block_index.into_inner(),
        }),
        TxId::Evm(tx_log_id) => types::Tx::Evm(EvmTx {
            hash: tx_log_id.tx_hash.to_string(),
            log_index: Some(tx_log_id.index.into_inner()),
        }),
    };

    Some(TraceEntry {
        chain: Some(chain),
        event: Some(event),
        start: now.into_inner(),
        end: Some(now.into_inner()),
        tx: Some(tx),
        block_number: block_number.map(|x| x.into_inner()),
        result: Some(result),
    })
}

fn add(id: FlowId, entry: TraceEntry) {
    mutate_state(|s| {
        while s.flow.traces.len() > MAX_TRACES {
            s.flow.traces.pop_first();
        }
        s.flow.traces.entry(id).or_default().entries.push(entry);
        id
    });
}
