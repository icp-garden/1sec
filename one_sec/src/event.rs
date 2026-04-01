//! This module defines the events of the global state machine.
use minicbor::{Decode, Encode};

use crate::{
    api::types::EvmChain,
    config::Config,
    evm::{self},
    flow, icp,
    numeric::Timestamp,
    state::{mutate_state, InitInput, State, UpgradeInput},
    storage::{record_event, with_event_iter},
    task::timestamp_ms,
};

/// An event of the global state machine.
///
/// Events are persistent and stored in an append-only log in the stable memory.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum EventType {
    /// This event is sent on canister init.
    #[n(0)]
    Init(#[n(0)] InitInput),

    /// This event is sent on canister upgrade.
    #[n(1)]
    Upgrade(#[n(0)] UpgradeInput),

    /// An event of [an EVM state machine](evm).
    #[n(3)]
    Evm {
        #[n(0)]
        chain: EvmChain,
        #[n(1)]
        event: evm::Event,
    },

    /// An event of [the ICP state machine](icp).
    #[n(4)]
    Icp(#[n(0)] icp::Event),

    /// An event of [the flow state machine](flow).
    #[n(5)]
    Flow(#[n(0)] flow::Event),
}

/// An event with the timestamp.
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub struct RootEvent {
    #[n(0)]
    pub timestamp: Timestamp,
    #[n(1)]
    pub event: EventType,
}

/// Records the event in the stable memory and applies the corresponding state
/// transition if the global state machine.
pub fn process_event(event: EventType) {
    mutate_state(|s| {
        record_and_apply_event(s, event);
    })
}

/// Updates the state to reflect the given state transition.
fn apply_event(state: &mut State, event: EventType, timestamp: Timestamp) {
    match event {
        EventType::Init(InitInput { .. }) => {
            panic!("state re-initialization is not allowed");
        }
        EventType::Upgrade(input) => {
            state.record_upgrade(input);
        }
        EventType::Evm { chain, event } => {
            let evm = state
                .evm
                .get_mut(&chain)
                .unwrap_or_else(|| unreachable!("BUG: cannot find evm state: {:?}", chain));
            evm::apply_event(evm, event, timestamp);
        }
        EventType::Icp(event) => icp::apply_event(&mut state.icp, event, timestamp),

        EventType::Flow(event) => {
            flow::apply_event(state, event, timestamp);
        }
    }
}

/// Records the given event payload in the event log and updates the state to
/// reflect the change.
fn record_and_apply_event(state: &mut State, event: EventType) {
    let timestamp = timestamp_ms();
    let root_event = RootEvent { timestamp, event };
    record_event(&root_event);
    apply_event(state, root_event.event, root_event.timestamp);
}

/// Recomputes the state from the event log.
///
/// # Panics
///
/// This function panics if:
///   * The event log is empty.
///   * The first event in the log is not an Init event.
///   * One of the events in the log invalidates the state invariants.
pub fn replay_events(config: Config) -> State {
    with_event_iter(|mut iter| {
        let Some(RootEvent {
            event: EventType::Init(init_arg),
            ..
        }) = iter.next()
        else {
            panic!("BUG: event log doesn't start with Init");
        };
        let mut state = State::new(init_arg, config);
        for event in iter {
            apply_event(&mut state, event.event, event.timestamp);
        }
        state
    })
}
