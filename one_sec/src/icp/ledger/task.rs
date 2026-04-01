//! This module defines tasks of the ICP ledger state machine.
use candid::{CandidType, Nat, Principal};
use ic_canister_log::log;
use ic_ledger_types::AccountIdentifier;
use icrc_ledger_client_cdk::{CdkRuntime, ICRC1Client};
use icrc_ledger_types::icrc1::{
    account::Account as IcrcAccount,
    transfer::{TransferArg, TransferError},
};
use num_traits::ToPrimitive;
use serde::Deserialize;

use crate::{
    api::types::{Chain, Token},
    config::OperatingMode,
    event::process_event,
    flow::{
        self,
        event::{Input, Operation, TxId},
        state::FlowId,
        trace::{self, TraceEvent},
    },
    icp::{
        ledger::state::{read_ledger_state, Request},
        IcpAccount,
    },
    logs::{DEBUG, ERROR},
    metrics::CanisterCall,
    numeric::{Amount, BlockIndex},
    state::mutate_state,
    task::{schedule_after, schedule_now, timestamp_ms},
};

use super::event::Event;

const NANOS_PER_MS: u64 = 1_000_000;

/// A task of the ICP ledger state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    /// A task that performs transfers for unlocking and minting steps.
    Transfer,
    /// A task that transfers the collected fees.
    TransferFee,
}

impl Task {
    pub async fn run(self, token: Token) -> Result<(), String> {
        match self {
            Task::Transfer => transfer_task(token).await,
            Task::TransferFee => transfer_fee_task(token).await,
        }
    }

    pub fn get_all_tasks(token: Token) -> Vec<crate::task::TaskType> {
        vec![Task::Transfer.wrap(token), Task::TransferFee.wrap(token)]
    }

    pub fn wrap(self, token: Token) -> crate::task::TaskType {
        crate::task::TaskType::Icp(crate::icp::Task::Ledger { token, task: self })
    }
}

async fn transfer_task(token: Token) -> Result<(), String> {
    let start = timestamp_ms();
    let config = read_ledger_state(token, |s| s.config.clone());

    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: config.canister,
    };

    let fee = config.transfer_fee;

    fail_quarantined(token);

    let batch: Vec<_> = read_ledger_state(token, |s| {
        s.pending
            .iter()
            .take(config.transfer_batch)
            .cloned()
            .collect()
    });

    let awaiting: Vec<_> = batch
        .into_iter()
        .map(|id| do_transfer(&client, token, fee, id, config.supports_account_id))
        .collect();

    if !awaiting.is_empty() {
        let count = awaiting.len();
        let results = futures::future::join_all(awaiting).await;
        for result in results {
            if let Err(err) = result {
                log!(ERROR, "{}", err);
            }
        }
        log!(
            DEBUG,
            "{:?} ledger: {} transfers in {}ms",
            token,
            count,
            timestamp_ms().into_inner() - start.into_inner()
        );
    }

    let task_delay = if read_ledger_state(token, |s| s.pending.is_empty()) {
        config.transfer_task_idle_delay
    } else {
        config.transfer_task_busy_delay
    };

    schedule_after(task_delay, Task::Transfer.wrap(token), "recurring".into());
    Ok(())
}

async fn do_transfer(
    client: &ICRC1Client<CdkRuntime>,
    token: Token,
    fee: Amount,
    id: FlowId,
    supports_account_id: bool,
) -> Result<(), String> {
    let request = read_ledger_state(token, |s| s.unlock_or_mint.get(&id).cloned())
        .ok_or_else(|| format!("BUG: failed to find an unlock/mint request by id: {}", id))?;

    if !read_ledger_state(token, |s| s.pending.contains(&request.id)) {
        return Err(format!(
            "BUG: a flow disappeared from the pending set: {:?}",
            request.id
        ));
    }

    match request.op {
        Operation::Lock | Operation::Burn => {
            return Err(format!(
                "BUG: lock and burn operations should not be pending: {:?}",
                request
            ));
        }
        Operation::Mint => {
            // Nothing to do.
        }
        Operation::Unlock => {
            // Sanity check: if the ledger balance is not sufficient for the transfer,
            // then fail the flow instead of potentially panicking after the transfer.
            let new_balance = read_ledger_state(token, |s| {
                s.balance
                    .checked_sub(request.amount)
                    .and_then(|x| x.checked_sub(fee))
            });

            if new_balance.is_none() {
                let id = request.id;
                let op = request.op;
                let err = format!("transfer failed due to insufficient balance: {}", id);
                process_event(
                    Event::Failed {
                        id,
                        op,
                        err: err.clone(),
                    }
                    .wrap(token),
                );
                process_event(
                    flow::Event::FailedStep {
                        id,
                        chain: Chain::ICP,
                        op,
                        tx: None,
                        err: err.clone(),
                    }
                    .wrap(),
                );
                trace::err(
                    id,
                    TraceEvent::ConfirmTx,
                    TxId::Icp(BlockIndex::ZERO),
                    None,
                    &err,
                );
                return Ok(());
            }
        }
    }

    match request.account {
        IcpAccount::ICRC(account) => transfer_icrc1(client, token, fee, request, account).await,
        IcpAccount::AccountId(account_id) => {
            if supports_account_id {
                transfer_custom(client.ledger_canister_id, token, fee, request, account_id).await
            } else {
                let err = format!("{:?} ledger does not support account identifiers", token);
                process_event(
                    Event::Failed {
                        id,
                        op: request.op,
                        err: err.clone(),
                    }
                    .wrap(token),
                );
                process_event(
                    flow::Event::FailedStep {
                        id,
                        chain: Chain::ICP,
                        op: request.op,
                        tx: None,
                        err: err.clone(),
                    }
                    .wrap(),
                );
                trace::err(
                    id,
                    TraceEvent::ConfirmTx,
                    TxId::Icp(BlockIndex::ZERO),
                    None,
                    &err,
                );
                Ok(())
            }
        }
    }
}

async fn transfer_icrc1(
    client: &ICRC1Client<CdkRuntime>,
    token: Token,
    fee: Amount,
    request: Request,
    account: IcrcAccount,
) -> Result<(), String> {
    let transfer_args = TransferArg {
        to: account,
        fee: Some(fee.into()),
        amount: request.amount.into(),
        memo: Some(request.id.into_inner().into()),
        from_subaccount: None,
        created_at_time: Some(request.created_at.into_inner() * NANOS_PER_MS),
    };

    quarantine(token, request.id);

    let cc = CanisterCall::new(client.ledger_canister_id, "transfer", 0);
    let result = client.transfer(transfer_args).await;

    let id = request.id;
    let op = request.op;

    if !read_ledger_state(token, |s| s.pending.contains(&id)) {
        log!(
            ERROR,
            "BUG: flow {} is no longer pending after transfer: {:?}",
            id,
            result
        );
        discharge(token, request.id);
        return Ok(());
    }

    // Do not use early returns here because of `discharge()`.
    match result {
        Ok(Ok(n)) => {
            cc.returned_ok();
            let tx = as_block_index(n);
            process_event(Event::Succeeded { id, op, tx }.wrap(token));
            process_event(
                flow::Event::SucceededStep {
                    id,
                    chain: Chain::ICP,
                    op,
                    tx: TxId::Icp(tx),
                }
                .wrap(),
            );
            trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);
            discharge(token, request.id);
            Ok(())
        }
        Ok(Err(TransferError::Duplicate { duplicate_of })) => {
            let tx = as_block_index(duplicate_of);
            process_event(Event::Succeeded { id, op, tx }.wrap(token));
            process_event(
                flow::Event::SucceededStep {
                    id,
                    chain: Chain::ICP,
                    op,
                    tx: TxId::Icp(tx),
                }
                .wrap(),
            );
            trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);
            discharge(token, request.id);
            Ok(())
        }
        Ok(Err(err)) => {
            let err_msg = err.to_string();
            cc.returned_err(&err_msg);
            discharge(token, request.id);
            // We will retry this transfer in subsequent tasks.
            Ok(())
        }
        Err((code, message)) => {
            cc.returned_err(&message);
            discharge(token, request.id);
            // We couldn't make a call. There is no point in
            // continuing with other withdrawals.
            Err(format!(
                "failed to call transfer for {:?}: code={}, message={}",
                id, code, message
            ))
        }
    }
}

async fn transfer_custom(
    ledger_canister_id: Principal,
    token: Token,
    fee: Amount,
    request: Request,
    account: AccountIdentifier,
) -> Result<(), String> {
    use ic_ledger_types::Memo;
    use ic_ledger_types::Tokens;
    use ic_ledger_types::TransferArgs;
    use ic_ledger_types::TransferError;

    let fee: u64 = fee
        .into_inner()
        .try_into()
        .map_err(|err| format!("Fee amount {} does not fit u64: {}", fee, err))?;

    let amount: u64 = request
        .amount
        .into_inner()
        .try_into()
        .map_err(|err| format!("Amount {} does not fit u64: {}", request.amount, err))?;

    let transfer_args = TransferArgs {
        to: account,
        fee: Tokens::from_e8s(fee),
        amount: Tokens::from_e8s(amount),
        memo: Memo(request.id.into_inner()),
        from_subaccount: None,
        created_at_time: Some(ic_ledger_types::Timestamp {
            timestamp_nanos: request.created_at.into_inner() * NANOS_PER_MS,
        }),
    };

    quarantine(token, request.id);

    let cc = CanisterCall::new(ledger_canister_id, "transfer", 0);
    let result = ic_ledger_types::transfer(ledger_canister_id, transfer_args).await;

    let id = request.id;
    let op = request.op;

    if !read_ledger_state(token, |s| s.pending.contains(&id)) {
        log!(
            ERROR,
            "BUG: flow {} is no longer pending after transfer: {:?}",
            id,
            result
        );
        discharge(token, request.id);
        return Ok(());
    }

    // Do not use early returns here because of `discharge()`.
    match result {
        Ok(Ok(n)) => {
            cc.returned_ok();
            let tx = as_block_index(Nat::from(n));
            process_event(Event::Succeeded { id, op, tx }.wrap(token));
            process_event(
                flow::Event::SucceededStep {
                    id,
                    chain: Chain::ICP,
                    op,
                    tx: TxId::Icp(tx),
                }
                .wrap(),
            );
            trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);
            discharge(token, request.id);
            Ok(())
        }
        Ok(Err(TransferError::TxDuplicate { duplicate_of })) => {
            let tx = as_block_index(Nat::from(duplicate_of));
            process_event(Event::Succeeded { id, op, tx }.wrap(token));
            process_event(
                flow::Event::SucceededStep {
                    id,
                    chain: Chain::ICP,
                    op,
                    tx: TxId::Icp(tx),
                }
                .wrap(),
            );
            trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);
            discharge(token, request.id);
            Ok(())
        }
        Ok(Err(err)) => {
            let err_msg = err.to_string();
            cc.returned_err(&err_msg);
            discharge(token, request.id);
            // We will retry this transfer in subsequent tasks.
            Ok(())
        }
        Err((code, message)) => {
            cc.returned_err(&message);
            discharge(token, request.id);
            // We couldn't make a call. There is no point in
            // continuing with other withdrawals.
            Err(format!(
                "failed to call transfer for {:?}: code={:?}, message={}",
                id, code, message
            ))
        }
    }
}

/// Converts the given value into a block index.
/// Note: it returns 0 if the value doesn't fit `u64` (the case that shouldn't
/// happen with real ledgers).
pub fn as_block_index(value: Nat) -> BlockIndex {
    BlockIndex::new(value.0.to_u64().unwrap_or_default())
}

/// Starts a mint step for the given flow and input.
pub fn start_mint(id: FlowId, input: Input) -> Result<(), String> {
    let token = input.icp_token;
    let chain = Chain::ICP;
    let op = Operation::Mint;

    let config = read_ledger_state(token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Minter {
        let err = format!(
            "mint requested for ICP locker: id={}, token={:?}",
            id, token
        );
        log!(ERROR, "BUG: {}", err);
        return Err(err);
    }

    process_event(flow::Event::StartedStep { id, chain, op }.wrap());
    process_event(
        Event::Started {
            id,
            op,
            account: input.icp_account,
            amount: input.icp_amount,
            collected_fee: input
                .fee()
                .checked_sub(config.transfer_fee)
                .unwrap_or_default(),
            ledger_fee: config.transfer_fee,
        }
        .wrap(token),
    );

    schedule_now(Task::Transfer.wrap(token), "start mint".into());

    Ok(())
}

/// Starts an unlock step for the given flow and input.
pub fn start_unlock(id: FlowId, input: Input) -> Result<(), String> {
    let token = input.icp_token;
    let chain = Chain::ICP;
    let op = Operation::Unlock;

    let config = read_ledger_state(token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Locker {
        let err = format!(
            "unlock requested for ICP minter: id={}, token={:?}",
            id, token
        );
        log!(ERROR, "BUG: {}", err);
        return Err(err);
    }

    let ledger_fee = config.transfer_fee;

    // Sanity check of the balance to avoid panicking when handling the events below.
    let balance = read_ledger_state(token, |s| s.balance);
    if balance
        .checked_sub(input.icp_amount)
        .and_then(|x| x.checked_sub(ledger_fee))
        .is_none()
    {
        return Err(format!(
            "BUG: icp/ledger {:?}: underflow in unlock: {} {} vs {}",
            token, id, balance, input.icp_amount,
        ));
    }

    process_event(flow::Event::StartedStep { id, chain, op }.wrap());
    process_event(
        Event::Started {
            id,
            op,
            account: input.icp_account,
            amount: input.icp_amount,
            collected_fee: input.fee().checked_sub(ledger_fee).unwrap_or_default(),
            ledger_fee,
        }
        .wrap(token),
    );

    schedule_now(Task::Transfer.wrap(token), "start unlock".into());

    Ok(())
}

/// Processes a burn step of the given flow that has already happened in the
/// given block index.
pub fn record_burn(id: FlowId, input: Input, tx: BlockIndex) -> Result<(), String> {
    let token = input.icp_token;
    let chain = Chain::ICP;
    let op = Operation::Burn;

    let config = read_ledger_state(token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Minter {
        let err = format!(
            "burn requested for ICP locker: id={}, token={:?}",
            id, token
        );
        log!(ERROR, "BUG: {}", err);
        return Err(err);
    }

    let ledger_fee = config.transfer_fee;

    // Sanity check of the balance to avoid panicking when handling the events below.
    let balance = read_ledger_state(token, |s| s.balance);
    if balance
        .checked_sub(input.icp_amount)
        .and_then(|x| x.checked_sub(ledger_fee))
        .is_none()
    {
        return Err(format!(
            "BUG: icp/ledger {:?}: underflow in burn: {} {} vs {}",
            token, id, balance, input.icp_amount,
        ));
    }

    process_event(
        Event::Started {
            id,
            op,
            account: input.icp_account,
            amount: input.icp_amount,
            collected_fee: input.fee().checked_sub(ledger_fee).unwrap_or_default(),
            ledger_fee,
        }
        .wrap(token),
    );
    process_event(Event::Succeeded { id, op, tx }.wrap(token));

    process_event(flow::Event::StartedStep { id, chain, op }.wrap());
    process_event(
        flow::Event::SucceededStep {
            id,
            chain: Chain::ICP,
            op,
            tx: TxId::Icp(tx),
        }
        .wrap(),
    );

    trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);

    Ok(())
}

/// Processes a lock step of the given flow that has already happened in the
/// given block index.
pub fn record_lock(id: FlowId, input: Input, tx: BlockIndex) -> Result<(), String> {
    let token = input.icp_token;
    let chain = Chain::ICP;
    let op = Operation::Lock;

    let config = read_ledger_state(token, |s| s.config.clone());
    if config.operating_mode != OperatingMode::Locker {
        let err = format!(
            "lock requested for ICP minter: id={}, token={:?}",
            id, token
        );
        log!(ERROR, "BUG: {}", err);
        return Err(err);
    }

    process_event(
        Event::Started {
            id,
            op,
            account: input.icp_account,
            amount: input.icp_amount,
            collected_fee: input
                .fee()
                .checked_sub(config.transfer_fee)
                .unwrap_or_default(),
            ledger_fee: config.transfer_fee,
        }
        .wrap(token),
    );
    process_event(Event::Succeeded { id, op, tx }.wrap(token));

    process_event(flow::Event::StartedStep { id, chain, op }.wrap());
    process_event(
        flow::Event::SucceededStep {
            id,
            chain: Chain::ICP,
            op,
            tx: TxId::Icp(tx),
        }
        .wrap(),
    );

    trace::ok(id, TraceEvent::ConfirmTx, TxId::Icp(tx), None);

    Ok(())
}

/// Fails all flows in the quarantine list.
fn fail_quarantined(token: Token) {
    let quarantine = mutate_state(|s| {
        std::mem::take(
            &mut s
                .icp
                .ledger
                .get_mut(&token)
                .unwrap_or_else(|| unreachable!("cannot find ICP ledger for {:?}", token))
                .quarantine,
        )
    });

    for id in quarantine {
        if !read_ledger_state(token, |s| s.pending.contains(&id)) {
            log!(
                ERROR,
                "BUG: a quarantined flow is no longer pending: {}",
                id
            );
            continue;
        }
        let Some(op) = read_ledger_state(token, |s| s.unlock_or_mint.get(&id).map(|r| r.op)) else {
            log!(
                ERROR,
                "BUG: cannot find an unlock/mint request for quarantined flow: {}",
                id
            );
            continue;
        };
        let err = format!("transfer was paused due to an internal error: {}", id);
        process_event(
            Event::Failed {
                id,
                op,
                err: err.clone(),
            }
            .wrap(token),
        );
        process_event(
            flow::Event::FailedStep {
                id,
                chain: Chain::ICP,
                op,
                tx: None,
                err: err.clone(),
            }
            .wrap(),
        );
        trace::err(
            id,
            TraceEvent::ConfirmTx,
            TxId::Icp(BlockIndex::ZERO),
            None,
            &err,
        );
    }
}

/// Add the flow to the quarantine list.
fn quarantine(token: Token, id: FlowId) {
    mutate_state(|s| {
        s.icp
            .ledger
            .get_mut(&token)
            .unwrap_or_else(|| unreachable!("cannot find ICP ledger for {:?}", token))
            .quarantine
            .push(id);
    });
}

/// Removes the flow from the quarantine list.
fn discharge(token: Token, id: FlowId) {
    mutate_state(|s| {
        s.icp
            .ledger
            .get_mut(&token)
            .unwrap_or_else(|| unreachable!("cannot find ICP ledger for {:?}", token))
            .quarantine
            .retain(|x| *x != id);
    });
}

async fn transfer_fee_task(token: Token) -> Result<(), String> {
    let config = read_ledger_state(token, |s| s.config.clone());

    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: config.canister,
    };

    let ledger_fee = config.transfer_fee;
    let collected_fee = read_ledger_state(token, |s| s.fees);

    if collected_fee < config.fee_threshold.max(ledger_fee) {
        schedule_after(
            config.transfer_fee_task_delay,
            Task::TransferFee.wrap(token),
            "recurring".into(),
        );

        return Ok(());
    }

    if config.operating_mode == OperatingMode::Locker {
        let balance = read_ledger_state(token, |s| s.balance);
        if balance.checked_sub(collected_fee).is_none() {
            log!(
                ERROR,
                "BUG: {:?} ledger: underflow in balance -= collected_fee: {} {}",
                token,
                collected_fee,
                balance
            );
            return Ok(());
        }
    }

    let now = timestamp_ms().into_inner();

    let amount = collected_fee.sub(ledger_fee, "BUG: underflow in collected_fee - ledger_fee");

    let transfer_args = TransferArg {
        to: IcrcAccount {
            owner: config.fee_receiver,
            subaccount: None,
        },
        fee: Some(ledger_fee.into()),
        amount: amount.into(),
        memo: Some(now.into()),
        from_subaccount: None,
        created_at_time: Some(now * NANOS_PER_MS),
    };

    let cc = CanisterCall::new(client.ledger_canister_id, "transfer", 0);
    let result = client.transfer(transfer_args).await;

    match result {
        Ok(Ok(_n)) => {
            cc.returned_ok();
            process_event(Event::TransferredFee { amount, ledger_fee }.wrap(token));
        }
        Ok(Err(TransferError::Duplicate { duplicate_of })) => {
            let tx = as_block_index(duplicate_of);
            let err = format!("duplicate transaction: tx={:?}", tx);
            cc.returned_err(&err);
            log!(ERROR, "Failed to transfer fees for {:?}: {}", token, err);
        }
        Ok(Err(err)) => {
            let err = err.to_string();
            cc.returned_err(&err);
            log!(ERROR, "Failed to transfer fees for {:?}: {}", token, err);
        }
        Err((code, err)) => {
            cc.returned_err(&err);
            log!(
                ERROR,
                "Failed to transfer fees for {:?}: code={} {}",
                token,
                code,
                err
            );
        }
    };

    schedule_after(
        config.transfer_fee_task_delay,
        Task::TransferFee.wrap(token),
        "recurring".into(),
    );

    Ok(())
}

/// Returns how many other pending flows are ahead of this flow in the queue.
pub fn queue_position(token: Token, id: FlowId) -> Option<u64> {
    read_ledger_state(token, |s| {
        s.pending.get(&id)?;
        Some(s.pending.range(..id).count() as u64)
    })
}
