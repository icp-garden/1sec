use std::time::Duration;

use crate::{
    evm::TxHash,
    numeric::{BlockNumber, Timestamp, TxLogIndex},
};

use super::{apply_event, Config, Event, State, TxLogId};

fn log(x: u8) -> TxLogId {
    TxLogId {
        tx_hash: TxHash([x; 32]),
        index: TxLogIndex::new(x as u64),
    }
}

fn new_state() -> State {
    State::new(Config {
        initial_block: None,
        num_blocks_to_fetch_initially: 0,
        max_num_blocks_to_fetch_per_call: 100,
        fetch_tx_logs_task_delay: Duration::from_secs(1),
    })
}

#[test]
fn test_fetch_ok() {
    let time = Timestamp::ZERO;

    let mut state = new_state();

    assert!(state.last_fully_fetched_block.is_none());
    assert!(!state.done.contains(&log(1)));
    assert!(!state.done.contains(&log(2)));

    apply_event(
        &mut state,
        Event::FetchedTxLog {
            block_number: BlockNumber::new(1),
            tx_log_id: log(1),
        },
        time,
    );

    assert!(state.last_fully_fetched_block.is_none());
    assert!(state.done.contains(&log(1)));

    apply_event(
        &mut state,
        Event::FetchedTxLog {
            block_number: BlockNumber::new(1),
            tx_log_id: log(2),
        },
        time,
    );

    assert!(state.done.contains(&log(2)));

    apply_event(&mut state, Event::FetchedBlock(BlockNumber::new(2)), time);

    assert_eq!(state.last_fully_fetched_block, Some(BlockNumber::new(2)));

    apply_event(&mut state, Event::FetchedBlock(BlockNumber::new(4)), time);

    assert_eq!(state.last_fully_fetched_block, Some(BlockNumber::new(4)));
}

#[should_panic]
#[test]
fn test_fetch_duplicate() {
    let time = Timestamp::ZERO;

    let mut state = new_state();

    apply_event(
        &mut state,
        Event::FetchedTxLog {
            block_number: BlockNumber::new(1),
            tx_log_id: log(1),
        },
        time,
    );
    apply_event(
        &mut state,
        Event::FetchedTxLog {
            block_number: BlockNumber::new(1),
            tx_log_id: log(1),
        },
        time,
    );
}

#[should_panic]
#[test]
fn test_fetch_wrong_block() {
    let time = Timestamp::ZERO;

    let mut state = new_state();

    apply_event(&mut state, Event::FetchedBlock(BlockNumber::new(2)), time);

    apply_event(
        &mut state,
        Event::FetchedTxLog {
            block_number: BlockNumber::new(1),
            tx_log_id: log(1),
        },
        time,
    );
}
