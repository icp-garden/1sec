use ic_stable_structures::{
    log::Log as StableLog,
    memory_manager::{MemoryId, MemoryManager, VirtualMemory},
    storable::{Bound, Storable},
    DefaultMemoryImpl,
};
use std::borrow::Cow;
use std::cell::RefCell;

use crate::event::RootEvent;

const LOG_INDEX_MEMORY_ID: MemoryId = MemoryId::new(0);
const LOG_DATA_MEMORY_ID: MemoryId = MemoryId::new(1);

type VMem = VirtualMemory<DefaultMemoryImpl>;
type EventLog = StableLog<RootEvent, VMem, VMem>;

impl Storable for RootEvent {
    fn to_bytes(&self) -> Cow<[u8]> {
        let mut buf = vec![];
        minicbor::encode(self, &mut buf).expect("BUG: failed to encode event");
        Cow::Owned(buf)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        minicbor::decode(bytes.as_ref())
            .unwrap_or_else(|e| panic!("BUG: failed to decode event {}: {e}", hex::encode(bytes)))
    }

    const BOUND: Bound = Bound::Unbounded;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static EVENTS: RefCell<EventLog> = MEMORY_MANAGER
        .with(|m|
              RefCell::new(
                  StableLog::init(
                      m.borrow().get(LOG_INDEX_MEMORY_ID),
                      m.borrow().get(LOG_DATA_MEMORY_ID)
                  ).expect("BUG: failed to initialize stable log")
              )
        );

}

/// Appends the event to the event log.
pub fn record_event(event: &RootEvent) {
    EVENTS
        .with(|events| events.borrow().append(event))
        .expect("BUG: failed to record event");
}

/// Returns the total number of events in the audit log.
pub fn total_event_count() -> u64 {
    EVENTS.with(|events| events.borrow().len())
}

/// Returns the total size of events in the audit log.
pub fn total_event_size_in_bytes() -> u64 {
    EVENTS.with(|events| events.borrow().log_size_bytes())
}

pub fn with_event_iter<F, R>(f: F) -> R
where
    F: for<'a> FnOnce(Box<dyn Iterator<Item = RootEvent> + 'a>) -> R,
{
    EVENTS.with(|events| f(Box::new(events.borrow().iter())))
}

#[cfg(feature = "dev")]
pub fn replace_events(events: &[RootEvent]) -> Result<(), String> {
    MEMORY_MANAGER.with(|m| {
        EVENTS.replace(StableLog::new(
            m.borrow().get(LOG_INDEX_MEMORY_ID),
            m.borrow().get(LOG_DATA_MEMORY_ID),
        ));
    });
    for event in events {
        record_event(event);
    }

    let restored_events: Vec<_> = with_event_iter(|iter| iter.collect());

    if events.len() != restored_events.len() {
        return Err(format!(
            "mismatch in event count: {} vs {}",
            events.len(),
            restored_events.len()
        ));
    }

    for i in 0..events.len() {
        if events[i] != restored_events[i] {
            return Err(format!(
                "mismatch in event {}:\n{:?}----\n{:?}\n",
                i, events[i], restored_events[i],
            ));
        }
    }
    Ok(())
}
