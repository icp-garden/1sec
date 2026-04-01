//! This module defines events of the ICP state machine.
use minicbor::{Decode, Encode};

use crate::{api::types::Token, numeric::Timestamp};

use super::{ledger, State};

/// An event of the ICP state machine.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    /// An event of the ledger state machine.
    #[n(0)]
    Ledger {
        #[n(1)]
        token: Token,
        #[n(2)]
        event: ledger::Event,
    },
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, time: Timestamp) {
    match event {
        Event::Ledger { token, event } => {
            let ledger = state
                .ledger
                .get_mut(&token)
                .unwrap_or_else(|| unreachable!("BUG: cannot find ledger state for {:?}", token));
            ledger::apply_event(ledger, event, time);
        }
    }
}
