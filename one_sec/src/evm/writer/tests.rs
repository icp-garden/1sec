use std::time::Duration;

use ic_ethereum_types::Address;

use crate::{
    api::types::Token,
    evm::{
        tx::{TxReceipt, TxStatus},
        TxFee, TxHash,
    },
    flow::{event::Operation, state::FlowId},
    numeric::{BlockNumber, GasAmount, Percent, Timestamp, TxNonce, Wei, WeiPerGas},
};

use super::{apply_event, Config, Event, State, TxInput};

fn new_state() -> State {
    State::new(Config {
        initial_nonce: TxNonce::ZERO,
        initial_fee_estimate: TxFee {
            max_fee_per_gas: WeiPerGas::new(10),
            max_priority_fee_per_gas: WeiPerGas::new(10),
        },
        tx_fee_bump: Percent::from_percent(10),
        tx_fee_margin: Percent::from_percent(20),
        tx_sign_batch: 4,
        tx_resubmit_delay: Duration::from_secs(1),
        tx_resend_delay: Duration::from_secs(1),
        tx_sign_to_send_delay: Duration::from_secs(1),
        tx_sign_to_poll_delay: Duration::from_secs(1),
        tx_send_to_poll_delay: Duration::from_secs(1),
        fetch_fee_estimate_delay: Duration::from_secs(1),
    })
}

#[test]
fn test_start() {
    let time = Timestamp::ZERO;
    let id = FlowId::new(1);
    let token = Token::ICP;
    let tx_input = TxInput {
        contract: Address::new([1; 20]),
        calldata: vec![],
        gas_limit: GasAmount::new(100_000),
        cost_limit: Wei::new(100_000),
    };

    let mut state = new_state();

    let nonce = state.next_nonce;

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Mint,
            tx_input: tx_input.clone(),
        },
        time,
    );

    assert_eq!(state.pending.get(&id).unwrap().id, id);
    assert_eq!(state.pending.get(&id).unwrap().nonce, nonce);
    assert_eq!(state.pending.get(&id).unwrap().op, Operation::Mint);
    assert_eq!(state.pending.get(&id).unwrap().tx_input, tx_input);
    assert_eq!(state.pending.get(&id).unwrap().token, token);
    assert!(state.pending.get(&id).unwrap().signed.is_empty());
    assert!(state.pending.get(&id).unwrap().sending.is_empty());

    assert_eq!(state.next_nonce, nonce.increment(""));
    assert!(state.done.is_empty());
    assert!(state.done_tx.is_empty());
}

#[test]
fn test_finish() {
    let time = Timestamp::ZERO;
    let id = FlowId::new(1);
    let token = Token::ICP;
    let tx_input = TxInput {
        contract: Address::new([1; 20]),
        calldata: vec![],
        gas_limit: GasAmount::new(100_000),
        cost_limit: Wei::new(100_000),
    };

    let mut state = new_state();

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Mint,
            tx_input: tx_input.clone(),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::SignedTx {
            id,
            tx_hash: TxHash([1; 32]),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Finished {
            id,
            tx_receipt: TxReceipt {
                tx_hash: TxHash([1; 32]),
                status: TxStatus::Success,
                block_number: BlockNumber::new(1),
            },
        },
        time,
    );

    assert!(state.pending.is_empty());
    assert_eq!(
        state.done.get(&id).unwrap().block_number,
        BlockNumber::new(1)
    );
    assert_eq!(state.done.get(&id).unwrap().status, TxStatus::Success);
    assert_eq!(state.done.get(&id).unwrap().tx_hash, TxHash([1; 32]));
    assert!(state.done_tx.contains(&TxHash([1; 32])));
}

#[should_panic]
#[test]
fn test_duplicate_start() {
    let time = Timestamp::ZERO;
    let id = FlowId::new(1);
    let token = Token::ICP;
    let tx_input = TxInput {
        contract: Address::new([1; 20]),
        calldata: vec![],
        gas_limit: GasAmount::new(100_000),
        cost_limit: Wei::new(100_000),
    };

    let mut state = new_state();

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Mint,
            tx_input: tx_input.clone(),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Unlock,
            tx_input: tx_input.clone(),
        },
        time,
    );
}

#[should_panic]
#[test]
fn test_duplicate_receipt() {
    let time = Timestamp::ZERO;
    let id = FlowId::new(1);
    let token = Token::ICP;
    let tx_input = TxInput {
        contract: Address::new([1; 20]),
        calldata: vec![],
        gas_limit: GasAmount::new(100_000),
        cost_limit: Wei::new(100_000),
    };

    let mut state = new_state();

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Mint,
            tx_input: tx_input.clone(),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::SignedTx {
            id,
            tx_hash: TxHash([1; 32]),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Finished {
            id,
            tx_receipt: TxReceipt {
                tx_hash: TxHash([1; 32]),
                status: TxStatus::Success,
                block_number: BlockNumber::new(1),
            },
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Finished {
            id,
            tx_receipt: TxReceipt {
                tx_hash: TxHash([1; 32]),
                status: TxStatus::Failure,
                block_number: BlockNumber::new(1),
            },
        },
        time,
    );
}

#[should_panic]
#[test]
fn test_finish_unknown_hash() {
    let time = Timestamp::ZERO;
    let id = FlowId::new(1);
    let token = Token::ICP;
    let tx_input = TxInput {
        contract: Address::new([1; 20]),
        calldata: vec![],
        gas_limit: GasAmount::new(100_000),
        cost_limit: Wei::new(100_000),
    };

    let mut state = new_state();

    apply_event(
        &mut state,
        Event::Started {
            id,
            token,
            op: Operation::Mint,
            tx_input: tx_input.clone(),
        },
        time,
    );

    apply_event(
        &mut state,
        Event::Finished {
            id,
            tx_receipt: TxReceipt {
                tx_hash: TxHash([1; 32]),
                status: TxStatus::Success,
                block_number: BlockNumber::new(1),
            },
        },
        time,
    );
}
