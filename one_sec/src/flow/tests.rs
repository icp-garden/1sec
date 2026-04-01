use candid::Principal;
use ic_ethereum_types::Address;
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;

use crate::{
    api::types::{Chain, Deployment, EvmChain, Token},
    evm::{reader::TxLogId, TxHash},
    flow::{
        event::{InvalidInput, Operation, TxId},
        state::{InvalidFlow, Progress, Step},
    },
    icp::IcpAccount,
    numeric::{Amount, BlockIndex, Percent, Timestamp, TxLogIndex},
    state::InitInput,
};

use super::{
    apply_event,
    event::{Direction, Input},
    Event,
};

fn new_state() -> crate::state::State {
    let mut config = crate::config::Config::test();

    for flow in config.flow.flows.iter_mut() {
        flow.min_amount = Amount::new(10_000);
        flow.max_amount = Amount::new(1_000_000);
        flow.fee = Percent::from_permille(1);
    }

    crate::state::State::new(
        InitInput {
            deployment: Deployment::Local,
            icp: None,
            evm: vec![],
        },
        config,
    )
}

fn account() -> IcpAccount {
    IcpAccount::ICRC(IcrcAccount {
        owner: Principal::anonymous(),
        subaccount: None,
    })
}

fn tx(x: u8) -> TxLogId {
    TxLogId {
        tx_hash: TxHash([x; 32]),
        index: TxLogIndex::ZERO,
    }
}

#[test]
fn test_input() {
    let time = Timestamp::ZERO;
    let mut state = new_state();

    let input = Input {
        direction: Direction::IcpToEvm,
        icp_account: account(),
        icp_token: Token::ICP,
        icp_amount: Amount::new(100),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::ICP,
        evm_amount: Amount::new(99),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    let input = Input {
        direction: Direction::EvmToIcp,
        icp_account: account(),
        icp_token: Token::ICP,
        icp_amount: Amount::new(100),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::ICP,
        evm_amount: Amount::new(99),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    assert_eq!(state.flow.next_flow_id, id.increment(""));

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::Base,
            op: Operation::Burn,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Unlock,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    let input = Input {
        direction: Direction::EvmToIcp,
        icp_account: account(),
        icp_token: Token::USDC,
        icp_amount: Amount::new(99),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::USDC,
        evm_amount: Amount::new(100),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::Base,
            op: Operation::Lock,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    state
        .evm
        .get_mut(&EvmChain::Base)
        .unwrap()
        .ledger
        .get_mut(&Token::USDC)
        .unwrap()
        .positive_balance = Amount::new(100);

    let input = Input {
        direction: Direction::IcpToEvm,
        icp_account: account(),
        icp_token: Token::USDC,
        icp_amount: Amount::new(100),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::USDC,
        evm_amount: Amount::new(99),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Burn,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Unlock,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );
}

#[test]
fn test_flow_ok() {
    let time = Timestamp::ZERO;
    let mut state = new_state();

    let input = Input {
        direction: Direction::IcpToEvm,
        icp_account: account(),
        icp_token: Token::ICP,
        icp_amount: Amount::new(100),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::ICP,
        evm_amount: Amount::new(99),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    apply_event(
        &mut state,
        Event::StartedStep {
            id,
            chain: Chain::ICP,
            op: Operation::Lock,
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Running,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    apply_event(
        &mut state,
        Event::SucceededStep {
            id,
            chain: Chain::ICP,
            op: Operation::Lock,
            tx: TxId::Icp(BlockIndex::new(1)),
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Succeeded(TxId::Icp(BlockIndex::new(1))),
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    apply_event(
        &mut state,
        Event::StartedStep {
            id,
            chain: Chain::Base,
            op: Operation::Mint,
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Succeeded(TxId::Icp(BlockIndex::new(1))),
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Running,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    assert!(state.flow.pending.contains(&id));

    apply_event(
        &mut state,
        Event::SucceededStep {
            id,
            chain: Chain::Base,
            op: Operation::Mint,
            tx: TxId::Evm(tx(1)),
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Lock,
            progress: Progress::Succeeded(TxId::Icp(BlockIndex::new(1))),
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::Base,
            op: Operation::Mint,
            progress: Progress::Succeeded(TxId::Evm(tx(1))),
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    assert!(state.flow.pending.is_empty());
}

#[test]
fn test_flow_err() {
    let time = Timestamp::ZERO;
    let mut state = new_state();

    let input = Input {
        direction: Direction::EvmToIcp,
        icp_account: account(),
        icp_token: Token::USDC,
        icp_amount: Amount::new(99),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::USDC,
        evm_amount: Amount::new(100),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::Base,
            op: Operation::Lock,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    apply_event(
        &mut state,
        Event::StartedStep {
            id,
            chain: Chain::Base,
            op: Operation::Lock,
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::Base,
            op: Operation::Lock,
            progress: Progress::Running,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    apply_event(
        &mut state,
        Event::FailedStep {
            id,
            chain: Chain::Base,
            op: Operation::Lock,
            tx: Some(TxId::Evm(tx(1))),
            err: "fail".into(),
        },
        time,
    );

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::Base,
            op: Operation::Lock,
            progress: Progress::Failed {
                tx: Some(TxId::Evm(tx(1))),
                err: "fail".into(),
            },
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );

    assert!(state.flow.pending.is_empty());
}

#[test]
fn test_invalid_input() {
    let time = Timestamp::ZERO;
    let mut state = new_state();

    let input = InvalidInput {
        tx_log_id: tx(1),
        evm_chain: EvmChain::Base,
        evm_token: Token::ICP,
        error: "fail".into(),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::InvalidInput(input.clone()), time);

    assert!(state.flow.pending.is_empty());
    assert!(state.flow.flow.is_empty());
    assert_eq!(
        state.flow.invalid_flow.get(&id).unwrap(),
        &InvalidFlow { input }
    );
    assert_eq!(state.flow.next_flow_id, id.increment(""));
}

#[test]
fn test_refund() {
    let time = Timestamp::ZERO;
    let mut state = new_state();

    let input = Input {
        direction: Direction::IcpToEvm,
        icp_account: account(),
        icp_token: Token::USDC,
        icp_amount: Amount::new(100),
        evm_chain: EvmChain::Base,
        evm_account: Address::new([1; 20]),
        evm_token: Token::USDC,
        evm_amount: Amount::new(99),
    };

    let id = state.flow.next_flow_id;

    apply_event(&mut state, Event::Input(input.clone()), time);

    let flow = state.flow.flow.get(&id).unwrap();

    assert_eq!(flow.id, id);
    assert_eq!(flow.input, input);
    assert_eq!(
        flow.step[0],
        Step {
            chain: Chain::ICP,
            op: Operation::Burn,
            progress: Progress::Planned,
            start: flow.step[0].start,
            end: flow.step[0].end,
        }
    );
    assert_eq!(
        flow.step[1],
        Step {
            chain: Chain::ICP,
            op: Operation::Mint,
            progress: Progress::Planned,
            start: flow.step[1].start,
            end: flow.step[1].end,
        }
    );
}
