use candid::Principal;
use itertools::Itertools;
use std::{collections::HashMap, time::Duration};

use crate::{
    api::types::{CanisterCallResult, Token},
    numeric::{Amount64, Timestamp},
    state::{mutate_state, read_state},
    storage,
    task::timestamp_ms,
};

const MAX_RECORDED_CALLS: usize = 1000;

pub type CanisterCallId = Amount64<CanisterCallTag>;
pub enum CanisterCallTag {}

#[derive(Debug, Clone)]
pub struct CanisterCallEnd {
    pub result: Result<(), String>,
    pub duration: Duration,
    pub cost_in_cycles: u64,
    pub response_size_in_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CanisterCallEntry {
    pub canister: Principal,
    pub method: String,
    pub start: Timestamp,
    pub attached_cycles: u64,
    pub end: Option<CanisterCallEnd>,
}

#[derive(Debug, Clone)]
pub struct CanisterCall {
    pub id: CanisterCallId,
    pub entry: Option<CanisterCallEntry>,
}

impl CanisterCall {
    pub fn new<T: AsRef<str>>(canister: Principal, method: T, attached_cycles: u64) -> Self {
        let id = next_call_id();

        let entry = CanisterCallEntry {
            canister,
            method: method.as_ref().into(),
            start: timestamp_ms(),
            attached_cycles,
            end: None,
        };

        write(id, entry.clone());

        Self {
            id,
            entry: Some(entry),
        }
    }

    pub fn returned_ok(mut self) {
        self.returned(Result::Ok(()));
    }

    pub fn returned_err<T: AsRef<str>>(mut self, err: T) {
        // Mask out digits to group similar errors.
        let err = err
            .as_ref()
            .chars()
            .take(64)
            .map(|c| if c.is_numeric() { '*' } else { c })
            .collect();
        self.returned(Result::Err(err));
    }

    fn returned(&mut self, result: Result<(), String>) {
        if let Some(mut entry) = self.entry.take() {
            // The system API does not allow calling `ic0::msg_cycles_refunded`
            // and `ic0::msg_arg_data_size()` in a cleanup callback context.
            // That is why it is impossible to compute the cycles cost and
            // response size.
            let cost_in_cycles = 0;
            let response_size_in_bytes = 0;
            entry.end = Some(CanisterCallEnd {
                duration: Duration::from_millis(
                    timestamp_ms()
                        .checked_sub(entry.start)
                        .unwrap_or(Timestamp::ZERO)
                        .into_inner(),
                ),
                result,
                cost_in_cycles,
                response_size_in_bytes,
            });
            write(self.id, entry);
        }
    }
}

impl Drop for CanisterCall {
    fn drop(&mut self) {
        self.returned(Result::Err("unknown".into()));
    }
}

#[derive(Default, Debug, Clone)]
pub struct CanisterCallStats {
    pub count: usize,
    pub total_cost_in_cycles: u64,
    pub total_duration_ms: u64,
    pub total_response_size_bytes: u64,
    pub max_duration_ms: u64,
}

impl CanisterCallStats {
    pub fn add(&mut self, call: &CanisterCallEntry) {
        if let Some(end) = &call.end {
            self.count += 1;
            self.total_cost_in_cycles += end.cost_in_cycles;
            self.total_duration_ms += end.duration.as_millis() as u64;
            self.total_response_size_bytes += end.response_size_in_bytes;
            self.max_duration_ms = self.max_duration_ms.max(end.duration.as_millis() as u64);
        }
    }

    pub fn average_cost_in_cycles(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_cost_in_cycles as f64 / self.count as f64
        }
    }

    pub fn average_duration_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_duration_ms as f64 / self.count as f64
        }
    }

    pub fn average_response_size_in_bytes(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total_response_size_bytes as f64 / self.count as f64
        }
    }

    pub fn max_duration_ms(&self) -> f64 {
        self.max_duration_ms as f64
    }
}

pub fn encode_metrics(w: &mut ic_metrics_encoder::MetricsEncoder<Vec<u8>>) -> std::io::Result<()> {
    const WASM_PAGE_SIZE_IN_BYTES: f64 = 65536.0;

    read_state(|s| {
        w.encode_gauge(
            "cycle_balance",
            ic_cdk::api::canister_balance128() as f64,
            "Cycle balance.",
        )?;
        w.encode_gauge(
            "stable_memory_bytes",
            ic_cdk::api::stable::stable_size() as f64 * WASM_PAGE_SIZE_IN_BYTES,
            "Size of the stable memory allocated by this canister.",
        )?;
        #[cfg(target_family = "wasm")]
        w.encode_gauge(
            "wasm_memory_bytes",
            core::arch::wasm32::memory_size(0) as f64 * WASM_PAGE_SIZE_IN_BYTES,
            "Size of the Wasm memory allocated by this canister.",
        )?;
        w.encode_gauge(
            "total_event_count",
            storage::total_event_count() as f64,
            "The total number of events in the state machine",
        )?;
        w.encode_gauge(
            "total_event_bytes",
            storage::total_event_size_in_bytes() as f64,
            "The total size of events in the state machine",
        )?;
        w.encode_gauge(
            "last_upgrade_time",
            read_state(|s| s.last_upgrade_time.into_inner() as f64),
            "The last upgrade time",
        )?;
        w.encode_gauge(
            "eth_per_icp",
            s.icp
                .exchange_rate
                .get(&Token::ICP)
                .map(|x| x.eth_per_token.as_f64())
                .unwrap_or_default(),
            "Fetched exchange rate of ETH per ICP.",
        )?;
        w.encode_gauge(
            "eth_per_usdc",
            s.icp
                .exchange_rate
                .get(&Token::USDC)
                .map(|x| x.eth_per_token.as_f64())
                .unwrap_or_default(),
            "Fetched exchange rate of ETH per USDC.",
        )?;
        w.encode_gauge(
            "eth_per_usdt",
            s.icp
                .exchange_rate
                .get(&Token::USDT)
                .map(|x| x.eth_per_token.as_f64())
                .unwrap_or_default(),
            "Fetched exchange rate of ETH per USDT.",
        )?;
        w.encode_gauge(
            "eth_per_btc",
            s.icp
                .exchange_rate
                .get(&Token::cbBTC)
                .map(|x| x.eth_per_token.as_f64())
                .unwrap_or_default(),
            "Fetched exchange rate of ETH per BTC.",
        )?;
        for state in s.evm.values() {
            w.encode_gauge(
                &format!("nonce_{}", state.chain_id),
                state.writer.next_nonce.as_f64(),
                "Transaction nonce.",
            )?;
            w.encode_gauge(
                &format!("max_fee_per_gas_fetched_{}", state.chain_id),
                state
                    .writer
                    .fetched_fee
                    .as_ref()
                    .map(|x| x.fee.max_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Fetched max fee per gas.",
            )?;
            w.encode_gauge(
                &format!("max_priority_fee_per_gas_fetched_{}", state.chain_id),
                state
                    .writer
                    .fetched_fee
                    .as_ref()
                    .map(|x| x.fee.max_priority_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Fetched max priority fee per gas.",
            )?;
            w.encode_gauge(
                &format!("max_fee_per_gas_relayed_{}", state.chain_id),
                state
                    .writer
                    .relayed_fee
                    .as_ref()
                    .map(|x| x.fee.max_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Relayed max fee per gas.",
            )?;
            w.encode_gauge(
                &format!("max_priority_fee_per_gas_relayed_{}", state.chain_id),
                state
                    .writer
                    .relayed_fee
                    .as_ref()
                    .map(|x| x.fee.max_priority_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Relayed max priority fee per gas.",
            )?;
            w.encode_gauge(
                &format!("max_fee_per_gas_average_{}", state.chain_id),
                state
                    .writer
                    .average_fee()
                    .as_ref()
                    .map(|x| x.max_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Average max fee per gas.",
            )?;
            w.encode_gauge(
                &format!("max_priority_fee_per_gas_average_{}", state.chain_id),
                state
                    .writer
                    .average_fee()
                    .as_ref()
                    .map(|x| x.max_priority_fee_per_gas.as_f64())
                    .unwrap_or_default(),
                "Average max priority fee per gas.",
            )?;
            w.encode_gauge(
                &format!("latest_block_number_{}", state.chain_id),
                state
                    .prover
                    .head
                    .latest
                    .as_ref()
                    .map(|x| x.block_number.into_inner())
                    .unwrap_or_default() as f64,
                "Latest block number",
            )?;
            w.encode_gauge(
                &format!("latest_fetch_time_{}", state.chain_id),
                state
                    .prover
                    .head
                    .latest
                    .as_ref()
                    .map(|x| x.fetch_time.into_inner())
                    .unwrap_or_default() as f64,
                "Latest fetch time",
            )?;
            w.encode_gauge(
                &format!("safe_block_number_{}", state.chain_id),
                state
                    .prover
                    .head
                    .safe
                    .as_ref()
                    .map(|x| x.block_number.into_inner())
                    .unwrap_or_default() as f64,
                "Safe block number",
            )?;
            w.encode_gauge(
                &format!("safe_fetch_time_{}", state.chain_id),
                state
                    .prover
                    .head
                    .safe
                    .as_ref()
                    .map(|x| x.fetch_time.into_inner())
                    .unwrap_or_default() as f64,
                "Safe fetch time",
            )?;
            w.encode_gauge(
                &format!("block_time_ms_{}", state.chain_id),
                state.prover.head.block_time_ms as f64,
                "Inferred block time in milliseconds",
            )?;
        }

        for ledger in s.icp.ledger.values() {
            let name: String = format!("ledger_ICP_{:?}", ledger.config.token);
            w.encode_gauge(
                &format!("{}_balance", name),
                ledger.balance.into_inner() as f64,
                "The balance of the ledger",
            )?;
        }

        for evm in s.evm.values() {
            w.encode_gauge(
                &format!("{:?}_forwarder_unconfirmed_count", evm.chain),
                evm.forwarder.unconfirmed_queue.len() as f64,
                "The number of items in the unconfirmed queue of the forwarder",
            )?;

            w.encode_gauge(
                &format!("{:?}_forwarder_signing_count", evm.chain),
                evm.forwarder.signing_queue.len() as f64,
                "The number of items in the signing queue of the forwarder",
            )?;

            for ledger in evm.ledger.values() {
                let name: String = format!("ledger_{:?}_{:?}", evm.chain, ledger.config.token);
                w.encode_gauge(
                    &format!("{}_balance", name),
                    ledger.balance().into_inner() as f64,
                    "The balance of the ledger",
                )?;

                w.encode_gauge(
                    &format!("{}_positive_balance", name),
                    ledger.positive_balance.into_inner() as f64,
                    "The positive balance of the ledger",
                )?;

                w.encode_gauge(
                    &format!("{}_negative_balance", name),
                    ledger.negative_balance.into_inner() as f64,
                    "The positive balance of the ledger",
                )?;

                w.encode_gauge(
                    &format!("{}_pending_balance_sub", name),
                    ledger.pending_balance_sub.into_inner() as f64,
                    "The pending negative delta of the ledger balance",
                )?;

                w.encode_gauge(
                    &format!("{}_pending_balance_add", name),
                    ledger.pending_balance_add.into_inner() as f64,
                    "The pending positive delta of the ledger balance",
                )?;

                w.encode_gauge(
                    &format!("{}_pending_balance_sub", name),
                    ledger.pending_balance_sub.into_inner() as f64,
                    "The pending negative delta of the ledger balance",
                )?;
            }
        }

        for ((canister, method), stats) in read_state(|s| s.canister_call_stats.clone()).iter() {
            let name: String = format!("canister_call_{}_{}", canister, method)
                .replace('-', "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || c == &'_')
                .collect();

            w.encode_gauge(
                &format!("{}_count", name),
                stats.count as f64,
                "Total number of calls",
            )?;

            w.encode_gauge(
                &format!("{}_cost_cycles_total", name),
                stats.total_cost_in_cycles as f64,
                "Total cost of the canister call in cycles",
            )?;

            w.encode_gauge(
                &format!("{}_cost_cycles", name),
                stats.average_cost_in_cycles(),
                "Average cost of the canister call in cycles",
            )?;

            w.encode_gauge(
                &format!("{}_duration_ms", name),
                stats.average_duration_ms(),
                "Average duration of the canister call in ms",
            )?;

            w.encode_gauge(
                &format!("{}_duration_ms_max", name),
                stats.max_duration_ms(),
                "Max duration of the canister call in ms",
            )?;

            w.encode_gauge(
                &format!("{}_response_bytes", name),
                stats.average_response_size_in_bytes(),
                "Average response size of the canister call in bytes",
            )?;
        }

        for ((canister, method), calls) in canister_calls_by_endpoints() {
            let results = canister_call_results(calls.iter());

            let name: String = format!("canister_call_{}_{}", canister, method)
                .replace('-', "_")
                .chars()
                .filter(|c| c.is_alphanumeric() || c == &'_')
                .collect();

            for (index, result) in results.into_iter().enumerate() {
                let mut label: String = result
                    .label
                    .to_ascii_lowercase()
                    .replace(" ", "_")
                    .chars()
                    .filter(|c| c.is_alphanumeric() || c == &'_')
                    .take(32)
                    .collect();

                if result.label.len() > 16 {
                    // Guarantee uniqueness of the label even if it was truncated.
                    label = format!("{}_{}", label, index);
                }

                w.encode_gauge(
                    &format!("{}_result_{}", name, label),
                    result.count as f64,
                    &format!("Number of canister calls with the result: {}", result.label),
                )?;
            }
        }
        Ok(())
    })
}

fn next_call_id() -> CanisterCallId {
    mutate_state(|s| {
        let id = s.next_canister_call_id;
        s.next_canister_call_id = s
            .next_canister_call_id
            .increment("BUG: overflow in next_canister_id++");
        id
    })
}

fn write(id: CanisterCallId, call: CanisterCallEntry) {
    mutate_state(move |s| {
        s.canister_call_stats
            .entry((call.canister, call.method.clone()))
            .or_default()
            .add(&call);
        if s.canister_calls.len() < MAX_RECORDED_CALLS {
            s.canister_calls.push(call);
        } else {
            let index = (id.into_inner() as usize) % MAX_RECORDED_CALLS;
            s.canister_calls[index] = call;
        }
    })
}

pub fn canister_calls_by_endpoints() -> Vec<((Principal, String), Vec<CanisterCallEntry>)> {
    let mut calls = read_state(|s| s.canister_calls.clone());
    calls.sort_by_key(|x| (x.canister, x.method.clone(), x.start));
    calls
        .into_iter()
        .chunk_by(|x| (x.canister, x.method.clone()))
        .into_iter()
        .map(|(k, v)| (k, v.collect()))
        .collect()
}

pub fn canister_call_durations<'a>(
    iter: impl Iterator<Item = &'a CanisterCallEntry>,
) -> Vec<Duration> {
    let mut result: Vec<_> = iter
        .filter_map(|i| i.end.as_ref().map(|end| end.duration))
        .collect();
    result.sort();
    result
}

pub fn canister_call_costs<'a>(iter: impl Iterator<Item = &'a CanisterCallEntry>) -> Vec<u64> {
    let mut result: Vec<_> = iter
        .filter_map(|i| i.end.as_ref().map(|end| end.cost_in_cycles))
        .collect();
    result.sort();
    result
}

pub fn canister_call_response_sizes<'a>(
    iter: impl Iterator<Item = &'a CanisterCallEntry>,
) -> Vec<u64> {
    let mut result: Vec<_> = iter
        .filter_map(|i| i.end.as_ref().map(|end| end.response_size_in_bytes))
        .collect();
    result.sort();
    result
}

pub fn canister_call_results<'a>(
    iter: impl Iterator<Item = &'a CanisterCallEntry>,
) -> Vec<CanisterCallResult> {
    let mut table: HashMap<String, u64> = HashMap::new();
    iter.filter_map(|i| {
        i.end.as_ref().map(|end| match &end.result {
            Ok(_) => "OK".into(),
            Err(err) => err.clone(),
        })
    })
    .for_each(|x| {
        *table.entry(x).or_default() += 1;
    });
    let mut result: Vec<_> = table
        .into_iter()
        .map(|(label, count)| CanisterCallResult { label, count })
        .collect();

    result.sort_by_key(|x| std::cmp::Reverse(x.count));

    result
}
