//! This module defines the state of the global state machine.
use candid::Principal;
use ic_ethereum_types::Address;
use minicbor::{Decode, Encode};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use crate::{
    api::{
        types::{Deployment, EvmChain, Token},
        Endpoint,
    },
    config::Config,
    evm, flow, icp,
    metrics::{CanisterCallEntry, CanisterCallId, CanisterCallStats},
    numeric::{Amount, BlockNumber, Timestamp, TxNonce},
    task::{timestamp_ms, TaskType},
};

thread_local! {
    static __STATE: RefCell<Option<State>> = RefCell::default();
}

/// The state of the global state machine.
#[derive(Debug)]
pub struct State {
    /// The state of the ICP state machine.
    pub icp: icp::State,

    /// The states of the EVM state machines.
    pub evm: BTreeMap<EvmChain, evm::State>,

    /// The state of the flow state machine.
    pub flow: flow::state::State,

    /// This is used by some update endpoints to limit the number of concurrent
    /// executions per principal and in total.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub principal_guards: BTreeSet<Principal>,

    /// This set keeps track of all tasks that are currently running.
    /// It is used to ensure that there is at most one instance of a task
    /// running at any given moment.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub active_tasks: BTreeSet<TaskType>,

    /// The set of tasks that were paused by a controller of the canister.
    /// Paused tasks skip execution when scheduled.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub paused_tasks: BTreeSet<TaskType>,

    /// The set of endpoints that were paused by a controller of the canister.
    /// Paused endpoints skip execution when called.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub paused_endpoints: BTreeSet<Endpoint>,

    /// The last time a task was executed.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub last_task_execution: BTreeMap<TaskType, Timestamp>,

    /// Enabled debug logs.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub debug_tracing: bool,

    /// Telemetry information about cross-canister calls.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub next_canister_call_id: CanisterCallId,

    /// Telemetry information about cross-canister calls.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub canister_calls: Vec<CanisterCallEntry>,

    /// Telemetry information about cross-canister calls.
    /// Note: this state is ephemeral (cleared on upgrades).
    pub canister_call_stats: BTreeMap<(Principal, String), CanisterCallStats>,

    /// The timestamp of the last canister upgrade (or init).
    pub last_upgrade_time: Timestamp,
}

impl State {
    pub fn new(input: InitInput, mut config: Config) -> Self {
        // Override ICP balances in the config.
        let ledgers = input.icp.map(|x| x.ledger.clone()).unwrap_or_default();
        for ledger in ledgers {
            for ledger_config in config.icp.ledger.iter_mut() {
                if ledger.token != ledger_config.token {
                    continue;
                }
                if let Some(initial_balance) = ledger.initial_balance {
                    ledger_config.initial_balance = initial_balance;
                }
            }
        }

        // Override EVM addresses and balances in the config.
        for input in input.evm {
            for config in config.evm.iter_mut() {
                if input.chain != config.chain {
                    continue;
                }
                if let Some(initial_block) = input.initial_block {
                    config.reader.initial_block = Some(initial_block);
                }
                if let Some(initial_nonce) = input.initial_nonce {
                    config.writer.initial_nonce = TxNonce::new(initial_nonce);
                }
                for ledger in input.ledger.iter() {
                    for ledger_config in config.ledger.iter_mut() {
                        if ledger.token != ledger_config.token {
                            continue;
                        }
                        if let Some(erc20_address) = ledger.erc20_address {
                            ledger_config.erc20_address = erc20_address;
                        }

                        if let Some(logger_address) = ledger.logger_address {
                            ledger_config.logger_address = logger_address;
                        }

                        if let Some(initial_balance) = ledger.initial_balance {
                            ledger_config.initial_balance = initial_balance;
                        }
                    }
                }
            }
        }

        Self {
            icp: icp::State::new(config.icp),
            evm: config
                .evm
                .into_iter()
                .map(|c| (c.chain, evm::State::new(c)))
                .collect(),
            flow: flow::state::State::new(config.flow),
            principal_guards: Default::default(),
            active_tasks: Default::default(),
            paused_tasks: Default::default(),
            paused_endpoints: Default::default(),
            last_task_execution: Default::default(),
            next_canister_call_id: CanisterCallId::ZERO,
            canister_calls: vec![],
            canister_call_stats: Default::default(),
            debug_tracing: true,
            last_upgrade_time: timestamp_ms(),
        }
    }

    pub fn record_upgrade(&mut self, input: UpgradeInput) {
        assert_matches!(input, UpgradeInput { deployment: _ })
    }
}

/// Overrides EVM ledger parameters.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct InitEvmTokenInput {
    /// The token.
    #[n(0)]
    pub token: Token,
    /// The new ERC20 contract address for the token.
    #[n(1)]
    pub erc20_address: Option<Address>,
    /// The new address of the contract that emits log events.
    /// Note: tokens that are minted have `logger_address` equal to
    /// `erc20_address`.
    #[n(2)]
    pub logger_address: Option<Address>,
    /// The initial balance of the ledger.
    #[n(3)]
    pub initial_balance: Option<Amount>,
}

/// Overrides ICP ledger parameters.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct InitIcpTokenInput {
    /// The token.
    #[n(0)]
    pub token: Token,
    /// The initial balance of the ledger.
    #[n(1)]
    pub initial_balance: Option<Amount>,
}

/// Overrides configuration parameters of the EVM state machine after canister init.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct InitEvmInput {
    #[n(0)]
    pub chain: EvmChain,
    #[n(1)]
    pub initial_nonce: Option<u64>,
    #[n(2)]
    pub initial_block: Option<BlockNumber>,
    #[n(3)]
    pub ledger: Vec<InitEvmTokenInput>,
}

/// Overrides configuration parameters of the ICP state machine after canister init.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct InitIcpInput {
    #[n(0)]
    pub ledger: Vec<InitIcpTokenInput>,
}

/// Overrides configuration parameters of the global state machine after
/// canister init.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct InitInput {
    #[n(0)]
    pub deployment: Deployment,
    #[n(1)]
    pub icp: Option<InitIcpInput>,
    #[n(2)]
    pub evm: Vec<InitEvmInput>,
}

/// Input for canister upgrade.
#[derive(Encode, Decode, PartialEq, Eq, Clone, Debug)]
pub struct UpgradeInput {
    #[n(0)]
    pub deployment: Deployment,
}

/// Mutates (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn mutate_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut State) -> R,
{
    __STATE.with(|s| f(s.borrow_mut().as_mut().expect("BUG: empty state")))
}

/// Read (part of) the current state using `f`.
///
/// Panics if there is no state.
pub fn read_state<F, R>(f: F) -> R
where
    F: FnOnce(&State) -> R,
{
    __STATE.with(|s| f(s.borrow().as_ref().expect("BUG: empty state")))
}

/// Replaces the current state.
pub fn replace_state(state: State) {
    __STATE.with(|s| {
        *s.borrow_mut() = Some(state);
    });
}
