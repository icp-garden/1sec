//! This module defines functions that are called from other tasks.
use crate::{
    api::types::{EvmChain, Token},
    config::OperatingMode,
    event::process_event,
    evm::{
        self,
        reader::{TxLog, TxLogId},
        state::read_evm_state,
        writer,
    },
    flow::{
        self,
        endpoint::advance_flow_to_next_step,
        event::{Input, InvalidInput, Operation, TxId},
        state::FlowId,
        trace::{self, TraceEvent},
    },
    numeric::Wei,
    state::read_state,
    task::schedule_now,
};

use super::{
    parser::{parse_tx_log, pre_parse_tx_log},
    state::read_ledger_state,
    tx::{build_mint_tx, build_unlock_tx},
    Event,
};

/// Starts a mint step for the given flow and input.
pub fn start_mint(id: FlowId, input: Input) -> Result<(), String> {
    let chain = input.evm_chain;
    let token = input.evm_token;
    let op = Operation::Mint;

    let config = read_ledger_state(chain, token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Minter {
        return Err(format!(
            "BUG: mint requested for EVM locker: {:?}",
            (chain, token, id)
        ));
    }

    let tx_input = build_mint_tx(&input, &config);

    process_event(
        flow::Event::StartedStep {
            id,
            chain: chain.into(),
            op,
        }
        .wrap(),
    );

    process_event(
        Event::Started {
            id,
            op,
            account: input.evm_account,
            amount: input.evm_amount,
        }
        .wrap(chain, token),
    );

    process_event(
        writer::Event::Started {
            id,
            op,
            token,
            tx_input,
        }
        .wrap(chain),
    );

    schedule_now(evm::writer::Task::NewTx.wrap(chain), "start mint".into());

    Ok(())
}

/// Starts an unlock step for the given flow and input.
pub fn start_unlock(id: FlowId, input: Input) -> Result<(), String> {
    let chain = input.evm_chain;
    let token = input.evm_token;
    let op = Operation::Unlock;

    let config = read_ledger_state(chain, token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Locker {
        return Err(format!(
            "BUG: unlock requested for EVM minter: {:?}",
            (chain, token, id)
        ));
    }

    // Sanity check of the balance to avoid panicking when handling the events below.
    let balance = read_ledger_state(chain, token, |s| s.maybe_balance()).ok_or_else(|| {
        format!(
            "BUG: evm/ledger {:?}/{:?}: underflow in balance",
            chain, token
        )
    })?;

    if balance.checked_sub(input.evm_amount).is_none() {
        return Err(format!(
            "BUG: evm/ledger {:?}/{:?}: underflow in unlock: {} {} vs {}",
            chain, token, id, balance, input.evm_amount,
        ));
    }

    let tx_input = build_unlock_tx(&input, &config);

    process_event(
        flow::Event::StartedStep {
            id,
            chain: chain.into(),
            op,
        }
        .wrap(),
    );

    process_event(
        Event::Started {
            id,
            op,
            account: input.evm_account,
            amount: input.evm_amount,
        }
        .wrap(chain, token),
    );

    process_event(
        writer::Event::Started {
            id,
            op,
            token,
            tx_input,
        }
        .wrap(chain),
    );

    schedule_now(evm::writer::Task::NewTx.wrap(chain), "start unlock".into());

    Ok(())
}

/// Processes a burn step of the given flow that has already happened in the
/// given transaction.
pub fn record_burn(id: FlowId, input: Input, tx: TxLogId) -> Result<(), String> {
    let token = input.evm_token;
    let chain = input.evm_chain;
    let op = Operation::Burn;

    let config = read_ledger_state(chain, token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Minter {
        return Err(format!(
            "burn requested for EVM locker: {:?}",
            (chain, token, id)
        ));
    }

    // Sanity check of the balance to avoid panicking when handling the events below.
    let balance = read_ledger_state(chain, token, |s| s.maybe_balance()).ok_or_else(|| {
        format!(
            "BUG: evm/ledger {:?}/{:?}: underflow in balance",
            chain, token
        )
    })?;

    if balance.checked_sub(input.evm_amount).is_none() {
        return Err(format!(
            "BUG: evm/ledger {:?}/{:?}: underflow in burn: {} {} vs {}",
            chain, token, id, balance, input.evm_amount,
        ));
    }

    process_event(
        Event::Started {
            id,
            op,
            account: input.evm_account,
            amount: input.evm_amount,
        }
        .wrap(chain, token),
    );
    process_event(Event::Succeeded { id, tx }.wrap(chain, token));

    process_event(
        flow::Event::StartedStep {
            id,
            chain: chain.into(),
            op,
        }
        .wrap(),
    );
    process_event(
        flow::Event::SucceededStep {
            id,
            chain: chain.into(),
            op,
            tx: TxId::Evm(tx),
        }
        .wrap(),
    );

    trace::ok(id, TraceEvent::ConfirmTx, TxId::Evm(tx), None);
    Ok(())
}

/// Processes a lock step of the given flow that has already happened in the
/// given transaction.
pub fn record_lock(id: FlowId, input: Input, tx: TxLogId) -> Result<(), String> {
    let token = input.evm_token;
    let chain = input.evm_chain;
    let op = Operation::Lock;

    let config = read_ledger_state(chain, token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Locker {
        return Err(format!(
            "lock requested for EVM minter: {:?}",
            (chain, token, id)
        ));
    }

    process_event(
        Event::Started {
            id,
            op,
            account: input.evm_account,
            amount: input.evm_amount,
        }
        .wrap(chain, token),
    );
    process_event(Event::Succeeded { id, tx }.wrap(chain, token));

    process_event(
        flow::Event::StartedStep {
            id,
            chain: chain.into(),
            op,
        }
        .wrap(),
    );
    process_event(
        flow::Event::SucceededStep {
            id,
            chain: chain.into(),
            op,
            tx: TxId::Evm(tx),
        }
        .wrap(),
    );
    trace::ok(id, TraceEvent::ConfirmTx, TxId::Evm(tx), None);
    Ok(())
}

/// Parses and handles the given event log corresponding to a burn or lock operation.
pub fn process_tx_log(chain: EvmChain, token: Token, tx_log: TxLog) -> Result<(), String> {
    let tx = tx_log.id;
    let tx_log = pre_parse_tx_log(tx_log);

    match parse_tx_log(chain, token, tx_log) {
        Ok(input) => {
            let flow_id = read_state(|s| s.flow.next_flow_id);
            process_event(flow::Event::Input(input).wrap());
            let flow = read_state(|s| s.flow.flow.get(&flow_id).unwrap().clone());

            match flow.step[0].op {
                Operation::Burn => {
                    record_burn(flow_id, flow.input, tx)?;
                }
                Operation::Lock => {
                    record_lock(flow_id, flow.input, tx)?;
                }
                Operation::Mint | Operation::Unlock => {
                    return Err(format!(
                        "BUG: invalid first step of flow: {}, {:?}",
                        flow_id, flow.step[0]
                    ));
                }
            }
            advance_flow_to_next_step(flow_id)?;
        }
        Err(err) => {
            let input = InvalidInput {
                tx_log_id: tx,
                evm_chain: chain,
                evm_token: token,
                error: err,
            };
            process_event(flow::Event::InvalidInput(input).wrap());
        }
    }

    Ok(())
}

/// Estimates the cost of an unlock/mint transaction for the given token.
pub fn estimate_tx_cost(chain: EvmChain, token: Token) -> Option<(Wei, Wei)> {
    let fee = read_evm_state(chain, |s| s.writer.latest_fee().clone());
    let fee_margin = read_evm_state(chain, |s| s.writer.config.tx_fee_margin);
    let (gas_limit, max_cost) = read_ledger_state(chain, token, |s| {
        (s.config.gas_limit_for_unlock_or_mint, s.config.max_tx_cost)
    });
    fee.map(|fee| (fee.cost(gas_limit, fee_margin), max_cost))
}
