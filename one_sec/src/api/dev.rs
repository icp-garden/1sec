//! The update endpoints of the canister.
use std::cell::RefCell;

use ic_cdk::{api::is_controller, caller};

use crate::{
    event::RootEvent,
    storage::{self},
};

thread_local! {
    static __UPLOADED_EVENTS: RefCell<Vec<RootEvent>> = RefCell::default();
}

pub fn upload_events(events: Vec<Vec<u8>>) -> Result<(), String> {
    if !is_controller(&caller()) {
        return Err("Only a controller can call this endpoint".into());
    }
    __UPLOADED_EVENTS.with(|s| {
        let mut uploaded_events = s.borrow_mut();
        for buf in events.iter() {
            let event: RootEvent = minicbor::decode(buf).map_err(|err| err.to_string())?;
            uploaded_events.push(event);
        }
        Ok::<(), String>(())
    })?;
    Ok(())
}

pub fn replace_events() -> Result<(), String> {
    if !is_controller(&caller()) {
        return Err("Only a controller can call this endpoint".into());
    }
    let events = __UPLOADED_EVENTS.take();
    storage::replace_events(&events)
}
