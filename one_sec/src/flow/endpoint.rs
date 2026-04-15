//! This module defines endpoints that are called to start a bridging transfer.
use candid::Principal;
use evm_rpc_types::Hex32;
use ic_ethereum_types::Address;
use icrc_ledger_client_cdk::{CdkRuntime, ICRC1Client};
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;
use icrc_ledger_types::icrc2::transfer_from::TransferFromArgs;
use std::{str::FromStr, time::Duration};

use crate::{
    api::{
        self,
        types::{
            Chain, FetchedBlock, TransferEvmToIcpArg, TransferIcpToEvmArg, TransferId,
            TransferResponse,
        },
    },
    event::process_event,
    evm::{
        self,
        prover::{self},
        read_evm_state, reader,
        writer::{self},
        TxHash,
    },
    guards::GuardPrincipal,
    icp::{
        self,
        ledger::{as_block_index, state::read_ledger_state},
        IcpAccount,
    },
    metrics::CanisterCall,
    numeric::{Amount, TxLogIndex},
    state::read_state,
    task::{schedule_now, schedule_soon},
};

use super::{
    event::{Direction, Event, Input, Operation},
    state::{read_flow_config, FlowId, Progress},
};

/// This function is called when the user notifies about a lock or burn
/// transaction that has happened on an EVM chain.
///
/// If the corresponding event log has already been detected and processed, then
/// the function returns the id that can be used to query more details about
/// bridging.
///
pub fn evm_to_icp(arg: TransferEvmToIcpArg) -> Result<TransferResponse, String> {
    let token = arg.token;
    let evm_chain = arg.evm_chain;
    let evm_tx = arg.evm_tx;

    if !read_state(|s| {
        s.flow
            .config
            .contains_key(&(Direction::EvmToIcp, token, evm_chain, token))
    }) {
        return Err(format!(
            "transfers of {:?} are not supported from {:?}",
            token, evm_chain
        ));
    }

    let tx_hash = TxHash(
        Hex32::try_from(evm_tx.hash)
            .map_err(|err| format!("Couldn't parse the transaction hash: {}", err))?
            .into(),
    );

    let log_index = evm_tx.log_index.map(TxLogIndex::from);

    let candidates = read_state(|s| {
        s.flow
            .flow_by_tx_hash
            .get(&tx_hash)
            .cloned()
            .unwrap_or_default()
    });

    let candidates: Vec<_> = candidates
        .into_iter()
        .filter(|x| log_index.is_none() || Some(x.0) == log_index)
        .collect();

    if candidates.is_empty() {
        let fetched_safe_block = read_evm_state(evm_chain, |s| {
            s.prover
                .head
                .safe
                .as_ref()
                .map(|s| s.block_number)
                .unwrap_or_default()
        });
        schedule_soon(
            Duration::from_secs(10),
            prover::Task::FetchLatestBlock.wrap(evm_chain),
            "evm_to_icp".into(),
        );
        schedule_soon(
            Duration::from_secs(10),
            reader::Task::FetchTxLogs.wrap(evm_chain),
            "evm_to_icp".into(),
        );
        Ok(TransferResponse::Fetching(FetchedBlock {
            block_height: fetched_safe_block.into_inner(),
        }))
    } else if candidates.len() == 1 {
        Ok(TransferResponse::Accepted(TransferId {
            id: candidates[0].1.into_inner(),
        }))
    } else {
        Err(format!(
            "Multiple deposits for this transaction, use log index to pick one: {:?}",
            candidates
        ))
    }
}

/// This function is called when the user wants to start bridging from ICP to
/// EVM. It locks or burns ICP tokens and starts minting or unlocking of the EVM
/// token.
pub async fn icp_to_evm(arg: TransferIcpToEvmArg) -> Result<TransferResponse, String> {
    let token = arg.token;
    let evm_chain = arg.evm_chain;

    if !read_state(|s| {
        s.flow
            .config
            .contains_key(&(Direction::IcpToEvm, token, evm_chain, token))
    }) {
        return Err(format!(
            "transfers of {:?} are not supported to {:?}",
            token, evm_chain
        ));
    }

    let icp_account = match arg.icp_account {
        api::types::IcpAccount::ICRC(account) => account,
        api::types::IcpAccount::AccountId(_) => {
            return Err(
                "Account identifiers are not supported. Please specify an ICP principal.".into(),
            );
        }
    };

    if icp_account.owner != ic_cdk::caller() {
        return Err("Transfer must be initiated by the source account".into());
    }

    if icp_account.owner == Principal::anonymous() {
        return Err("Transfers from anonymous account are not allowed".into());
    }

    let _guard = GuardPrincipal::new(icp_account.owner)
        .map_err(|_| "Please wait until your previous transfer completes.")?;

    let pending_flows: Vec<_> = read_state(|s| s.flow.pending.iter().cloned().collect());
    let max_concurrent_flows = read_state(|s| s.flow.max_concurrent_flows);

    if pending_flows.len() >= max_concurrent_flows {
        return Err(format!(
            "The alpha version allows {} concurrent transfers. \
            This restriction will be lifted in future. \
            Currently processing transfers: {:?}.",
            max_concurrent_flows, pending_flows
        ));
    }

    if !read_state(|s| s.evm.contains_key(&evm_chain)) {
        return Err(format!("Unsupported destination chain: {:?}", evm_chain));
    }

    if !read_state(|s| s.icp.ledger.contains_key(&token)) {
        return Err(format!("Unsupported source token: {:?}", token));
    }

    if !read_evm_state(evm_chain, |s| s.ledger.contains_key(&token)) {
        return Err(format!("Unsupported destination token: {:?}", token));
    }

    let evm_account = Address::from_str(&arg.evm_account.address)?;

    let icp_amount = Amount::try_from(arg.icp_amount)?;

    let config = read_flow_config(Direction::IcpToEvm, token, evm_chain, token, |c| c.clone());

    if icp_amount < config.min_amount {
        return Err("The amount is too low".into());
    }

    if icp_amount > config.max_amount {
        return Err("The amount is too high".into());
    }

    let ledger_config = read_ledger_state(token, |s| s.config.clone());
    let ledger_fee = ledger_config.transfer_fee;

    let fee_percent = if read_state(|s| s.icp.config.market_makers.contains(&icp_account.owner)) {
        config.fee.as_f64() / 10.0
    } else {
        config.fee.as_f64()
    };
    let protocol_fee = Amount::new((icp_amount.as_f64() * fee_percent).round() as u128);
    let Some((tx_cost, max_tx_cost)) = evm::ledger::estimate_tx_cost(evm_chain, token) else {
        schedule_now(
            writer::Task::FetchFeeEstimate.wrap(evm_chain),
            "icp_to_evm".into(),
        );
        return Err(
            "Transaction fee estimate is too old. Please retry after a few seconds.".into(),
        );
    };
    if tx_cost > max_tx_cost {
        return Err(format!(
            "EVM transaction cost is too high: {} wei, limit = {} wei",
            tx_cost, max_tx_cost
        ));
    }

    let tx_cost_in_token = icp::exchange_rate::convert_eth_to_token(tx_cost, token)?;

    let total_fee = ledger_fee
        .add(protocol_fee, "BUG: impossible")
        .add(tx_cost_in_token, "BUG: impossible");
    let evm_amount = icp_amount
        .checked_sub(total_fee)
        .ok_or("The amount is too low")?;

    if let Some(min_amount) = arg.evm_amount {
        let min_amount = Amount::try_from(min_amount)?;
        if evm_amount < min_amount {
            return Err("The fee has increased. Please retry.".to_string());
        }
    }

    if let Some(available) = read_evm_state(evm_chain, |s| {
        s.ledger.get(&token).and_then(|s| s.available())
    }) {
        if evm_amount > available {
            return Err(format!(
                "Liquidity of {:?} has decreased on {:?}. Please retry.",
                token, evm_chain
            ));
        }
    }

    let transfer_amount = icp_amount.sub(ledger_fee, "BUG: impossible");

    // Include identifying info in the memo so that if the canister traps
    // after a successful transfer_from but before recording the flow event,
    // the stuck funds can be traced via the ledger transaction history.
    // Must stay ≤ 8 bytes: some ledgers (e.g. USDC) enforce that limit.
    let memo_data = icp_amount.into_inner() as u64;

    let transfer_args = TransferFromArgs {
        from: icp_account,
        to: IcrcAccount {
            owner: ic_cdk::id(),
            subaccount: None,
        },
        amount: transfer_amount.into(),
        fee: Some(ledger_fee.into()),
        memo: Some(memo_data.into()),
        created_at_time: Some(ic_cdk::api::time()),
        spender_subaccount: None,
    };

    let client = ICRC1Client {
        runtime: CdkRuntime,
        ledger_canister_id: ledger_config.canister,
    };

    let cc = CanisterCall::new(ledger_config.canister, "transferFrom", 0);
    let block_index = match client.transfer_from(transfer_args).await {
        Ok(Ok(block_index)) => {
            cc.returned_ok();
            as_block_index(block_index)
        }
        Ok(Err(err)) => {
            cc.returned_err(err.to_string());
            return Err(format!("{}", err));
        }
        Err((code, message)) => {
            cc.returned_err(&message);
            return Err(format!(
                "ICP ledger failed to transfer: {} {}",
                code, message
            ));
        }
    };

    let input = Input {
        direction: Direction::IcpToEvm,
        icp_account: IcpAccount::ICRC(icp_account),
        icp_token: token,
        icp_amount,
        evm_chain,
        evm_account,
        evm_token: token,
        evm_amount,
    };

    let flow_id = read_state(|s| s.flow.next_flow_id);
    process_event(Event::Input(input).wrap());
    let flow = read_state(|s| s.flow.flow.get(&flow_id).unwrap().clone());

    match flow.step[0].op {
        Operation::Burn => {
            icp::ledger::record_burn(flow_id, flow.input, block_index)?;
        }
        Operation::Lock => {
            icp::ledger::record_lock(flow_id, flow.input, block_index)?;
        }
        Operation::Mint | Operation::Unlock => {
            return Err(format!(
                "BUG: invalid first step of flow: {}, {:?}",
                flow_id, flow.step[0]
            ));
        }
    }

    advance_flow_to_next_step(flow_id)?;

    Ok(TransferResponse::Accepted(TransferId {
        id: flow_id.into_inner(),
    }))
}

/// Starts the second step of the given flow.
pub fn advance_flow_to_next_step(id: FlowId) -> Result<(), String> {
    let flow = read_state(|s| s.flow.flow.get(&id).cloned())
        .ok_or_else(|| format!("missing flow {}", id))?;

    for step in flow.step.iter() {
        match &step.progress {
            Progress::Running => {
                return Err(format!(
                    "BUG: attempt to advance flow to next step while there is a running step: {:?}",
                    flow,
                ));
            }
            Progress::Failed { .. } => {
                return Err(format!(
                    "BUG: attempt to advance flow to next step after a failed step: {:?}",
                    flow,
                ));
            }
            Progress::Succeeded(..) => {
                // Nothing to do. Proceed to the next step.
            }
            Progress::Planned => {
                match step.op {
                    Operation::Mint => match step.chain {
                        Chain::ICP => {
                            return icp::ledger::start_mint(id, flow.input);
                        }
                        Chain::Base | Chain::Arbitrum | Chain::Ethereum => {
                            return evm::ledger::start_mint(id, flow.input);
                        }
                    },
                    Operation::Unlock => match step.chain {
                        Chain::ICP => {
                            return icp::ledger::start_unlock(id, flow.input);
                        }
                        Chain::Base | Chain::Arbitrum | Chain::Ethereum => {
                            return evm::ledger::start_unlock(id, flow.input);
                        }
                    },
                    Operation::Lock | Operation::Burn => {
                        // These steps are performed either by an endpoint or by reader.
                        // Nothing to do here.
                        return Ok(());
                    }
                }
            }
        }
    }

    Err(format!(
        "failed to advance flow {}: steps={:?}",
        id, flow.step
    ))
}
