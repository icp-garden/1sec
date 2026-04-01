use std::time::Duration;

use candid::Principal;
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;

use crate::{
    api::types::Token,
    config::OperatingMode,
    flow::{event::Operation, state::FlowId},
    icp::{
        ledger::state::{Request, Status},
        IcpAccount,
    },
    numeric::{Amount, BlockIndex, Timestamp},
};

use super::{apply_event, Event, State};

fn new_state(operating_mode: OperatingMode) -> State {
    State::new(super::Config {
        token: Token::ICP,
        operating_mode,
        decimals: 8,
        canister: Principal::anonymous(),
        index_canister: None,
        supports_account_id: true,
        initial_balance: Amount::ZERO,
        fee_receiver: Principal::anonymous(),
        fee_threshold: Amount::ZERO,
        transfer_batch: 40,
        transfer_fee: match operating_mode {
            OperatingMode::Minter => Amount::ZERO,
            OperatingMode::Locker => Amount::new(1),
        },
        transfer_task_busy_delay: Duration::from_secs(1),
        transfer_task_idle_delay: Duration::from_secs(2),
        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
    })
}

fn tx(x: u8) -> BlockIndex {
    BlockIndex::new(x as u64)
}

fn account() -> IcpAccount {
    IcpAccount::ICRC(IcrcAccount {
        owner: Principal::anonymous(),
        subaccount: None,
    })
}

#[test]
fn test_mint_ok() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance, Amount::new(0));
    assert!(!state.pending.contains(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(0));
    assert!(state.pending.contains(&id));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Pending,
            created_at: time,
        }
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Mint,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Succeeded { tx: tx(1) },
            created_at: time,
        }
    );
}

#[test]
fn test_mint_err() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance, Amount::new(0));
    assert!(!state.pending.contains(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(0));
    assert!(state.pending.contains(&id));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Pending,
            created_at: time,
        }
    );

    apply_event(
        &mut state,
        Event::Failed {
            id,
            op: Operation::Mint,
            err: "fail".into(),
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(0));
    assert_eq!(state.fees, Amount::new(0));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Failed { err: "fail".into() },
            created_at: time,
        }
    );
}

#[test]
fn test_burn_ok() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Mint,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Burn,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert!(state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(2),
            op: Operation::Burn,
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(98));
    assert_eq!(state.fees, Amount::new(20));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Burn,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Succeeded { tx: tx(2) },
            created_at: time,
        }
    );
}

#[test]
fn test_burn_err() {
    let mut state = new_state(OperatingMode::Minter);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Mint,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Mint,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Burn,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert!(state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    apply_event(
        &mut state,
        Event::Failed {
            id,
            err: "fail".into(),
            op: Operation::Burn,
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Burn,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Failed { err: "fail".into() },
            created_at: time,
        }
    );
}

#[test]
fn test_lock_ok() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance, Amount::new(0));
    assert!(!state.pending.contains(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(0));
    assert!(state.pending.contains(&id));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Pending,
            created_at: time,
        }
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Burn,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Succeeded { tx: tx(1) },
            created_at: time,
        }
    );
}

#[test]
fn test_lock_err() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    assert_eq!(state.balance, Amount::new(0));
    assert!(!state.pending.contains(&id));

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(0));
    assert!(state.pending.contains(&id));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Pending,
            created_at: time,
        }
    );

    apply_event(
        &mut state,
        Event::Failed {
            id,
            err: "fail".into(),
            op: Operation::Lock,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(0));
    assert_eq!(state.fees, Amount::new(0));

    assert_eq!(
        state.lock_or_burn.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Failed { err: "fail".into() },
            created_at: time,
        }
    );
}

#[test]
fn test_unlock_ok() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Lock,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Unlock,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    assert!(state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(2),
            op: Operation::Unlock,
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(121 - 23));
    assert_eq!(state.fees, Amount::new(20));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Unlock,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Succeeded { tx: tx(2) },
            created_at: time,
        }
    );
}

#[test]
fn test_unlock_err() {
    let mut state = new_state(OperatingMode::Locker);
    let id = FlowId::ZERO;
    let time = Timestamp::ZERO;

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Lock,
            account: account(),
            amount: Amount::new(123),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Succeeded {
            id,
            tx: tx(1),
            op: Operation::Lock,
        },
        time,
    );

    assert!(!state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    let id = FlowId::new(1);

    apply_event(
        &mut state,
        Event::Started {
            id,
            op: Operation::Unlock,
            account: account(),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            amount: Amount::new(23),
        },
        time,
    );

    assert!(state.pending.contains(&id));
    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    apply_event(
        &mut state,
        Event::Failed {
            id,
            err: "fail".into(),
            op: Operation::Unlock,
        },
        time,
    );

    assert_eq!(state.balance, Amount::new(122));
    assert_eq!(state.fees, Amount::new(10));

    assert_eq!(
        state.unlock_or_mint.get(&id).unwrap(),
        &Request {
            id,
            op: Operation::Unlock,
            account: account(),
            amount: Amount::new(23),
            collected_fee: Amount::new(10),
            ledger_fee: Amount::new(1),
            status: Status::Failed { err: "fail".into() },
            created_at: time,
        }
    );
}
