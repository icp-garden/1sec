//! This module defines the canister API: endpoints and candid types.

pub mod icrc21;
pub mod queries;
pub mod types;
pub mod updates;

#[cfg(feature = "dev")]
pub mod dev;

use candid::CandidType;
use serde::Deserialize;

use crate::state::read_state;

/// All query and update endpoints of the canister.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Endpoint {
    ForwardEvmToIcp,
    GetCanisterCalls,
    GetEvents,
    GetEvmAddress,
    GetForwardingAccounts,
    GetForwardingAddress,
    GetForwardingTransactions,
    GetMetadata,
    GetRelayTasks,
    GetTransfer,
    GetTransfers,
    GetWeiPerIcpRate,
    HttpRequest,
    RunTask,
    ScheduleAllTasks,
    SubmitForwardingUpdate,
    SubmitRelayProof,
    Transfer,
    TransferEvmToIcp,
    TransferIcpToEvm,
    UpdateEstimates,
    ValidateForwardingAddress,
}

fn is_endpoint_paused(endpoint: Endpoint) -> bool {
    read_state(|s| s.paused_endpoints.contains(&endpoint))
}
