use candid::CandidType;
use ic_canister_log::log;
use ic_management_canister_types_private::DerivationPath;
use ic_secp256k1::PublicKey;
use serde::Deserialize;

use crate::{
    api::types::{EvmChain, RequestedTx},
    config::OperatingMode,
    evm::{
        forwarder::state::{Signed, SigningArgs, SigningData},
        ledger::{call_burn_or_lock_tx, call_tx_with_address_and_amount, encode_icp_account},
        state::{mutate_evm_state, read_evm_state},
        tx::{
            wrap_signature, AccessList, Eip1559TransactionRequest, SignedEip1559TransactionRequest,
        },
    },
    logs::DEBUG,
    numeric::{Percent, TxNonce, Wei},
    state::read_state,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    SignTx,
}

impl Task {
    pub async fn run(self, chain: EvmChain) -> Result<(), String> {
        match self {
            Task::SignTx => sign_tx_task(chain).await,
        }
    }

    pub fn wrap(self, chain: EvmChain) -> crate::task::TaskType {
        crate::task::TaskType::Evm {
            chain,
            task: crate::evm::Task::Forwarder(self),
        }
    }

    pub fn get_all_tasks(chain: EvmChain) -> Vec<crate::task::TaskType> {
        vec![Task::SignTx.wrap(chain)]
    }
}

pub async fn sign_tx_task(chain: EvmChain) -> Result<(), String> {
    let batch_size = read_evm_state(chain, |s| s.forwarder.config.batch_size);

    for _ in 0..batch_size {
        let Some((fa, data, args)) = mutate_evm_state(chain, |s| {
            let fa = s.forwarder.signing_queue.front()?;
            let signing = s.forwarder.signing_map.get_mut(fa)?;
            let mut entry = signing.args.first_entry()?;
            let args = entry.get_mut().front()?;
            Some((fa.clone(), signing.data.clone(), args.clone()))
        }) else {
            break;
        };

        let signed = sign_txs(chain, &data, &args).await?;

        mutate_evm_state(chain, |s| {
            let nonce = args.nonce;
            if let Some(signing) = s.forwarder.signing_map.get_mut(&fa) {
                if let Some(queue) = signing.args.get_mut(&nonce) {
                    if Some(&args) == queue.front() {
                        queue.pop_front();
                        s.forwarder
                            .signed
                            .entry(fa.clone())
                            .or_default()
                            .push(signed);
                    }
                    if queue.is_empty() {
                        signing.args.remove(&nonce);
                        if signing.args.is_empty() {
                            s.forwarder.signing_map.remove(&fa);
                        }
                    }
                }
            }
            if Some(&fa) == s.forwarder.signing_queue.front() {
                s.forwarder.signing_queue.pop_front();
                if s.forwarder.signing_map.contains_key(&fa) {
                    s.forwarder.signing_queue.push_back(fa);
                }
            }
        });
    }

    Ok(())
}

async fn sign_txs(
    chain: EvmChain,
    data: &SigningData,
    args: &SigningArgs,
) -> Result<Signed, String> {
    let mut total_tx_cost_in_wei: u64 = 0;
    let approve_tx = match args.requested_tx {
        RequestedTx::Burn | RequestedTx::Lock => None,
        RequestedTx::ApproveAndLock => {
            let (tx, tx_cost) = sign_approve_tx(chain, data, args).await?;
            total_tx_cost_in_wei += tx_cost;
            Some(tx)
        }
    };

    let (tx, tx_cost) = sign_lock_or_burn_tx(chain, data, args).await?;
    total_tx_cost_in_wei += tx_cost;

    Ok(Signed {
        nonce: args.nonce.into_inner(),
        receiver: data.receiver,
        total_tx_cost_in_wei,
        approve_tx,
        lock_or_burn_tx: tx,
    })
}

async fn sign_approve_tx(
    chain: EvmChain,
    data: &SigningData,
    args: &SigningArgs,
) -> Result<(SignedEip1559TransactionRequest, u64), String> {
    let ecdsa_key_name = read_state(|s| s.icp.config.ecdsa_key_name.clone());
    let chain_id = read_evm_state(chain, |s| s.chain_id);
    let amount = read_evm_state(chain, |s| s.forwarder.config.approve_amount).max(args.amount);
    let (erc20, helper, gas_limit, mode) = read_evm_state(chain, |s| {
        s.ledger.get(&data.token).map(|x| {
            (
                x.config.erc20_address,
                x.config.logger_address,
                x.config.gas_limit_for_approve,
                x.config.operating_mode,
            )
        })
    })
    .ok_or_else(|| format!("couldn't find ledger for {:?}/{:?}", chain, data.token))?;

    if mode != OperatingMode::Locker {
        return Err(format!(
            "approve tx requested for minter: {:?}/{:?}",
            chain, data.token
        ));
    }

    let request = Eip1559TransactionRequest {
        chain_id,
        nonce: args.nonce,
        max_priority_fee_per_gas: args.fee.max_priority_fee_per_gas,
        max_fee_per_gas: args.fee.max_fee_per_gas,
        gas_limit,
        destination: erc20,
        amount: Wei::ZERO,
        data: call_tx_with_address_and_amount("approve", helper, amount),
        access_list: AccessList::default(),
    };
    let tx_cost: u64 = args
        .fee
        .cost(gas_limit, Percent::from_percent(0))
        .into_inner()
        .try_into()
        .map_err(|err| format!("tx cost does not fit into u64: {}", err))?;
    let signed_tx = sign_tx(
        request,
        ecdsa_key_name,
        data.ecdsa_public_key.clone(),
        data.derivation_path.clone(),
    )
    .await?;
    log!(
        DEBUG,
        "[{:?}]: forwarder signed approve tx: {:?} {}",
        chain,
        data.token,
        amount
    );
    Ok((signed_tx, tx_cost))
}

async fn sign_lock_or_burn_tx(
    chain: EvmChain,
    data: &SigningData,
    args: &SigningArgs,
) -> Result<(SignedEip1559TransactionRequest, u64), String> {
    let ecdsa_key_name = read_state(|s| s.icp.config.ecdsa_key_name.clone());
    let chain_id = read_evm_state(chain, |s| s.chain_id);
    let (helper, gas_limit, mode) = read_evm_state(chain, |s| {
        s.ledger.get(&data.token).map(|x| {
            (
                x.config.logger_address,
                x.config.gas_limit_for_lock_or_burn,
                x.config.operating_mode,
            )
        })
    })
    .ok_or_else(|| format!("couldn't find ledger for {:?}/{:?}", chain, data.token))?;

    let method = match (args.requested_tx, mode) {
        (RequestedTx::Burn, OperatingMode::Minter) => "burn",
        (RequestedTx::Lock, OperatingMode::Locker)
        | (RequestedTx::ApproveAndLock, OperatingMode::Locker) => "lock",
        _ => {
            return Err(format!(
                "invalid combination of requested tx and mode: {:?} {:?}",
                args.requested_tx, mode
            ));
        }
    };

    let nonce = match args.requested_tx {
        RequestedTx::Burn | RequestedTx::Lock => args.nonce,
        RequestedTx::ApproveAndLock => args.nonce.add(TxNonce::ONE, "BUG: overflow in nonce += 1"),
    };

    let request = Eip1559TransactionRequest {
        chain_id,
        nonce,
        max_priority_fee_per_gas: args.fee.max_priority_fee_per_gas,
        max_fee_per_gas: args.fee.max_fee_per_gas,
        gas_limit,
        destination: helper,
        amount: Wei::ZERO,
        data: call_burn_or_lock_tx(method, args.amount, encode_icp_account(data.receiver)),
        access_list: AccessList::default(),
    };

    let tx_cost: u64 = args
        .fee
        .cost(gas_limit, Percent::from_percent(0))
        .into_inner()
        .try_into()
        .map_err(|err| format!("tx cost does not fit into u64: {}", err))?;
    let signed_tx = sign_tx(
        request,
        ecdsa_key_name,
        data.ecdsa_public_key.clone(),
        data.derivation_path.clone(),
    )
    .await?;

    log!(
        DEBUG,
        "[{:?}]: forwarder signed {} tx: {} {:?}",
        chain,
        method,
        args.amount,
        data.token,
    );
    Ok((signed_tx, tx_cost))
}

async fn sign_tx(
    request: Eip1559TransactionRequest,
    ecdsa_key_name: String,
    ecdsa_public_key: PublicKey,
    derivation_path: DerivationPath,
) -> Result<SignedEip1559TransactionRequest, String> {
    let hash = request.hash();
    assert!(
        !derivation_path.get().is_empty(),
        "BUG: forwarder derivation path is empty"
    );
    let raw_signature = crate::management::sign_with_ecdsa(ecdsa_key_name, derivation_path, hash)
        .await
        .map_err(|e| format!("failed to sign tx: {}", e))?;
    let signature = wrap_signature(raw_signature, hash, ecdsa_public_key)?;
    let tx = SignedEip1559TransactionRequest::new(request, signature);
    Ok(tx)
}
