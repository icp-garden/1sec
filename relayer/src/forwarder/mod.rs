use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque},
    sync::Arc,
    time::Duration,
};

use alloy::{
    hex::FromHex,
    network::Network,
    primitives::{Address, FixedBytes, TxKind, U256},
    providers::{
        fillers::{FillProvider, TxFiller},
        PendingTransactionBuilder, Provider, ProviderBuilder,
    },
    rpc::types::TransactionRequest,
    signers::{k256::ecdsa::SigningKey, local::LocalSigner},
    sol,
    transports::{RpcError, TransportErrorKind},
};
use alloy_consensus::TxEnvelope;
use alloy_eips::BlockNumberOrTag;
use alloy_rlp::Decodable;
use candid::{decode_one, Encode, Nat, Principal};
use evm_rpc_types::Nat256;
use eyre::eyre;
use ic_agent::{export::reqwest::Url, Agent};
use num_traits::ToPrimitive;
use one_sec::{
    api::types::{
        Chain, EvmChain, EvmTx, ForwardedTx, ForwardingAccount, ForwardingBalance,
        ForwardingUpdate, Metadata, RequestedTx, SignedForwardingTx, Token, TransferFee,
        UnsignedForwardingTx,
    },
    config::OperatingMode,
    evm::{
        writer::{fee_history_args, get_fee_from_history},
        TxFee,
    },
    numeric::Percent,
};
use regex::Regex;
use tokio::time::Instant;

const PAYMENT_DELAY: Duration = Duration::from_secs(5);
const MAX_PAYMENT_DELAY: Duration = Duration::from_secs(60);
const RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct Unconfirmed {
    time: Instant,
    step: Duration,
    account: ForwardingAccount,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord)]
struct Forwarding {
    time: Instant,
    account: ForwardingAccount,
    amount: u128,
}

pub async fn forward(
    identity: Arc<dyn ic_agent::Identity>,
    canister_id: Principal,
    icp_url: String,
    chain: EvmChain,
    rpc_url: Url,
    signer: LocalSigner<SigningKey>,
) -> Result<(), eyre::Error> {
    let agent = Agent::builder()
        .with_arc_identity(identity)
        .with_url(&icp_url)
        .build()?;

    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let relayer_address = signer.address();

    let provider = ProviderBuilder::new()
        .network::<alloy::network::Ethereum>()
        .wallet(signer)
        .on_http(rpc_url);

    let tokens: Vec<_> = loop {
        match agent
            .query(&canister_id, "get_metadata")
            .with_arg(Encode!(&())?)
            .await
        {
            Ok(result) => {
                let metadata: Result<Metadata, String> = decode_one(&result).unwrap();
                break metadata
                    .unwrap()
                    .tokens
                    .into_iter()
                    .filter(|c| c.chain == Some(chain.into()))
                    .collect();
            }
            Err(err) => {
                println!("Error in get_metadata: {}", err);
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
        };
    };

    let contracts: BTreeMap<Token, Address> = tokens
        .iter()
        .filter(|&t| t.chain == Some(chain.into()))
        .cloned()
        .map(|t| (t.token.unwrap(), Address::from_hex(t.contract).unwrap()))
        .collect();

    let mode_and_locker: BTreeMap<Token, (OperatingMode, Option<Address>)> = tokens
        .iter()
        .filter(|&t| t.chain == Some(chain.into()))
        .cloned()
        .map(|t| {
            (
                t.token.unwrap(),
                match t.locker {
                    None => (OperatingMode::Minter, None),
                    Some(locker) => (
                        OperatingMode::Locker,
                        Some(Address::from_hex(locker).unwrap()),
                    ),
                },
            )
        })
        .collect();

    let fees: Vec<_> = loop {
        match agent
            .query(&canister_id, "get_transfer_fees")
            .with_arg(Encode!(&())?)
            .await
        {
            Ok(result) => {
                let fees: Vec<TransferFee> = decode_one(&result).unwrap();
                break fees;
            }
            Err(err) => {
                println!("Error in get_transfer_fees: {}", err);
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
        };
    };

    let limits: BTreeMap<Token, (u128, u128)> = fees
        .iter()
        .filter(|&f| {
            f.source_chain == Some(chain.into()) && f.destination_chain == Some(Chain::ICP)
        })
        .cloned()
        .map(|f| {
            (
                f.source_token.unwrap(),
                (
                    f.min_amount.0.to_u128().unwrap(),
                    f.max_amount.0.to_u128().unwrap(),
                ),
            )
        })
        .collect();

    let mut unconfirmed_queue: BinaryHeap<Reverse<Unconfirmed>> = BinaryHeap::new();
    let mut unconfirmed_set: BTreeSet<(Token, String)> = Default::default();

    let mut forwarding_queue: VecDeque<Forwarding> = Default::default();
    let mut forwarding_set: BTreeSet<(Token, String)> = Default::default();

    let mut balances: BTreeMap<(Token, String), u128> = Default::default();

    let mut last_update = std::time::Instant::now();

    loop {
        let mut update = ForwardingUpdate {
            chain,
            balances: vec![],
            to_sign: vec![],
            forwarded: vec![],
        };

        match agent
            .query(&canister_id, "get_forwarding_accounts")
            .with_arg(Encode!(&chain, &0_u64, &1000_u64)?)
            .await
        {
            Ok(result) => {
                let accounts: Vec<ForwardingAccount> = decode_one(&result).unwrap();
                for account in accounts {
                    let key = (account.token, account.address.clone());
                    if !unconfirmed_set.contains(&key) {
                        unconfirmed_set.insert(key);
                        println!(
                            "{} {:?}: forwarder is tracking {}",
                            chrono::Local::now().format("%m-%d %H:%M:%S"),
                            chain,
                            account.address
                        );
                        let step = PAYMENT_DELAY;
                        unconfirmed_queue.push(Reverse(Unconfirmed {
                            time: Instant::now().checked_add(step).unwrap(),
                            step,
                            account,
                        }));
                    }
                }
            }
            Err(err) => {
                println!("Error in get_forwarding_accounts: {}", err);
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
        }

        while let Some(x) = unconfirmed_queue.peek() {
            if x.0.time > Instant::now() {
                break;
            }
            let x = unconfirmed_queue.pop().unwrap().0;
            let token = x.account.token;
            let address = Address::from_hex(&x.account.address).unwrap();
            let step = x.step;

            if forwarding_set.contains(&(token, x.account.address.clone())) {
                unconfirmed_set.remove(&(token, x.account.address.clone()));
                continue;
            }

            let erc20 = ERC20::new(*contracts.get(&token).unwrap(), &provider);
            match erc20.balanceOf(address).call().await {
                Ok(balance) => {
                    let balance: u128 = balance._0.try_into()?;
                    let key = (token, address.to_string());
                    if balance != balances.get(&key).cloned().unwrap_or_default() {
                        update.balances.push(ForwardingBalance {
                            token,
                            address: address.to_string(),
                            balance: balance.into(),
                        });
                    }
                    let (min_amount, max_amount) = limits.get(&token).unwrap();
                    if balance >= *min_amount {
                        println!(
                            "{} {:?}: forwarder fetched balance of {}: {}",
                            chrono::Local::now().format("%m-%d %H:%M:%S"),
                            chain,
                            address,
                            balance
                        );
                        unconfirmed_set.remove(&(token, x.account.address.clone()));
                        forwarding_set.insert((token, x.account.address.clone()));
                        forwarding_queue.push_back(Forwarding {
                            time: Instant::now(),
                            account: x.account,
                            amount: balance.min(*max_amount),
                        });
                    } else {
                        println!(
                            "{} {:?}: forwarder balance of {} is too low: {} vs {}",
                            chrono::Local::now().format("%m-%d %H:%M:%S"),
                            chain,
                            address,
                            balance,
                            *min_amount,
                        );
                        unconfirmed_queue.push(Reverse(Unconfirmed {
                            time: Instant::now().checked_add(step).unwrap(),
                            step: step
                                .checked_add(step.checked_div(2).unwrap())
                                .unwrap()
                                .min(MAX_PAYMENT_DELAY),
                            account: x.account,
                        }));
                    }
                }
                Err(err) => {
                    unconfirmed_queue.push(Reverse(Unconfirmed {
                        time: Instant::now().checked_add(step).unwrap(),
                        step: step
                            .checked_add(step.checked_div(2).unwrap())
                            .unwrap()
                            .min(MAX_PAYMENT_DELAY),
                        account: x.account,
                    }));
                    println!("failed to get balance of {}: {}", address, err);
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
            if update.balances.len() > 100 {
                break;
            }
        }

        let fee = match fetch_fee(&provider).await {
            Ok(fee) => fee,
            Err(err) => {
                println!("failed to fetch fee history: {}", err);
                tokio::time::sleep(RETRY_DELAY).await;
                continue;
            }
        };

        while let Some(x) = forwarding_queue.front() {
            if x.time > Instant::now() {
                break;
            }
            let x = forwarding_queue.pop_front().unwrap();
            let token = x.account.token;
            let address = Address::from_hex(&x.account.address).unwrap();

            let erc20 = ERC20::new(*contracts.get(&token).unwrap(), &provider);
            match erc20.balanceOf(address).call().await {
                Ok(balance) => {
                    let balance: u128 = balance._0.try_into()?;
                    let (min_amount, _max_amount) = limits.get(&token).unwrap();
                    if balance < *min_amount {
                        forwarding_set.remove(&(token, address.to_string().clone()));
                        continue;
                    }
                }
                Err(err) => {
                    println!("failed to fetch balance of {}: {}", address, err);
                    forwarding_queue.push_back(Forwarding {
                        time: Instant::now().checked_add(Duration::from_secs(10)).unwrap(),
                        account: x.account,
                        amount: x.amount,
                    });
                    tokio::time::sleep(RETRY_DELAY).await;
                    continue;
                }
            }

            let requested_tx = match mode_and_locker.get(&token).unwrap() {
                (OperatingMode::Minter, _) => RequestedTx::Burn,
                (OperatingMode::Locker, locker) => {
                    let allowance = erc20.allowance(address, locker.unwrap()).call().await;
                    match allowance {
                        Ok(allowance) => {
                            let allowance: u128 = allowance._0.try_into().unwrap_or_default();
                            if allowance >= x.amount {
                                RequestedTx::Lock
                            } else {
                                RequestedTx::ApproveAndLock
                            }
                        }
                        Err(_) => RequestedTx::ApproveAndLock,
                    }
                }
            };
            let nonce = match provider.get_transaction_count(address).await {
                Ok(nonce) => nonce,
                Err(err) => {
                    println!("failed to fetch nonce of {}: {}", address, err);
                    forwarding_queue.push_back(Forwarding {
                        time: Instant::now().checked_add(RETRY_DELAY).unwrap(),
                        account: x.account,
                        amount: x.amount,
                    });
                    continue;
                }
            };
            let bumped_fee = fee.bump(Percent::from_percent(20));

            println!(
                "{} {:?}: forwarder requesting to sign {:?} of {:?} from {} at nonce={}, max_fee_per_gas={}, max_priority_fee_per_gas={}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                requested_tx,
                token,
                x.account.address,
                nonce,
                bumped_fee.max_fee_per_gas,
                bumped_fee.max_priority_fee_per_gas
            );

            update.to_sign.push(UnsignedForwardingTx {
                token: x.account.token,
                address: x.account.address.clone(),
                receiver: x.account.receiver.clone(),
                amount: x.amount.into(),
                nonce,
                max_fee_per_gas: bumped_fee.max_fee_per_gas.into_inner() as u64,
                max_priority_fee_per_gas: bumped_fee.max_priority_fee_per_gas.into_inner() as u64,
                requested_tx,
            });

            forwarding_queue.push_back(Forwarding {
                time: Instant::now().checked_add(Duration::from_secs(30)).unwrap(),
                account: x.account,
                amount: x.amount,
            });

            if update.to_sign.len() > 100 {
                break;
            }
        }

        match agent
            .query(&canister_id, "get_forwarding_transactions")
            .with_arg(Encode!(&chain).unwrap())
            .await
        {
            Ok(result) => {
                let txs: Vec<SignedForwardingTx> = decode_one(&result).unwrap();
                if !txs.is_empty() {
                    println!(
                        "{} {:?}: forwarder received {} tx to send",
                        chrono::Local::now().format("%m-%d %H:%M:%S"),
                        chain,
                        txs.len()
                    );
                }
                let txs = dedup_by_address_and_nonce(txs);
                if !txs.is_empty() {
                    println!(
                        "{} {:?}: forwarder deduplicated to {} tx to send",
                        chrono::Local::now().format("%m-%d %H:%M:%S"),
                        chain,
                        txs.len()
                    );
                }
                for tx in txs {
                    match submit_tx(chain, &provider, tx.clone(), relayer_address).await {
                        Ok((nonce, tx_hash)) => {
                            update.forwarded.push(ForwardedTx {
                                token: tx.token,
                                address: tx.address,
                                receiver: tx.receiver,
                                nonce,
                                total_tx_cost_in_wei: tx.total_tx_cost_in_wei,
                                lock_or_burn_tx: EvmTx {
                                    hash: tx_hash,
                                    log_index: None,
                                },
                            });
                        }
                        Err(err) => {
                            println!(
                                "{} {:?}: forwarder failed to submit tx: {}",
                                chrono::Local::now().format("%m-%d %H:%M:%S"),
                                chain,
                                err
                            );
                            tokio::time::sleep(RETRY_DELAY).await;
                        }
                    }
                }
            }
            Err(err) => {
                println!("Error in get_forwarding_transactions: {}", err);
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }

        if !update.balances.is_empty()
            || !update.to_sign.is_empty()
            || !update.forwarded.is_empty()
            || last_update.elapsed() > Duration::from_secs(5 * 60)
        {
            loop {
                match agent
                    .update(&canister_id, "submit_forwarding_update")
                    .with_arg(Encode!(&update).unwrap())
                    .await
                {
                    Ok(result) => {
                        last_update = std::time::Instant::now();
                        let result: Result<(), String> = decode_one(&result).unwrap();
                        match result {
                            Ok(()) => {
                                for b in update.balances {
                                    balances.insert(
                                        (b.token, b.address),
                                        b.balance.0.try_into().unwrap(),
                                    );
                                }
                            }
                            Err(err) => {
                                println!("Error in submit_forwarding_update: {}", err);
                                tokio::time::sleep(RETRY_DELAY).await;
                            }
                        }
                        break;
                    }
                    Err(err) => {
                        println!("Error in submit_forwarding_update: {} (will retry)", err);
                        tokio::time::sleep(RETRY_DELAY).await;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(10000)).await;
    }
}

async fn submit_tx<F, P>(
    chain: EvmChain,
    provider: &FillProvider<F, P, alloy::network::Ethereum>,
    tx: SignedForwardingTx,
    relayer_address: Address,
) -> Result<(u64, String), eyre::Error>
where
    F: TxFiller<alloy::network::Ethereum>,
    P: Provider<alloy::network::Ethereum>,
{
    let address = Address::from_hex(&tx.address).unwrap();
    let nonce = provider.get_transaction_count(address).await?;
    let next_nonce = match tx.approve_tx {
        Some(_) => tx.nonce + 2,
        None => tx.nonce + 1,
    };
    if nonce >= next_nonce {
        // Nonce already consumed on-chain. Compute the tx hash from the raw
        // bytes and verify it exists on-chain before reporting it.
        let tx_hash = decode_tx_hash(&tx.lock_or_burn_tx.bytes)?;
        let receipt = provider.get_transaction_receipt(tx_hash).await?;
        let tx_hash = match receipt {
            Some(_) => tx_hash.to_string(),
            None => {
                // The original tx was never mined — the nonce was consumed
                // by a different transaction (e.g. after a crash/restart).
                // Use a zero hash so the signed queue gets cleared without
                // storing a bogus hash in the forwarded map.
                println!(
                    "{} {:?} forwarder: tx nonce too low ({} >= {}), original tx NOT found on-chain: {}",
                    chrono::Local::now().format("%m-%d %H:%M:%S"),
                    chain,
                    nonce,
                    next_nonce,
                    tx_hash,
                );
                format!("{:#066x}", 0)
            }
        };
        println!(
            "{} {:?} forwarder: tx nonce too low ({} >= {}), clearing with hash: {}",
            chrono::Local::now().format("%m-%d %H:%M:%S"),
            chain,
            nonce,
            next_nonce,
            tx_hash,
        );
        return Ok((tx.nonce, tx_hash));
    }

    let balance = provider.get_balance(address).await?;

    let cost = U256::from(tx.total_tx_cost_in_wei);

    if balance < cost {
        top_up_balance(chain, provider, relayer_address, address, cost - balance).await?;
    }

    if let Some(approve_tx) = tx.approve_tx {
        if nonce == tx.nonce {
            let result = provider.send_raw_transaction(&approve_tx.bytes).await;
            let result =
                top_up_balance_if_needed(chain, provider, relayer_address, address, cost, result)
                    .await?;
            println!(
                "{} {:?} forwarder awaiting for approve tx tx: {}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                result.tx_hash(),
            );
            let status = result.get_receipt().await?;
            if !status.status() {
                return Err(eyre!("approve tx has failed: {} {:?}", address, tx.token));
            }
            println!(
                "{} {:?} forwarder submitted approve tx: {}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                status.transaction_hash
            );
        } else {
            // This case may happen if approve has already succeeded, but
            // lock/burn has failed for some reason.
            // The we just need to try the lock/burn below.
            println!(
                "{} {:?} forwarder skipping approve tx due to nonce: {} vs {}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                nonce,
                tx.nonce,
            );
        }
    }

    let result = provider
        .send_raw_transaction(&tx.lock_or_burn_tx.bytes)
        .await;

    let result =
        top_up_balance_if_needed(chain, provider, relayer_address, address, cost, result).await?;

    println!(
        "{} {:?} forwarder awaiting for receipt of lock/burn tx: {}",
        chrono::Local::now().format("%m-%d %H:%M:%S"),
        chain,
        result.tx_hash(),
    );

    let status = result.get_receipt().await?;

    if !status.status() {
        return Err(eyre!("lock/burn tx has failed: {} {:?}", address, tx.token));
    }

    println!(
        "{} {:?} forwarder submitted lock/burn tx: {}",
        chrono::Local::now().format("%m-%d %H:%M:%S"),
        chain,
        status.transaction_hash
    );

    Ok((tx.nonce, status.transaction_hash.to_string()))
}

async fn top_up_balance<F, P>(
    chain: EvmChain,
    provider: &FillProvider<F, P, alloy::network::Ethereum>,
    relayer_address: Address,
    receiver_address: Address,
    amount: U256,
) -> Result<(), eyre::Error>
where
    F: TxFiller<alloy::network::Ethereum>,
    P: Provider<alloy::network::Ethereum>,
{
    let signer_balance = provider.get_balance(relayer_address).await?;
    if signer_balance < amount {
        return Err(eyre!(
            "insufficient eth balance of relayer: need at least {} and have {}",
            amount,
            signer_balance
        ));
    }
    let tx = TransactionRequest {
        from: Some(relayer_address),
        to: Some(TxKind::Call(receiver_address)),
        value: Some(amount),
        ..Default::default()
    };
    let result = provider.send_transaction(tx).await?;
    let status = result.get_receipt().await?;
    if !status.status() {
        return Err(eyre!(
            "failed to transfer {} wei to {}",
            amount,
            receiver_address
        ));
    }
    println!(
        "{} {:?} forwarder transferred {} wei to {} tx: {}",
        chrono::Local::now().format("%m-%d %H:%M:%S"),
        chain,
        amount,
        receiver_address,
        status.transaction_hash
    );
    Ok(())
}

async fn top_up_balance_if_needed<F, P>(
    chain: EvmChain,
    provider: &FillProvider<F, P, alloy::network::Ethereum>,
    relayer_address: Address,
    receiver_address: Address,
    cost: U256,
    result: Result<
        PendingTransactionBuilder<alloy::network::Ethereum>,
        RpcError<TransportErrorKind>,
    >,
) -> Result<PendingTransactionBuilder<alloy::network::Ethereum>, RpcError<TransportErrorKind>>
where
    F: TxFiller<alloy::network::Ethereum>,
    P: Provider<alloy::network::Ethereum>,
{
    match result {
        Ok(result) => Ok(result),
        Err(error) => match &error {
            RpcError::ErrorResp(_) => {
                let msg = format!("{}", error);
                if msg.contains("insufficient funds for gas * price + value") {
                    if let Some((have, want)) = parse_insufficient_funds(&msg) {
                        if want > have && want - have < cost {
                            let _ = top_up_balance(
                                chain,
                                provider,
                                relayer_address,
                                receiver_address,
                                want - have,
                            )
                            .await;
                        }
                    }
                }
                Err(error)
            }
            _ => Err(error),
        },
    }
}

fn parse_insufficient_funds(input: &str) -> Option<(U256, U256)> {
    let re = Regex::new(r"(have|balance)\s+(\d+),?\s+(want|tx cost)\s+(\d+)").ok()?;
    let captures = re.captures(input)?;
    let have = captures.get(2)?.as_str().parse::<u128>().ok()?;
    let want = captures.get(4)?.as_str().parse::<u128>().ok()?;
    Some((U256::from(have), U256::from(want)))
}

async fn fetch_fee<F, P, N>(provider: &FillProvider<F, P, N>) -> Result<TxFee, eyre::Error>
where
    F: TxFiller<N>,
    P: Provider<N>,
    N: Network,
{
    let args = fee_history_args(evm_rpc_types::BlockTag::Latest);

    let block_count = Nat::from(args.block_count).0.to_u64().unwrap_or_default();
    let reward_percentiles: Vec<_> = args
        .reward_percentiles
        .unwrap_or_default()
        .into_iter()
        .map(|x| x as f64)
        .collect();

    let fee_history = provider
        .get_fee_history(block_count, BlockNumberOrTag::Latest, &reward_percentiles)
        .await?;

    let fee_history = evm_rpc_types::FeeHistory {
        oldest_block: Nat256::from(fee_history.oldest_block),
        base_fee_per_gas: fee_history
            .base_fee_per_gas
            .into_iter()
            .map(Nat256::from)
            .collect(),
        gas_used_ratio: fee_history.gas_used_ratio,
        reward: fee_history
            .reward
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.into_iter().map(Nat256::from).collect())
            .collect(),
    };
    let fee = get_fee_from_history(fee_history).map_err(|err| eyre!(err))?;
    Ok(fee)
}

fn dedup_by_address_and_nonce(txs: Vec<SignedForwardingTx>) -> Vec<SignedForwardingTx> {
    let mut group = BTreeMap::new();
    for tx in txs {
        let entry = group
            .entry((tx.address.clone(), tx.nonce))
            .or_insert(tx.clone());
        if entry.total_tx_cost_in_wei < tx.total_tx_cost_in_wei {
            let _ = std::mem::replace(entry, tx);
        }
    }
    group.values().cloned().into_iter().collect()
}

sol!(
    #[allow(missing_docs)]
    #[allow(clippy::too_many_arguments)]
    #[sol(rpc)]
    ERC20,
    "../contracts/evm/out/Token.sol/Token.json"
);

fn decode_tx_hash(mut buf: &[u8]) -> Result<FixedBytes<32>, eyre::Error> {
    let decode_tx = TxEnvelope::decode(&mut buf)?;
    Ok(*decode_tx.tx_hash())
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;

    use crate::forwarder::parse_insufficient_funds;

    #[test]
    fn test_parsing_of_insufficient_funds() {
        let (have, want) = parse_insufficient_funds("09-08 16:39:09 Base: forwarder failed to submit tx: server returned an error response: error code -32000: insufficient funds for gas * price + value: balance 1699006000000, tx cost 1702313919518, overshot 3307919518").unwrap();
        assert_eq!(have, U256::from(1699006000000_u128));
        assert_eq!(want, U256::from(1702313919518_u128));

        let (have, want) = parse_insufficient_funds("09-08 16:39:09 Base: forwarder failed to submit tx: server returned an error response: error code -32000: insufficient funds for gas * price + value: have 1699006000000 want 1702313919518, overshot 3307919518").unwrap();
        assert_eq!(have, U256::from(1699006000000_u128));
        assert_eq!(want, U256::from(1702313919518_u128));
    }
}
