//! This module defines events of the EVM state machine.
use minicbor::{Decode, Encode};

use crate::{api::types::Token, numeric::Timestamp};

use super::{ledger, reader, writer, State};

/// An event of the EVM state machine.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    // An event of the reader state machine.
    #[n(0)]
    Reader(#[n(0)] reader::Event),
    // An event of the writer state machine.
    #[n(1)]
    Writer(#[n(0)] writer::Event),
    // An event of a ledger state machine.
    #[n(2)]
    Ledger {
        #[n(0)]
        token: Token,
        #[n(1)]
        event: ledger::Event,
    },
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut State, event: Event, time: Timestamp) {
    match event {
        Event::Reader(event) => reader::apply_event(&mut state.reader, event, time),
        Event::Writer(event) => writer::apply_event(&mut state.writer, event, time),
        Event::Ledger { token, event } => {
            let ledger = state
                .ledger
                .get_mut(&token)
                .unwrap_or_else(|| unreachable!("BUG: cannot find ledger state for {:?}", token));
            ledger::apply_event(ledger, event, time)
        }
    }
}
