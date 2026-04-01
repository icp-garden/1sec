//! The query endpoints of the canister.
use candid::Principal;
use evm_rpc_types::Hex32;
use ic_cdk::{api::is_controller, caller};
use ic_ethereum_types::Address;
use ic_http_types::{HttpRequest, HttpResponse, HttpResponseBuilder};
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;
use itertools::Itertools;
use std::{collections::BTreeMap, str::FromStr};

use crate::{
    api::{
        types::{
            EcdsaMetadata, EvmChainMetadata, ForwardEvmToIcpArg, ForwardingAccount,
            ForwardingResponse, ForwardingStatus, IcpAccount, IcpTokenMetadata, Metadata,
            SignedForwardingTx, TokenMetadata,
        },
        updates::is_allowed_relayer,
    },
    config::OperatingMode,
    dashboard::DashboardTemplate,
    event::EventType,
    evm::{
        self,
        forwarder::{self, ForwardingAddress},
        ledger::encode_icp_account,
        prover, read_evm_state,
        reader::TxLogId,
    },
    flow::{
        event::{Direction, TxId},
        state::FlowId,
    },
    icp,
    metrics::{
        self, canister_call_costs, canister_call_durations, canister_call_response_sizes,
        canister_call_results, encode_metrics,
    },
    numeric::{Amount, BlockNumber},
    state::{read_state, State},
    storage::{self, with_event_iter},
    task::{self, TaskType},
};

use super::{
    is_endpoint_paused,
    types::{
        CanisterCalls, Chain, EvmBlockStats, EvmChain, GetTransfersArg, RelayTask, Token, Transfer,
        TransferFee, TransferId, TransferStats,
    },
    Endpoint,
};

/// Returns the EVM address of the canister derived from its public tECDSA key.
pub fn get_evm_address() -> Option<String> {
    if is_endpoint_paused(Endpoint::GetEvmAddress) {
        return None;
    }
    let public_key = read_state(|s| s.icp.ecdsa_public_key.clone())?;
    Some(evm::derive_address_from_public_key(&public_key).to_string())
}

/// Returns an encoding of the given ICP account that can be used when locking
/// or burning tokens with EVM contracts.
/// # Parameters
/// - `arg`: An optional ICP account. If empty, the caller's principal is used.
pub fn get_evm_encoding(arg: Option<IcpAccount>) -> String {
    // This endpoint is cheap and cannot fail.
    // No need to support pausing it.
    let account = match arg {
        Some(account) => icp::IcpAccount::try_from(account).unwrap(),
        None => icp::IcpAccount::ICRC(IcrcAccount {
            owner: caller(),
            subaccount: None,
        }),
    };
    let encoded = encode_icp_account(account);
    let data1: [u8; 32] = encoded[0..32].try_into().unwrap();
    let data1 = Hex32::from(data1).to_string();
    let data2 = if encoded.len() > 32 {
        let data2: [u8; 32] = encoded[32..64].try_into().unwrap();
        Hex32::from(data2).to_string()
    } else {
        "".into()
    };
    data1 + &data2
}

/// Returns stats about fetched blocks of the given EVM chain.
pub fn get_evm_block_stats(chain: EvmChain) -> EvmBlockStats {
    read_state(|s| {
        let evm = s.evm.get(&chain).unwrap();
        let block_number_safe = evm.prover.head.safe.as_ref().map(|x| x.block_number);
        let fetch_time_safe = evm.prover.head.safe.as_ref().map(|x| x.fetch_time);
        let block_number_latest = evm.prover.head.latest.as_ref().map(|x| x.block_number);
        let fetch_time_latest = evm.prover.head.latest.as_ref().map(|x| x.fetch_time);
        EvmBlockStats {
            chain: Some(chain),
            block_number_safe: block_number_safe.map(|x| x.into_inner()),
            fetch_time_safe_ms: fetch_time_safe.map(|x| x.into_inner()),
            block_number_latest: block_number_latest.map(|x| x.into_inner()),
            fetch_time_latest_ms: fetch_time_latest.map(|x| x.into_inner()),
            block_time_ms: evm.prover.head.block_time_ms,
        }
    })
}

/// Returns detailed information about the transfer by its id.
/// # Parameters
/// - `arg`: a transfer id that was previously returned by the `transfer()`
///   endpoint.
pub fn get_transfer(arg: TransferId) -> Result<Transfer, String> {
    if is_endpoint_paused(Endpoint::GetTransfer) {
        return Err("get_transfer is tentatively paused.".to_string());
    }

    let id = FlowId::new(arg.id);
    match read_state(|s| s.flow.flow.get(&id).cloned()) {
        Some(flow) => Ok(flow.transfer_detailed()),
        None => {
            let invalid_flow = read_state(|s| s.flow.invalid_flow.get(&id).cloned())
                .ok_or("Transfer not found")?;
            Err(format!("Transfer failed: {:?}", invalid_flow.input.error))
        }
    }
}

/// Returns a list of transfers filtered by the given filter.
/// # Parameters
/// - `arg`: a filter argument. If no account is specified, it falls back to the latest transfers.
pub fn get_transfers(arg: GetTransfersArg) -> Result<Vec<Transfer>, String> {
    if is_endpoint_paused(Endpoint::GetTransfers) {
        return Err("get_transfers is tentatively paused.".to_string());
    }

    let icp_accounts: Vec<_> = arg
        .accounts
        .iter()
        .filter_map(|x| x.as_icp().ok())
        .map(icp::IcpAccount::try_from)
        .collect::<Result<_, String>>()?;

    let evm_accounts: Vec<_> = arg
        .accounts
        .iter()
        .filter_map(|x| x.as_evm().ok())
        .map(|x| Address::from_str(&x.address))
        .collect::<Result<_, String>>()?;

    if icp_accounts.len() > 2 {
        return Err("At most two ICP accounts are supported".to_string());
    }

    if evm_accounts.len() > 1 {
        return Err("At most one EVM account is supported".to_string());
    }

    let flows: Vec<_> = read_state(|s| {
        if !icp_accounts.is_empty() || !evm_accounts.is_empty() {
            let empty = vec![];
            let iter1 = if !icp_accounts.is_empty() {
                s.flow
                    .flow_by_icp_account
                    .get(&icp_accounts[0])
                    .unwrap_or(&empty)
                    .iter()
            } else {
                empty.iter()
            };

            let iter2 = if icp_accounts.len() > 1 {
                s.flow
                    .flow_by_icp_account
                    .get(&icp_accounts[1])
                    .unwrap_or(&empty)
                    .iter()
            } else {
                empty.iter()
            };

            let iter3 = if !evm_accounts.is_empty() {
                s.flow
                    .flow_by_evm_account
                    .get(&evm_accounts[0])
                    .unwrap_or(&empty)
                    .iter()
            } else {
                empty.iter()
            };

            iter1
                .rev()
                .merge_by(iter2.rev(), |a, b| a > b)
                .dedup()
                .merge_by(iter3.rev(), |a, b| a > b)
                .dedup()
                .skip(arg.skip as usize)
                .take(arg.count as usize)
                .filter_map(|id| s.flow.flow.get(id))
                .cloned()
                .collect()
        } else {
            s.flow
                .flow
                .values()
                .rev()
                .skip(arg.skip as usize)
                .take(arg.count as usize)
                .cloned()
                .collect()
        }
    });
    Ok(flows.into_iter().map(|x| x.transfer_light()).collect())
}

/// Returns a list of pending tasks for relayers.
pub fn get_relay_tasks(chain: EvmChain) -> Vec<RelayTask> {
    if is_endpoint_paused(Endpoint::GetForwardingAccounts) {
        return vec![];
    }
    prover::endpoint::get_relay_tasks(chain)
}

pub fn validate_forwarding_address(receiver: IcpAccount, address: String) -> Result<(), String> {
    if is_endpoint_paused(Endpoint::ValidateForwardingAddress) {
        return Err("validate_forwarding_address is tentatively paused.".to_string());
    }
    let expected = forwarder::endpoint::get_forwarding_address(receiver.try_into()?)?;
    if expected.to_string() == address {
        Ok(())
    } else {
        Err("the forwarding address does not match the receiver".into())
    }
}

pub fn get_forwarding_address(receiver: IcpAccount) -> Result<String, String> {
    fn is_canister(principal: &Principal) -> bool {
        // Heuristic: all canister ids have the same length.
        // We don't need this predicate to be 100% precise.
        principal != &Principal::anonymous()
            && principal.as_slice().len() == ic_cdk::id().as_slice().len()
    }

    if is_endpoint_paused(Endpoint::GetForwardingAddress) {
        return Err("get_forwarding_address is tentatively paused.".to_string());
    }

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) && !is_canister(&caller()) {
        return Err("This endpoint is not available".to_string());
    }

    let address = forwarder::endpoint::get_forwarding_address(receiver.try_into()?)?;
    Ok(address.to_string())
}

pub fn get_forwarding_accounts(chain: EvmChain, skip: u64, count: u64) -> Vec<ForwardingAccount> {
    if is_endpoint_paused(Endpoint::GetRelayTasks) {
        return vec![];
    }
    forwarder::endpoint::get_forwarding_accounts(chain, skip, count)
}

pub fn get_forwarding_transactions(chain: EvmChain) -> Vec<SignedForwardingTx> {
    if is_endpoint_paused(Endpoint::GetForwardingTransactions) {
        return vec![];
    }
    forwarder::endpoint::get_forwarding_transactions(chain)
}

pub fn get_forwarding_status(arg: ForwardEvmToIcpArg) -> Result<ForwardingResponse, String> {
    let chain = arg.chain;
    let fa = ForwardingAddress {
        token: arg.token,
        address: Address::from_str(&arg.address)?,
    };

    let Some(min_amount) = read_state(|s| {
        s.flow
            .config
            .get(&(Direction::EvmToIcp, fa.token, chain, fa.token))
            .map(|c| c.min_amount)
    }) else {
        return Err(format!(
            "transfers of {:?} are not supported from {:?}",
            fa.token, chain
        ));
    };

    let last_transfer = read_state(|s| {
        let flow_ids = s.flow.flow_by_evm_account.get(&fa.address)?;
        for flow_id in flow_ids.iter().rev().take(16) {
            if let Some(flow) = s.flow.flow.get(flow_id) {
                if flow.input.direction == Direction::EvmToIcp
                    && flow.input.evm_chain == chain
                    && flow.input.evm_token == fa.token
                {
                    return Some(*flow_id);
                }
            }
        }
        None
    });

    let status = read_evm_state(chain, |s| {
        if let Some(forwarded) = s.forwarder.forwarded.get(&fa) {
            if let Some(x) = forwarded.last() {
                return Some(ForwardingStatus::Forwarded(super::types::EvmTx {
                    hash: x.lock_or_burn_tx.to_string(),
                    log_index: None,
                }));
            }
        }

        if s.forwarder.signed.contains_key(&fa) || s.forwarder.signing_map.contains_key(&fa) {
            return Some(ForwardingStatus::Forwarding);
        }

        if let Some(balance) = s.forwarder.balance.get(&fa).cloned() {
            if balance < min_amount {
                return Some(ForwardingStatus::LowBalance {
                    balance: balance.into(),
                    min_amount: min_amount.into(),
                });
            }
        }

        if s.forwarder.unconfirmed_set.contains(&fa) {
            return Some(ForwardingStatus::CheckingBalance);
        }

        None
    });

    Ok(ForwardingResponse {
        done: last_transfer.map(|id| TransferId {
            id: id.into_inner(),
        }),
        status,
    })
}

/// Returns a list of events that were sent to the global state machine.
pub fn get_events_bin(count: u64, skip: u64) -> Result<Vec<Vec<u8>>, String> {
    if is_endpoint_paused(Endpoint::GetEvents) {
        return Err("get_events_bin is tentatively paused.".to_string());
    }

    with_event_iter(|iter| {
        iter.skip(skip as usize)
            .take(count as usize)
            .map(|e| {
                let mut buf = vec![];
                minicbor::encode(e, &mut buf).map_err(|err| err.to_string())?;
                Ok(buf)
            })
            .collect()
    })
}

/// Returns a list of events that were sent to the global state machine.
pub fn get_events(count: u64, skip: u64) -> Result<Vec<String>, String> {
    if is_endpoint_paused(Endpoint::GetEvents) {
        return Err("get_events is tentatively paused.".to_string());
    }

    Ok(with_event_iter(|iter| {
        iter.skip(skip as usize)
            .take(count as usize)
            .map(|e| format!("{}: {:?}", e.timestamp, e.event))
            .collect()
    }))
}

/// Returns telemetry information about the inter-canister calls.
pub fn get_canister_calls() -> Vec<CanisterCalls> {
    if is_endpoint_paused(Endpoint::GetCanisterCalls) {
        return vec![];
    }

    let canister_calls = metrics::canister_calls_by_endpoints();
    canister_calls
        .into_iter()
        .map(|((canister, method), calls)| CanisterCalls {
            canister,
            method,
            duration_in_ms: canister_call_durations(calls.iter())
                .into_iter()
                .map(|d| d.as_millis() as u64)
                .collect(),
            cost_in_cycles: canister_call_costs(calls.iter()),
            response_in_bytes: canister_call_response_sizes(calls.iter()),
            results: canister_call_results(calls.iter()),
        })
        .collect()
}

/// Returns the current ICP/Wei exchange rate.
pub fn get_wei_per_icp_rate() -> f64 {
    // This endpoint is cheap and cannot fail.
    // No need to support pausing it.
    read_state(|s| {
        s.icp
            .exchange_rate
            .get(&Token::ICP)
            .map(|x| x.eth_per_token.as_f64())
            .unwrap_or_default()
    })
}

fn get_ecdsa_metadata(state: &State) -> Option<EcdsaMetadata> {
    let public_key_pem = state.icp.ecdsa_public_key.as_ref()?.serialize_pem();
    let chain_code_hex = hex::encode(state.icp.chain_code.as_ref()?);
    Some(EcdsaMetadata {
        public_key_pem,
        chain_code_hex,
    })
}

fn get_token_metadata(state: &State) -> Vec<TokenMetadata> {
    let mut result = vec![];
    result.extend(state.icp.ledger.iter().map(|(token, s)| {
        TokenMetadata {
            token: Some(*token),
            chain: Some(Chain::ICP),
            contract: s.config.canister.to_text(),
            locker: None,
            topics: vec![],
            decimals: s.config.decimals,
            balance: s.balance.into(),
            queue_size: s.pending.len() as u64,
            wei_per_token: state
                .icp
                .exchange_rate
                .get(token)
                .map(|x| x.eth_per_token.as_f64())
                .unwrap_or_default(),
        }
    }));
    for evm in state.evm.values() {
        result.extend(evm.ledger.iter().map(|(token, s)| {
            TokenMetadata {
                token: Some(*token),
                chain: Some(Chain::from(evm.chain)),
                contract: s.config.erc20_address.to_string(),
                locker: match s.config.operating_mode {
                    OperatingMode::Minter => None,
                    OperatingMode::Locker => Some(s.config.logger_address.to_string()),
                },
                topics: s.config.logger_topics.iter().map(|x| x.to_vec()).collect(),
                decimals: s.config.decimals,
                balance: s.balance().into(),
                queue_size: s.pending.len() as u64,
                wei_per_token: state
                    .icp
                    .exchange_rate
                    .get(token)
                    .map(|x| x.eth_per_token.as_f64())
                    .unwrap_or_default(),
            }
        }));
    }
    result
}

fn get_evm_chain_metadata(state: &State) -> Vec<EvmChainMetadata> {
    let mut result = vec![];
    for evm in state.evm.values() {
        let block_number_safe = evm.prover.head.safe.as_ref().map(|x| x.block_number);
        let fetch_time_safe = evm.prover.head.safe.as_ref().map(|x| x.fetch_time);
        let block_number_latest = evm.prover.head.latest.as_ref().map(|x| x.block_number);
        let fetch_time_latest = evm.prover.head.latest.as_ref().map(|x| x.fetch_time);
        result.push(EvmChainMetadata {
            chain: Some(evm.chain),
            chain_id: evm.chain_id,
            nonce: evm.writer.next_nonce.into_inner(),
            block_number_safe: block_number_safe.map(|x| x.into_inner()),
            fetch_time_safe_ms: fetch_time_safe.map(|x| x.into_inner()),
            block_number_latest: block_number_latest.map(|x| x.into_inner()),
            fetch_time_latest_ms: fetch_time_latest.map(|x| x.into_inner()),
            block_time_ms: evm.prover.head.block_time_ms,
            max_fee_per_gas: evm
                .writer
                .latest_fee()
                .map(|x| x.max_fee_per_gas.into_inner())
                .unwrap_or_default() as u64,
            max_priority_fee_per_gas: evm
                .writer
                .latest_fee()
                .map(|x| x.max_priority_fee_per_gas.into_inner())
                .unwrap_or_default() as u64,
            max_fee_per_gas_average: evm
                .writer
                .average_fee()
                .map(|x| x.max_fee_per_gas.into_inner())
                .unwrap_or_default() as u64,
            max_priority_fee_per_gas_average: evm
                .writer
                .average_fee()
                .map(|x| x.max_priority_fee_per_gas.into_inner())
                .unwrap_or_default() as u64,
        });
    }
    result
}

pub fn get_metadata() -> Result<Metadata, String> {
    if is_endpoint_paused(Endpoint::GetMetadata) {
        return Err("get_metadata is tentatively paused.".to_string());
    }
    const WASM_PAGE_SIZE_IN_BYTES: u64 = 65536;

    read_state(|s| {
        Ok(Metadata {
            cycle_balance: ic_cdk::api::canister_balance128().into(),
            stable_memory_bytes: ic_cdk::api::stable::stable_size() * WASM_PAGE_SIZE_IN_BYTES,
            #[cfg(target_family = "wasm")]
            wasm_memory_bytes: (core::arch::wasm32::memory_size(0) as u64
                * WASM_PAGE_SIZE_IN_BYTES)
                .into(),
            #[cfg(not(target_family = "wasm"))]
            wasm_memory_bytes: 0,
            event_count: storage::total_event_count(),
            event_bytes: storage::total_event_size_in_bytes(),
            last_upgrade_time: s.last_upgrade_time.into_inner(),
            ecdsa: get_ecdsa_metadata(s),
            tokens: get_token_metadata(s),
            evm_chains: get_evm_chain_metadata(s),
        })
    })
}

pub fn get_icp_token_metadata() -> Vec<IcpTokenMetadata> {
    read_state(|state| {
        let mut result = vec![];
        result.extend(state.icp.ledger.iter().filter_map(
            |(token, s)| match s.config.operating_mode {
                OperatingMode::Minter => Some(IcpTokenMetadata {
                    token: Some(*token),
                    ledger: s.config.canister,
                    index: s.config.index_canister,
                }),
                OperatingMode::Locker => None,
            },
        ));
        result
    })
}

/// Returns a JSON structure with token balances grouped by token and chain.
/// Each token maps to a dictionary of chain -> balance.
fn get_metrics_json() -> BTreeMap<String, BTreeMap<String, String>> {
    read_state(|s| {
        let mut result: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

        for (token, ledger) in s.icp.ledger.iter() {
            result
                .entry(format!("{:?}", token))
                .or_default()
                .insert("icp".into(), ledger.balance.to_string());
        }

        for evm in s.evm.values() {
            let chain = format!("{:?}", evm.chain).to_lowercase();
            for (token, ledger) in evm.ledger.iter() {
                result
                    .entry(format!("{:?}", token))
                    .or_default()
                    .insert(chain.clone(), ledger.balance().to_string());
            }
        }

        result
    })
}

/// The HTTP endpoint of the canister that returns metrics, logs, and dashboard.
pub fn http_request(req: HttpRequest) -> HttpResponse {
    if is_endpoint_paused(Endpoint::HttpRequest) {
        return HttpResponseBuilder::not_found().build();
    }

    use crate::logs::{Log, Priority, Sort};
    use std::str::FromStr;

    if ic_cdk::api::data_certificate().is_none()
        && req.path() != "/metrics"
        && req.path() != "/api/balances"
        && req.path() != "/api/transactions"
    {
        ic_cdk::trap("update call rejected");
    }

    if req.path() == "/metrics" {
        let mut writer =
            ic_metrics_encoder::MetricsEncoder::new(vec![], ic_cdk::api::time() as i64 / 1_000_000);
        match encode_metrics(&mut writer) {
            Ok(()) => {
                return HttpResponseBuilder::ok()
                    .header("Content-Type", "text/plain; version=0.0.4")
                    .with_body_and_content_length(writer.into_inner())
                    .build()
            }
            Err(err) => {
                return HttpResponseBuilder::server_error(format!(
                    "Failed to encode metrics: {}",
                    err
                ))
                .build();
            }
        }
    } else if req.path() == "/logs" {
        let max_skip_timestamp = match req.raw_query_param("time") {
            Some(arg) => match u64::from_str(arg) {
                Ok(value) => value,
                Err(_) => {
                    return HttpResponseBuilder::bad_request()
                        .with_body_and_content_length("failed to parse the 'time' parameter")
                        .build();
                }
            },
            None => 0,
        };

        let mut log: Log = Default::default();

        match req.raw_query_param("priority") {
            Some(priority_str) => match Priority::from_str(priority_str) {
                Ok(priority) => match priority {
                    Priority::Error => log.push_logs(Priority::Error),
                    Priority::Info => log.push_logs(Priority::Info),
                    Priority::Debug => log.push_logs(Priority::Debug),
                    Priority::Http => log.push_logs(Priority::Http),
                },
                Err(_) => log.push_all(),
            },
            None => log.push_all(),
        }

        log.entries
            .retain(|entry| entry.timestamp >= max_skip_timestamp);

        fn ordering_from_query_params(sort: Option<&str>) -> Sort {
            match sort {
                Some(ord_str) => match Sort::from_str(ord_str) {
                    Ok(order) => order,
                    Err(_) => Sort::Descending,
                },
                None => Sort::Descending,
            }
        }

        log.sort_logs(ordering_from_query_params(req.raw_query_param("sort")));

        const MAX_BODY_SIZE: usize = 3_000_000;
        return HttpResponseBuilder::ok()
            .header("Content-Type", "application/json; charset=utf-8")
            .with_body_and_content_length(log.pretty(MAX_BODY_SIZE))
            .build();
    } else if req.path() == "/dashboard" {
        use askama::Template;

        let dashboard = read_state(DashboardTemplate::from_state);
        return HttpResponseBuilder::ok()
            .header("Content-Type", "text/html; charset=utf-8")
            .with_body_and_content_length(dashboard.render().unwrap())
            .build();
    } else if req.path() == "/api/balances" {
        let metrics = get_metrics_json();
        match serde_json::to_string(&metrics) {
            Ok(json) => {
                return HttpResponseBuilder::ok()
                    .header("Content-Type", "application/json; charset=utf-8")
                    .with_body_and_content_length(json)
                    .build();
            }
            Err(err) => {
                return HttpResponseBuilder::server_error(format!(
                    "Failed to serialize metrics: {}",
                    err
                ))
                .build();
            }
        }
    } else if req.path() == "/api/transactions" {
        // Parse 'from' and 'to' query parameters (Unix timestamps in seconds)
        let from_ts = match req.raw_query_param("from") {
            Some(arg) => match u64::from_str(arg) {
                Ok(value) => value,
                Err(_) => {
                    return HttpResponseBuilder::bad_request()
                        .with_body_and_content_length("failed to parse the 'from' parameter")
                        .build();
                }
            },
            None => {
                return HttpResponseBuilder::bad_request()
                    .with_body_and_content_length("missing required 'from' parameter")
                    .build();
            }
        };

        let to_ts = match req.raw_query_param("to") {
            Some(arg) => match u64::from_str(arg) {
                Ok(value) => value,
                Err(_) => {
                    return HttpResponseBuilder::bad_request()
                        .with_body_and_content_length("failed to parse the 'to' parameter")
                        .build();
                }
            },
            None => {
                return HttpResponseBuilder::bad_request()
                    .with_body_and_content_length("missing required 'to' parameter")
                    .build();
            }
        };

        if from_ts > to_ts {
            return HttpResponseBuilder::bad_request()
                .with_body_and_content_length("'from' must be less than or equal to 'to'")
                .build();
        }

        let response = get_transactions(from_ts, to_ts);
        match serde_json::to_string(&response) {
            Ok(json) => {
                return HttpResponseBuilder::ok()
                    .header("Content-Type", "application/json; charset=utf-8")
                    .with_body_and_content_length(json)
                    .build();
            }
            Err(err) => {
                return HttpResponseBuilder::server_error(format!(
                    "Failed to serialize transactions: {}",
                    err
                ))
                .build();
            }
        }
    }

    HttpResponseBuilder::not_found().build()
}

/// Returns a list of the currently paused tasks.
pub fn get_paused_tasks() -> Vec<TaskType> {
    // Endpoints related to pausing are not paused.

    task::paused_tasks()
}

/// Returns a list of the currently paused endpoints.
pub fn get_paused_endpoints() -> Vec<Endpoint> {
    // Endpoints related to pausing are not paused.

    read_state(|s| s.paused_endpoints.iter().cloned().collect())
}

fn get_transfer_fee(
    s: &State,
    icp_token: Token,
    evm_chain: EvmChain,
    evm_token: Token,
) -> Option<(Amount, Amount)> {
    let ledger_fee = s
        .icp
        .ledger
        .get(&icp_token)
        .map(|s| s.config.transfer_fee)?;

    let exchange_rate = s.icp.exchange_rate.get(&icp_token)?;

    let latest_fee = s.evm.get(&evm_chain)?.writer.latest_fee()?;
    let average_fee = s.evm.get(&evm_chain)?.writer.average_fee()?;
    let fee_margin = s.evm.get(&evm_chain)?.writer.config.tx_fee_margin;

    let gas_limit = s
        .evm
        .get(&evm_chain)?
        .ledger
        .get(&evm_token)?
        .config
        .gas_limit_for_unlock_or_mint;
    let latest_cost = latest_fee.cost(gas_limit, fee_margin);
    let average_cost = average_fee.cost(gas_limit, fee_margin);

    let latest = exchange_rate.eth_to_token(latest_cost)?;
    let average = exchange_rate.eth_to_token(average_cost)?;

    Some((
        latest.checked_add(ledger_fee)?,
        average.checked_add(ledger_fee)?,
    ))
}

pub fn get_transfer_fees() -> Vec<TransferFee> {
    let mut result = vec![];

    read_state(|s| {
        for ((direction, icp_token, evm_chain, evm_token), config) in s.flow.config.iter() {
            match direction {
                Direction::IcpToEvm => {
                    if let Some((latest, average)) =
                        get_transfer_fee(s, *icp_token, *evm_chain, *evm_token)
                    {
                        let available = s
                            .evm
                            .get(evm_chain)
                            .and_then(|s| s.ledger.get(evm_token))
                            .and_then(|s| s.available());
                        result.push(TransferFee {
                            source_chain: Some(Chain::ICP),
                            source_token: Some(*icp_token),
                            destination_chain: Some((*evm_chain).into()),
                            destination_token: Some(*evm_token),
                            min_amount: config.min_amount.into(),
                            max_amount: config.max_amount.into(),
                            available: available.map(|x| x.into()),
                            latest_transfer_fee_in_tokens: latest.into(),
                            average_transfer_fee_in_tokens: average.into(),
                            protocol_fee_in_percent: config.fee.as_f64(),
                        });
                    }
                }

                Direction::EvmToIcp => {
                    if let Some(transfer_fee) =
                        s.icp.ledger.get(icp_token).map(|s| s.config.transfer_fee)
                    {
                        result.push(TransferFee {
                            source_chain: Some((*evm_chain).into()),
                            source_token: Some(*evm_token),
                            destination_chain: Some(Chain::ICP),
                            destination_token: Some(*icp_token),
                            min_amount: config.min_amount.into(),
                            max_amount: config.max_amount.into(),
                            available: None,
                            latest_transfer_fee_in_tokens: transfer_fee.into(),
                            average_transfer_fee_in_tokens: transfer_fee.into(),
                            protocol_fee_in_percent: config.fee.as_f64(),
                        });
                    }
                }
            }
        }
    });

    result
}

pub fn get_transfer_stats(count: u64) -> Vec<TransferStats> {
    let mut stats: Vec<_> = read_state(|s| {
        s.flow
            .flow
            .values()
            .rev()
            .take(count as usize)
            .filter_map(|f| {
                let t = f.transfer_detailed();
                let duration_ms = t.duration_ms()?;
                Some(TransferStats {
                    source_chain: t.source.chain,
                    source_token: t.source.token,
                    destination_chain: t.destination.chain,
                    destination_token: t.destination.token,
                    count: 1,
                    duration_ms_avg: duration_ms,
                    duration_ms_max: duration_ms,
                })
            })
            .collect()
    });

    stats.sort_by_key(|s| {
        (
            s.source_chain,
            s.destination_chain,
            s.source_token,
            s.destination_token,
        )
    });

    stats
        .chunk_by(|a, b| {
            a.source_chain == b.source_chain
                && a.destination_chain == b.destination_chain
                && a.source_token == b.source_token
                && a.destination_token == b.destination_token
        })
        .map(|g| TransferStats {
            source_chain: g[0].source_chain,
            source_token: g[0].source_token,
            destination_chain: g[0].destination_chain,
            destination_token: g[0].destination_token,
            count: g.len() as u64,
            duration_ms_avg: (g.iter().map(|x| x.duration_ms_avg).sum::<u64>() as f64
                / g.len() as f64) as u64,
            duration_ms_max: g
                .iter()
                .max_by_key(|x| x.duration_ms_max)
                .unwrap()
                .duration_ms_max,
        })
        .collect()
}

/// A transaction for the /api/transactions endpoint (DefiLlama format).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionInfo {
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: u64,
    pub token: String,
    pub amount: String,
    pub from_chain: String,
    pub to_chain: String,
    pub from: String,
    pub to: String,
}

/// Response for the /api/transactions endpoint.
#[derive(serde::Serialize)]
pub struct TransactionsResponse {
    pub transactions: Vec<TransactionInfo>,
}

/// Builds a map of TxLogId to BlockNumber from the event log.
/// This is needed because incoming EVM transaction block numbers are only
/// stored in the event log, not in the flow state.
fn build_tx_log_block_map() -> BTreeMap<TxLogId, BlockNumber> {
    with_event_iter(|iter| {
        let mut map = BTreeMap::new();
        for event in iter {
            if let EventType::Evm { event, .. } = event.event {
                if let evm::Event::Reader(evm::reader::Event::FetchedTxLog {
                    block_number,
                    tx_log_id,
                }) = event
                {
                    map.insert(tx_log_id, block_number);
                }
            }
        }
        map
    })
}

fn format_icp_account(account: &icp::IcpAccount) -> String {
    match account {
        icp::IcpAccount::ICRC(icrc) => {
            if let Some(subaccount) = &icrc.subaccount {
                format!("{}.{}", icrc.owner, hex::encode(subaccount))
            } else {
                icrc.owner.to_string()
            }
        }
        icp::IcpAccount::AccountId(account_id) => account_id.to_hex(),
    }
}

/// Returns transactions filtered by timestamp range (DefiLlama format).
/// from_ts and to_ts are Unix timestamps in seconds.
///
/// Transactions are sorted by timestamp ascending.
///
/// This endpoint provides transaction data in a flat format suitable for
/// DefiLlama integration, which differs from `get_transfers()` that returns
/// nested source/destination structure via Candid.
pub fn get_transactions(from_ts: u64, to_ts: u64) -> TransactionsResponse {
    // Convert seconds to milliseconds for comparison with internal timestamps
    let from_ms = from_ts.saturating_mul(1000);
    let to_ms = to_ts.saturating_mul(1000);

    // Build a map of TxLogId -> BlockNumber for incoming EVM transactions
    let tx_log_block_map = build_tx_log_block_map();

    let mut transactions = read_state(|s| {
        let mut txs = Vec::new();

        for (flow_id, flow) in s.flow.flow.iter() {
            // Get the flow start timestamp in milliseconds
            let start_ms = match flow.step[0].start {
                Some(ts) => ts.into_inner(),
                None => continue,
            };

            // Filter by time range
            if start_ms < from_ms || start_ms > to_ms {
                continue;
            }

            // Build transaction info based on direction
            match flow.input.direction {
                Direction::IcpToEvm => {
                    // For ICP→EVM, the EVM tx is the second step (mint/unlock)
                    let Some(evm_state) = s.evm.get(&flow.input.evm_chain) else {
                        continue;
                    };
                    let Some(tx_receipt) = evm_state.writer.done.get(flow_id) else {
                        continue;
                    };

                    txs.push(TransactionInfo {
                        tx_hash: tx_receipt.tx_hash.to_string(),
                        block_number: tx_receipt.block_number.into_inner(),
                        timestamp: start_ms / 1000,
                        token: format!("{:?}", flow.input.icp_token),
                        amount: flow.input.icp_amount.into_inner().to_string(),
                        from_chain: "ICP".to_string(),
                        to_chain: format!("{:?}", flow.input.evm_chain),
                        from: format_icp_account(&flow.input.icp_account),
                        to: flow.input.evm_account.to_string(),
                    });
                }
                Direction::EvmToIcp => {
                    // For EVM→ICP, the EVM tx is the first step (burn/lock)
                    let tx_log_id = match &flow.step[0].progress {
                        crate::flow::state::Progress::Succeeded(TxId::Evm(id)) => *id,
                        crate::flow::state::Progress::Failed {
                            tx: Some(TxId::Evm(id)),
                            ..
                        } => *id,
                        _ => continue,
                    };

                    let block_number = tx_log_block_map
                        .get(&tx_log_id)
                        .map(|b| b.into_inner())
                        .unwrap_or(0);

                    txs.push(TransactionInfo {
                        tx_hash: tx_log_id.tx_hash.to_string(),
                        block_number,
                        timestamp: start_ms / 1000,
                        token: format!("{:?}", flow.input.evm_token),
                        amount: flow.input.evm_amount.into_inner().to_string(),
                        from_chain: format!("{:?}", flow.input.evm_chain),
                        to_chain: "ICP".to_string(),
                        from: flow.input.evm_account.to_string(),
                        to: format_icp_account(&flow.input.icp_account),
                    });
                }
            }
        }

        txs
    });

    // Sort by timestamp ascending
    transactions.sort_by_key(|t| t.timestamp);

    TransactionsResponse { transactions }
}
