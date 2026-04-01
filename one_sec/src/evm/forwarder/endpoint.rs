use std::{cmp::Ordering, str::FromStr};

use ic_canister_log::log;
use ic_ethereum_types::Address;
use ic_management_canister_types_private::DerivationPath;
use ic_secp256k1::{DerivationIndex, PublicKey};

use crate::{
    api::{
        self,
        types::{
            EvmChain, EvmTx, ForwardEvmToIcpArg, ForwardedTx, ForwardingAccount,
            ForwardingResponse, ForwardingStatus, ForwardingUpdate, SignedForwardingTx, Token,
            TransferId, UnsignedForwardingTx, RLP,
        },
    },
    evm::{
        derive_address_from_public_key,
        forwarder::{
            state::{Forwarded, ForwardingAddress, Signing, SigningArgs, SigningData, Unconfirmed},
            Task,
        },
        state::{mutate_evm_state, read_evm_state},
        TxFee, TxHash,
    },
    flow::event::Direction,
    icp::IcpAccount,
    logs::DEBUG,
    numeric::{Amount, Timestamp, TxNonce, WeiPerGas},
    state::read_state,
    task::{schedule_now, timestamp_ms},
};

pub fn derivation_path(receiver: &IcpAccount) -> ic_secp256k1::DerivationPath {
    const ICRC_TAG: u8 = 1;
    const ACCOUNT_ID_TAG: u8 = 2;
    match receiver {
        IcpAccount::ICRC(account) => ic_secp256k1::DerivationPath::new(vec![
            DerivationIndex(ICRC_TAG.to_be_bytes().to_vec()),
            DerivationIndex(account.owner.as_slice().to_vec()),
            DerivationIndex(account.effective_subaccount().to_vec()),
        ]),
        IcpAccount::AccountId(account) => ic_secp256k1::DerivationPath::new(vec![
            DerivationIndex(ACCOUNT_ID_TAG.to_be_bytes().to_vec()),
            DerivationIndex(account.as_bytes().to_vec()),
        ]),
    }
}

#[derive(Debug, Clone)]
struct ValidatedForwardingAccount {
    address: ForwardingAddress,
    receiver: IcpAccount,
    ecdsa_public_key: PublicKey,
    derivation_path: DerivationPath,
}

fn validate_forwarding_arg(
    token: Token,
    address: String,
    receiver: api::types::IcpAccount,
) -> Result<ValidatedForwardingAccount, String> {
    let receiver = receiver.try_into()?;
    let address = Address::from_str(&address)?;
    let fa = ForwardingAddress { token, address };
    let dp = derivation_path(&receiver);
    let main_public_key = read_state(|s| s.icp.ecdsa_public_key.clone())
        .ok_or("main public key is not fetched yet")?;
    let chain_code =
        read_state(|s| s.icp.chain_code.clone()).ok_or("chain code is not fetched yet")?;
    let (ecdsa_public_key, _) =
        main_public_key.derive_subkey_with_chain_code(&dp, &chain_code.try_into().unwrap());
    let actual_address = derive_address_from_public_key(&ecdsa_public_key);
    if address != actual_address {
        return Err(format!("invalid forwarding address: {}", address));
    }
    Ok(ValidatedForwardingAccount {
        address: fa,
        receiver,
        ecdsa_public_key,
        derivation_path: DerivationPath::new(
            dp.path().iter().map(|x| x.clone().0.into()).collect(),
        ),
    })
}

pub fn forward_evm_to_icp(arg: ForwardEvmToIcpArg) -> Result<ForwardingResponse, String> {
    let chain = arg.chain;
    let validated = validate_forwarding_arg(arg.token, arg.address, arg.receiver)?;
    let fa = validated.address;

    let Some(min_amount) = read_state(|s| {
        s.flow
            .config
            .get(&(Direction::EvmToIcp, fa.token, chain, fa.token))
            .map(|c| c.min_amount)
    }) else {
        return Err(format!(
            "transfers of {:?} are not supported from {:?}",
            fa.token, chain
        ));
    };

    let last_transfer = read_state(|s| {
        let flow_ids = s.flow.flow_by_evm_account.get(&fa.address)?;
        for flow_id in flow_ids.iter().rev().take(16) {
            if let Some(flow) = s.flow.flow.get(flow_id) {
                if flow.input.direction == Direction::EvmToIcp
                    && flow.input.evm_chain == chain
                    && flow.input.evm_token == fa.token
                {
                    return Some(*flow_id);
                }
            }
        }
        None
    });

    let status = read_evm_state(chain, |s| {
        if let Some(forwarded) = s.forwarder.forwarded.get(&fa) {
            if let Some(x) = forwarded.last() {
                return ForwardingStatus::Forwarded(EvmTx {
                    hash: x.lock_or_burn_tx.to_string(),
                    log_index: None,
                });
            }
        }

        if s.forwarder.signed.contains_key(&fa) || s.forwarder.signing_map.contains_key(&fa) {
            return ForwardingStatus::Forwarding;
        }

        if let Some(balance) = s.forwarder.balance.get(&fa).cloned() {
            if balance < min_amount && balance > Amount::ZERO {
                return ForwardingStatus::LowBalance {
                    balance: balance.into(),
                    min_amount: min_amount.into(),
                };
            }
        }

        ForwardingStatus::CheckingBalance
    });

    let in_unconfirmed_set = read_evm_state(chain, |s| s.forwarder.unconfirmed_set.contains(&fa));

    if in_unconfirmed_set {
        Ok(ForwardingResponse {
            done: last_transfer.map(|id| TransferId {
                id: id.into_inner(),
            }),
            status: Some(status),
        })
    } else {
        log!(
            DEBUG,
            "[{:?}]: forward_evm_to_icp: {:?} {} {:?}",
            chain,
            fa.token,
            fa.address,
            validated.receiver,
        );

        let receiver = validated.receiver;
        mutate_evm_state(chain, |s| {
            s.forwarder.unconfirmed_set.insert(fa.clone());
            s.forwarder.unconfirmed_queue.push_back(Unconfirmed {
                address: fa,
                receiver,
                time: timestamp_ms(),
            });
            while s.forwarder.unconfirmed_queue.len() > s.forwarder.config.max_pending_count {
                if let Some(x) = s.forwarder.unconfirmed_queue.pop_front() {
                    s.forwarder.unconfirmed_set.remove(&x.address);
                } else {
                    break;
                }
            }
        });
        Ok(ForwardingResponse {
            done: last_transfer.map(|id| TransferId {
                id: id.into_inner(),
            }),
            status: Some(ForwardingStatus::CheckingBalance),
        })
    }
}

pub fn get_forwarding_address(receiver: IcpAccount) -> Result<Address, String> {
    let dp = derivation_path(&receiver);
    let main_public_key = read_state(|s| s.icp.ecdsa_public_key.clone())
        .ok_or("main public key is not fetched yet")?;
    let chain_code =
        read_state(|s| s.icp.chain_code.clone()).ok_or("chain code is not fetched yet")?;
    let (ecdsa_public_key, _) =
        main_public_key.derive_subkey_with_chain_code(&dp, &chain_code.try_into().unwrap());
    let address = derive_address_from_public_key(&ecdsa_public_key);
    Ok(address)
}

pub fn get_forwarding_accounts(chain: EvmChain, skip: u64, count: u64) -> Vec<ForwardingAccount> {
    read_evm_state(chain, |s| {
        s.forwarder
            .unconfirmed_queue
            .iter()
            .skip(skip as usize)
            .take(count as usize)
            .map(|x| ForwardingAccount {
                chain,
                token: x.address.token,
                address: x.address.address.to_string(),
                receiver: x.receiver.into(),
            })
            .collect()
    })
}

pub fn submit_forwarding_update(arg: ForwardingUpdate) -> Result<(), String> {
    let chain = arg.chain;

    let balances = arg.balances;
    mutate_evm_state(chain, |s| {
        for balance in balances {
            let address = Address::from_str(&balance.address)?;
            let amount = Amount::try_from(balance.balance)?;
            let fa = ForwardingAddress {
                token: balance.token,
                address,
            };
            s.forwarder.balance.insert(fa, amount);
        }
        Ok::<(), String>(())
    })?;

    for unsigned_tx in arg.to_sign {
        add_unsigned_forwarding_tx(chain, unsigned_tx)?;
    }

    for forwarded in arg.forwarded {
        add_forwarded_tx(chain, forwarded)?;
    }

    if !read_evm_state(chain, |s| s.forwarder.signing_queue.is_empty()) {
        schedule_now(
            Task::SignTx.wrap(chain),
            "sign forwarding transaction".into(),
        );
    }

    let now = timestamp_ms();
    mutate_evm_state(chain, |s| {
        while let Some(x) = s.forwarder.unconfirmed_queue.front() {
            let expiration = x.time.add(
                Timestamp::new(s.forwarder.config.request_expiry.as_millis() as u64),
                "BUG: overflow in x.time += request_expiry",
            );
            if expiration > now {
                break;
            }
            s.forwarder.unconfirmed_set.remove(&x.address);
            s.forwarder.unconfirmed_queue.pop_front();
        }
    });

    Ok(())
}

fn add_unsigned_forwarding_tx(chain: EvmChain, tx: UnsignedForwardingTx) -> Result<(), String> {
    let validated = validate_forwarding_arg(tx.token, tx.address, tx.receiver)?;
    let fa = validated.address.clone();

    let amount: Amount = tx
        .amount
        .try_into()
        .map_err(|err| format!("invalid amount: {}", err))?;

    mutate_evm_state(chain, |s| {
        if !s.forwarder.signing_map.contains_key(&fa) {
            s.forwarder.signing_queue.push_back(fa.clone());
        }

        let new = SigningArgs {
            nonce: TxNonce::new(tx.nonce),
            amount,
            fee: TxFee {
                max_fee_per_gas: WeiPerGas::new(tx.max_fee_per_gas as u128),
                max_priority_fee_per_gas: WeiPerGas::new(tx.max_priority_fee_per_gas as u128),
            },
            requested_tx: tx.requested_tx,
        };

        let expected = validated.clone();

        let signing = s
            .forwarder
            .signing_map
            .entry(fa)
            .or_insert_with(|| Signing {
                data: SigningData {
                    token: validated.address.token,
                    sender: validated.address.address,
                    receiver: validated.receiver,
                    ecdsa_public_key: validated.ecdsa_public_key,
                    derivation_path: validated.derivation_path,
                },
                args: Default::default(),
            });

        assert_eq!(signing.data.token, expected.address.token);
        assert_eq!(signing.data.sender, expected.address.address);
        assert_eq!(signing.data.receiver, expected.receiver);
        assert_eq!(signing.data.derivation_path, expected.derivation_path);
        assert_eq!(signing.data.ecdsa_public_key, expected.ecdsa_public_key);

        let queue = signing.args.entry(new.nonce).or_default();

        if let Some(old) = queue.pop_back() {
            match new.cmp(&old) {
                Ordering::Equal | Ordering::Less => {
                    queue.push_back(old);
                }
                Ordering::Greater => {
                    queue.push_back(new);
                }
            }
        } else {
            queue.push_back(new);
        }
    });

    Ok(())
}

pub fn add_forwarded_tx(chain: EvmChain, forwarded: ForwardedTx) -> Result<(), String> {
    let nonce = forwarded.nonce;
    let validated =
        validate_forwarding_arg(forwarded.token, forwarded.address, forwarded.receiver)?;
    let tx_hash = TxHash::from_str(&forwarded.lock_or_burn_tx.hash)?;
    let fa = validated.address;

    log!(
        DEBUG,
        "[{:?}]: add_forwarded_tx: {:?} {} {:?} {}",
        chain,
        fa.token,
        fa.address,
        validated.receiver,
        tx_hash,
    );

    mutate_evm_state(chain, |s| {
        s.forwarder
            .forwarded
            .entry(fa.clone())
            .or_default()
            .push(Forwarded {
                nonce,
                total_tx_cost_in_wei: forwarded.total_tx_cost_in_wei,
                lock_or_burn_tx: tx_hash,
            });

        if let Some(txs) = s.forwarder.signed.get_mut(&fa) {
            txs.retain(|x| x.nonce > nonce);
            if txs.is_empty() {
                s.forwarder.signed.remove(&fa);
            }
        }

        if let Some(signing) = s.forwarder.signing_map.get_mut(&fa) {
            while let Some((n, _)) = signing.args.first_key_value() {
                if n.into_inner() <= nonce {
                    signing.args.pop_first();
                } else {
                    break;
                }
            }
            if signing.args.is_empty() {
                s.forwarder.signing_map.remove(&fa);
                s.forwarder.signing_queue.retain(|x| x != &fa);
            }
        }
    });

    Ok(())
}

pub fn get_forwarding_transactions(chain: EvmChain) -> Vec<SignedForwardingTx> {
    read_evm_state(chain, |s| {
        s.forwarder
            .signed
            .iter()
            .flat_map(|(fa, txs)| {
                txs.iter().map(|signed| SignedForwardingTx {
                    token: fa.token,
                    address: fa.address.to_string(),
                    receiver: signed.receiver.into(),
                    nonce: signed.nonce,
                    total_tx_cost_in_wei: signed.total_tx_cost_in_wei,
                    approve_tx: signed.approve_tx.as_ref().map(|x| RLP { bytes: x.rlp() }),
                    lock_or_burn_tx: RLP {
                        bytes: signed.lock_or_burn_tx.rlp(),
                    },
                })
            })
            .collect()
    })
}
