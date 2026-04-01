//! This module defines events of the EVM state machine.
use ic_canister_log::log;
use ic_ethereum_types::Address;
use minicbor::{Decode, Encode};

use crate::{
    api::types::{Chain, EvmChain, Token},
    event::EventType,
    evm::{self, reader::TxLogId},
    flow::state::{Flow, InvalidFlow, Progress},
    icp::IcpAccount,
    logs::DEBUG,
    numeric::{Amount, BlockIndex, Timestamp},
};

use super::state::{FlowId, State, Step};

/// The direction of a bridging transfer.
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    #[n(0)]
    IcpToEvm,
    #[n(1)]
    EvmToIcp,
}

/// The bridging transfer input provided by the user.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct Input {
    #[n(0)]
    pub direction: Direction,
    #[n(1)]
    pub icp_account: IcpAccount,
    #[n(2)]
    pub icp_token: Token,
    #[n(3)]
    pub icp_amount: Amount,
    #[n(4)]
    pub evm_chain: EvmChain,
    #[n(5)]
    pub evm_account: Address,
    #[n(6)]
    pub evm_token: Token,
    #[n(7)]
    pub evm_amount: Amount,
}

impl Input {
    pub fn fee(&self) -> Amount {
        match self.direction {
            Direction::IcpToEvm => self
                .icp_amount
                .checked_sub(self.evm_amount)
                .unwrap_or_default(),
            Direction::EvmToIcp => self
                .evm_amount
                .checked_sub(self.icp_amount)
                .unwrap_or_default(),
        }
    }
}

/// Some details of an invalid bridging transfer.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct InvalidInput {
    #[n(0)]
    pub tx_log_id: TxLogId,
    #[n(1)]
    pub evm_chain: EvmChain,
    #[n(2)]
    pub evm_token: Token,
    #[n(3)]
    pub error: String,
}

/// An event of the flow state machine.
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Event {
    /// A new bridging transfer request has been received.
    #[n(0)]
    Input(#[n(0)] Input),

    /// A new invalid bridging transfer has been received.
    #[n(1)]
    InvalidInput(#[n(0)] InvalidInput),

    /// Started executed a new step of a flow.
    #[n(2)]
    StartedStep {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        chain: Chain,
        #[n(2)]
        op: Operation,
    },

    /// A step of a flow has succeeded.
    #[n(4)]
    SucceededStep {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        chain: Chain,
        #[n(2)]
        op: Operation,
        #[n(3)]
        tx: TxId,
    },

    /// A step of a flow has failed.
    #[n(5)]
    FailedStep {
        #[n(0)]
        id: FlowId,
        #[n(1)]
        chain: Chain,
        #[n(2)]
        op: Operation,
        #[n(3)]
        tx: Option<TxId>,
        #[n(4)]
        err: String,
    },
}

impl Event {
    pub fn wrap(self) -> EventType {
        EventType::Flow(self)
    }
}

/// The operation performed by a step.
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, Eq)]
pub enum Operation {
    #[n(0)]
    Lock,
    #[n(1)]
    Unlock,
    #[n(2)]
    Burn,
    #[n(3)]
    Mint,
}

/// The id of the transaction performed by a step.
#[derive(Clone, Copy, Debug, Encode, Decode, PartialEq, Eq)]
pub enum TxId {
    #[n(0)]
    Icp(#[n(0)] BlockIndex),
    #[n(1)]
    Evm(#[n(0)] TxLogId),
}

impl TxId {
    fn matches(&self, chain: Chain) -> bool {
        match self {
            TxId::Icp(_) => chain == Chain::ICP,
            TxId::Evm(_) => EvmChain::try_from(chain).is_ok(),
        }
    }
}

/// Updates the state to reflect the given state transition.
pub fn apply_event(state: &mut crate::state::State, event: Event, time: Timestamp) {
    match event {
        Event::Input(input) => on_input(
            &mut state.flow,
            state.evm.get_mut(&input.evm_chain).unwrap_or_else(|| {
                unreachable!(
                    "BUG: couldn't find the EVM state for {:?} when applying flow event",
                    input.evm_chain
                )
            }),
            input,
        ),
        Event::InvalidInput(invalid_input) => {
            on_invalid_input(&mut state.flow, invalid_input);
        }
        Event::StartedStep { id, chain, op } => on_started(&mut state.flow, id, chain, op, time),
        Event::SucceededStep { id, chain, op, tx } => {
            finish_step(
                &mut state.flow,
                id,
                chain,
                op,
                Some(tx),
                Progress::Succeeded(tx),
                time,
            );
        }
        Event::FailedStep {
            id,
            chain,
            op,
            tx,
            err,
        } => {
            finish_step(
                &mut state.flow,
                id,
                chain,
                op,
                tx,
                Progress::Failed { tx, err },
                time,
            );
        }
    }
}

/// Plans execution steps for a flow based on the user input.
pub fn prepare_steps(input: &Input) -> [Step; 2] {
    let evm_chain: Chain = input.evm_chain.into();
    match (input.icp_token, input.direction) {
        (Token::ICP, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::ICP);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::ICP, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::ICP);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::CHAT, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::CHAT);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::CHAT, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::CHAT);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::GLDT, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::GLDT);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::GLDT, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::GLDT);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::USDC, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::USDC);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::USDC, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::USDC);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::USDT, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::USDT);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::USDT, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::USDT);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::cbBTC, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::cbBTC);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::cbBTC, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::cbBTC);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::ckBTC, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::ckBTC);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::ckBTC, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::ckBTC);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::BOB, Direction::IcpToEvm) => {
            assert_eq!(input.evm_token, Token::BOB);
            [
                Step {
                    chain: Chain::ICP,
                    op: Operation::Lock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: evm_chain,
                    op: Operation::Mint,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
        (Token::BOB, Direction::EvmToIcp) => {
            assert_eq!(input.evm_token, Token::BOB);
            [
                Step {
                    chain: evm_chain,
                    op: Operation::Burn,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
                Step {
                    chain: Chain::ICP,
                    op: Operation::Unlock,
                    progress: Progress::Planned,
                    start: None,
                    end: None,
                },
            ]
        }
    }
}

fn on_input(state: &mut State, evm_state: &mut evm::State, input: Input) {
    let id = state.next_flow_id;
    state.next_flow_id = state
        .next_flow_id
        .increment("BUG: overflow in next_flow_id++");

    state
        .flow_by_icp_account
        .entry(input.icp_account)
        .or_default()
        .push(id);

    state
        .flow_by_evm_account
        .entry(input.evm_account)
        .or_default()
        .push(id);

    let mut step = prepare_steps(&input);

    if input.direction == Direction::IcpToEvm && step[1].op == Operation::Unlock {
        assert!(step[1].chain != Chain::ICP);
        let available = evm_state
            .ledger
            .get(&input.evm_token)
            .unwrap_or_else(|| {
                unreachable!(
                    "BUG: could find EVM ledger for {:?}/{:?}",
                    input.evm_chain, input.evm_token
                )
            })
            .available()
            .unwrap_or_else(|| {
                unreachable!(
                    "BUG: overflow when computing available balance for {:?}/{:?}",
                    input.evm_chain, input.evm_token
                );
            });

        if input.evm_amount > available {
            // There is not enough EVM token available on the destination chain,
            // so we need to refund the ICP token.
            assert_eq!(step[0].chain, Chain::ICP);
            assert_eq!(step[0].op, Operation::Burn);
            step[1] = Step {
                chain: Chain::ICP,
                op: Operation::Mint,
                progress: Progress::Planned,
                start: None,
                end: None,
            };
        }
    }

    let flow = Flow { id, input, step };

    log!(
        DEBUG,
        "[{:?}]: flow {}: {:?} {} {:?} {}",
        flow.input.evm_chain,
        id,
        flow.input.direction,
        flow.input.icp_amount,
        flow.input.icp_token,
        flow.input.evm_account,
    );

    let overwritten = state.flow.insert(id, flow);
    assert!(overwritten.is_none(), "BUG: duplicate flow: {:?}", id);

    let success = state.pending.insert(id);
    assert!(success, "BUG: duplicate pending flow: {:?}", id);
}

fn on_invalid_input(state: &mut State, input: InvalidInput) {
    let id = state.next_flow_id;
    state.next_flow_id = state
        .next_flow_id
        .increment("BUG: overflow in next_flow_id++");

    state
        .flow_by_tx_hash
        .entry(input.tx_log_id.tx_hash)
        .or_default()
        .push((input.tx_log_id.index, id));

    let flow = InvalidFlow { input };
    let overwritten = state.invalid_flow.insert(id, flow);
    assert!(overwritten.is_none(), "BUG: duplicate flow: {:?}", id);
}

fn on_started(state: &mut State, id: FlowId, chain: Chain, op: Operation, time: Timestamp) {
    let flow = state
        .flow
        .get_mut(&id)
        .unwrap_or_else(|| unreachable!("BUG: cannot find flow to start: {}", id));

    log!(
        DEBUG,
        "[{:?}]: flow {}: start step {:?} {:?}",
        flow.input.evm_chain,
        id,
        chain,
        op,
    );

    let mut handled = false;

    for step in flow.step.iter_mut() {
        match step.progress {
            Progress::Planned => {
                assert_eq!(step.chain, chain, "BUG: starting wrong step: {}", id);
                assert_eq!(step.op, op, "BUG: starting wrong step: {}", id);
                step.progress = Progress::Running;
                step.start = Some(time);
                handled = true;
                break;
            }
            Progress::Running => {
                unreachable!("BUG: starting a new step, while the previous one is running");
            }
            Progress::Succeeded(..) | Progress::Failed { .. } => {
                // Nothing to do.
            }
        }
    }

    assert!(
        handled,
        "BUG: unhandled start step for flow {}: step={:?}",
        id,
        (chain, op),
    )
}

fn finish_step(
    state: &mut State,
    id: FlowId,
    chain: Chain,
    op: Operation,
    tx: Option<TxId>,
    finish: Progress,
    time: Timestamp,
) {
    let flow = state
        .flow
        .get_mut(&id)
        .unwrap_or_else(|| unreachable!("BUG: cannot find flow to finish: {}", id));

    log!(
        DEBUG,
        "[{:?}]: flow {}: finish step {:?} {:?}: {:?}",
        flow.input.evm_chain,
        id,
        chain,
        op,
        finish,
    );

    let has_failed = match &finish {
        Progress::Failed { .. } => true,
        Progress::Planned | Progress::Running | Progress::Succeeded(..) => false,
    };

    let mut step_number = None;

    for (i, step) in flow.step.iter_mut().enumerate() {
        match step.progress {
            Progress::Running => {
                assert_eq!(step.chain, chain, "BUG: finishing wrong step: {}", id);
                assert_eq!(step.op, op, "BUG: finishing wrong step: {}", id);
                if let Some(tx) = &tx {
                    assert!(tx.matches(chain), "BUG: mismatch {:?} vs {:?}", tx, chain);
                }
                step.progress = finish;
                step.end = Some(time);
                step_number = Some(i);
                break;
            }
            Progress::Planned => unreachable!("BUG: finishing step that was not started: {}", id),
            Progress::Succeeded(..) | Progress::Failed { .. } => {
                // Nothing to do.
            }
        }
    }

    match step_number {
        Some(i) => {
            if i + 1 == flow.step.len() || has_failed {
                // This was the last step or the step has failed, now the flow
                // is no longer pending.
                let success = state.pending.remove(&id);
                assert!(success, "BUG: missing pending flow: {:?}", id);
            }
        }
        None => {
            unreachable!("BUG: unhandled finish step: {}, {:?}", id, (chain, op))
        }
    }

    // This is a special case for EVM to allow looking up the flow from the
    // transaction hash and log.
    if let Some(TxId::Evm(tx)) = tx {
        state
            .flow_by_tx_hash
            .entry(tx.tx_hash)
            .or_default()
            .push((tx.index, id));
    }
}
