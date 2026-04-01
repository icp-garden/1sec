use ic_ethereum_types::Address;

use crate::{
    api::types::Token,
    config::OperatingMode,
    evm::{
        ledger::state::{Request, Status},
        reader::TxLogId,
        TxHash,
    },
    flow::{event::Operation, state::FlowId},
    numeric::{Amount, GasAmount, Timestamp, TxLogIndex, Wei},
};

use super::{apply_event, Event, State};

fn new_state(operating_mode: OperatingMode) -> State {
    State::new(super::Config {
        token: Token::ICP,
        operating_mode,
        decimals: 8,
        erc20_address: Address::new([1; 20]),
        max_tx_cost: Wei::new(1_000_000),
        gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
        gas_limit_for_lock_or_burn: GasAmount::new(100_000),
        gas_limit_for_approve: GasAmount::new(100_000),
        logger_address: Address::new([2; 20]),
        logger_topics: [[3; 32]; 4],
        initial_balance: Amount::ZERO,
    })
}

fn tx(x: u8) -> TxLogId {
    TxLogId {
        tx_hash: TxHash([x; 32]),
        index: TxLogIndex::ZERO,
    }
}

#[test]
fn test_mint_ok() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance(), Amount::new(0));
    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(123));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Pending,
        }
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Succeeded { tx: tx(1) },
        }
    );
}

#[test]
fn test_mint_err() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance(), Amount::new(0));
    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(123));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Pending,
        }
    );

    apply_event(
        &mut state,
        Event::Failed {
            id,
            tx: tx(1),
            err: "fail".into(),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(0));
    assert_eq!(state.pending_balance_add, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Failed {
                tx: tx(1),
                err: "fail".into()
            },
        }
    );
}

#[test]
fn test_burn_ok() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(23));

    apply_event(&mut state, Event::Succeeded { id, tx: tx(2) }, time);

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
            status: Status::Succeeded { tx: tx(2) },
        }
    );
}

#[test]
fn test_concurrent_mint_and_burn_ok() {
    let mut state = new_state(OperatingMode::Minter);
    let id0 = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert!(!state.requests.contains_key(&id0));

    apply_event(
        &mut state,
        Event::Started {
            id: id0,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(123));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(23));

    apply_event(&mut state, Event::Succeeded { id, tx: tx(2) }, time);

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
            status: Status::Succeeded { tx: tx(2) },
        }
    );

    apply_event(&mut state, Event::Succeeded { id: id0, tx: tx(1) }, time);
    assert_eq!(state.pending_balance_add, Amount::new(0));

    assert_eq!(
        state.requests.get(&id0).unwrap(),
        &Request {
            id: id0,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Succeeded { tx: tx(1) },
        }
    );
}

#[test]
fn test_burn_err() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(23));

    apply_event(
        &mut state,
        Event::Failed {
            id,
            tx: tx(2),
            err: "fail".into(),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_sub, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Burn,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
            status: Status::Failed {
                tx: tx(2),
                err: "fail".into(),
            },
        }
    );
}

#[test]
fn test_lock_ok() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance(), Amount::new(0));
    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(123));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Pending,
        }
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Succeeded { tx: tx(1) },
        }
    );
}

#[test]
fn test_lock_err() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance(), Amount::new(0));
    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_add, Amount::new(123));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Pending,
        }
    );

    apply_event(
        &mut state,
        Event::Failed {
            id,
            tx: tx(1),
            err: "fail".into(),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(0));
    assert_eq!(state.pending_balance_add, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
            status: Status::Failed {
                tx: tx(1),
                err: "fail".into()
            },
        }
    );
}

#[test]
fn test_unlock_ok() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Unlock,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(23));

    apply_event(&mut state, Event::Succeeded { id, tx: tx(2) }, time);

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Unlock,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
            status: Status::Succeeded { tx: tx(2) },
        }
    );
}

#[test]
fn test_unlock_err() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert!(!state.requests.contains_key(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: Address::new([4; 20]),
            amount: Amount::new(123),
        },
        time,
    );

    apply_event(&mut state, Event::Succeeded { id, tx: tx(1) }, time);

    assert_eq!(state.balance(), Amount::new(123));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Unlock,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(100));
    assert_eq!(state.pending_balance_sub, Amount::new(23));

    apply_event(
        &mut state,
        Event::Failed {
            id,
            tx: tx(2),
            err: "fail".into(),
        },
        time,
    );

    assert_eq!(state.balance(), Amount::new(123));
    assert_eq!(state.pending_balance_sub, Amount::new(0));

    assert_eq!(
        state.requests.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Unlock,
            account: Address::new([4; 20]),
            amount: Amount::new(23),
            status: Status::Failed {
                tx: tx(2),
                err: "fail".into(),
            },
        }
    );
}
