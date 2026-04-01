//! Helpers for interacting with the EVM RPC canister.
use candid::Principal;
use evm_rpc_client::{EvmRpcClient, IcRuntime, OverrideRpcConfig};
use evm_rpc_types::{
    GetLogsRpcConfig, HttpOutcallError, MultiRpcResult, RpcError, RpcResult, RpcServices,
};
use ic_cdk::api::call::RejectionCode;

use crate::logs::{PrintProxySink, TRACE_HTTP};

/// The configuration parameters related to the EVM RPC canister.
#[derive(Debug, Clone)]
pub struct Config {
    /// The EVM RPC nodes for requests that require consensus.
    pub rpc_services: RpcServices,
    /// The EVM RPC nodes for requests that don't require consensus such as
    /// fetching fees and sending transaction.
    pub rpc_service: RpcServices,
    /// The EVM RPC canister.
    pub evm_rpc_canister_id: Principal,
    /// The amount of cycles to attach to a call to the EVM RPC canister.
    pub evm_rpc_canister_cycles: u64,
    /// The minimum number of RPC nodes that need to agree in order to accept an
    /// RPC response that requires consensus.
    pub consensus_threshold: usize,
    /// The maximum number of blocks supported in `eth_getLogs`.
    pub get_logs_max_block_range: usize,
}

/// The result of an EVM RPC call using multiple RPC nodes.
pub enum ConsensusResult<T> {
    /// The required threshold of RPC nodes agreed on the result.
    Consensus(Result<T, RpcError>),
    /// RPC nodes returned different results.
    NoConsensus(Vec<Result<T, RpcError>>),
}

/// Returns a client for the EVM RPC canister.
pub fn build_evm_rpc_client(
    config: &Config,
    threshold: usize,
) -> EvmRpcClient<IcRuntime, PrintProxySink> {
    if threshold == 1 {
        EvmRpcClient::builder_for_ic(TRACE_HTTP)
            .with_providers(config.rpc_service.clone())
            .with_evm_canister_id(config.evm_rpc_canister_id)
            .with_min_attached_cycles(config.evm_rpc_canister_cycles as u128)
            .with_override_rpc_config(OverrideRpcConfig {
                eth_get_logs: Some(GetLogsRpcConfig {
                    max_block_range: Some(config.get_logs_max_block_range as u32),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .build()
    } else {
        EvmRpcClient::builder_for_ic(TRACE_HTTP)
            .with_providers(config.rpc_services.clone())
            .with_evm_canister_id(config.evm_rpc_canister_id)
            .with_min_attached_cycles(config.evm_rpc_canister_cycles as u128)
            .with_override_rpc_config(OverrideRpcConfig {
                eth_get_logs: Some(GetLogsRpcConfig {
                    max_block_range: Some(config.get_logs_max_block_range as u32),
                    ..Default::default()
                }),
                ..Default::default()
            })
            .build()
    }
}

/// Returns true if the RPC response was too large to fit in canister call
/// result.
pub fn is_response_too_large(err: &RpcError) -> bool {
    match err {
        RpcError::HttpOutcallError(err) => match err {
            HttpOutcallError::IcError { code, message } => {
                code == &RejectionCode::SysFatal
                    && (message.contains("size limit") || message.contains("length limit"))
            }
            HttpOutcallError::InvalidHttpJsonRpcResponse { .. } => false,
        },
        RpcError::ProviderError(_) | RpcError::JsonRpcError(_) | RpcError::ValidationError(_) => {
            false
        }
    }
}

/// Checks if there is consensus among the RPC nodes about the result of an RPC
/// call.
/// # Parameters
/// - result: the result returned by the EVM RPC canister.
/// - threshold: the minimum number of RPC nodes that need to agree on the result.
pub fn consensus<T>(result: MultiRpcResult<T>, threshold: usize) -> ConsensusResult<T>
where
    T: Eq + Clone,
{
    match result {
        MultiRpcResult::Consistent(r) => ConsensusResult::Consensus(r),
        MultiRpcResult::Inconsistent(items) => {
            let mut majority = pick_largest_group(items.into_iter().map(|x| x.1).collect());
            if majority.len() >= threshold {
                match majority.pop() {
                    Some(r) => ConsensusResult::Consensus(r),
                    None => ConsensusResult::NoConsensus(majority),
                }
            } else {
                ConsensusResult::NoConsensus(majority)
            }
        }
    }
}

/// Returns the largest group of items that are equal to each other in the given
/// vector of items.
fn pick_largest_group<T>(items: Vec<T>) -> Vec<T>
where
    T: Eq + Clone,
{
    let mut groups: Vec<Vec<T>> = vec![];

    for item in items {
        let mut found_group = false;
        for group in groups.iter_mut() {
            if &item == group.first().unwrap() {
                found_group = true;
                group.push(item.clone());
                break;
            }
        }
        if !found_group {
            groups.push(vec![item]);
        }
    }

    if let Some(largest) = groups.iter().map(|group| group.len()).max() {
        for group in groups {
            if group.len() == largest {
                return group;
            }
        }
    }

    vec![]
}

/// If the multi-rpc result has at least one ok result, then that is returned.
/// Otherwise, some error result is returned.
pub fn pick_any_ok<T>(result: MultiRpcResult<T>) -> RpcResult<T>
where
    T: Clone,
{
    match result {
        MultiRpcResult::Consistent(r) => r,
        MultiRpcResult::Inconsistent(mut items) => {
            for (_, r) in items.iter() {
                if let Ok(r) = r {
                    return Ok(r.clone());
                }
            }
            match items.pop() {
                Some(r) => r.1,
                None => Err(RpcError::ValidationError(
                    evm_rpc_types::ValidationError::Custom(
                        "BUG: pick_any_ok: empty set of result".to_string(),
                    ),
                )),
            }
        }
    }
}

/// Applies the given function to the multi-rpc result.
pub fn map_multi_rpc<T, S, F>(result: MultiRpcResult<T>, f: F) -> MultiRpcResult<S>
where
    F: Fn(RpcResult<T>) -> RpcResult<S>,
{
    match result {
        MultiRpcResult::Consistent(r) => MultiRpcResult::Consistent(f(r)),
        MultiRpcResult::Inconsistent(items) => {
            MultiRpcResult::Inconsistent(items.into_iter().map(|(s, r)| (s, f(r))).collect())
        }
    }
}
