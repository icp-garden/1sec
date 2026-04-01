use std::time::Duration;

use ::futures::future;
use alloy::{
    hex::FromHex,
    primitives::{keccak256, Address, FixedBytes},
};
use candid::{Nat, Principal};
use evm_rpc_types::Hex32;
use ic_ledger_types::{AccountIdentifier, Subaccount};
use icrc_ledger_types::icrc1::account::Account;
use one_sec::{
    api::types::{
        self, Asset, AssetRequest, Chain, EvmAccount, EvmChain, EvmTx, ForwardEvmToIcpArg,
        ForwardedTx, ForwardingStatus, ForwardingUpdate, Status, Token, TransferArg,
        UnsignedForwardingTx,
    },
    config,
    evm::{self, ledger::encode_icp_account},
    flow::{config::FlowConfig, event::Direction, trace::TraceEvent},
    icp::{self},
    numeric::{Amount, Amount128, Wei, E8S},
};

use crate::{helpers::icp::USDC_LEDGER_FEE, TestEnv, ICP_LEDGER_FEE};
use lazy_static::lazy_static;

lazy_static! {
    static ref CONFIG: config::Config = config::Config::test();
}

fn flow_config(direction: Direction, icp_token: Token, evm_token: Token) -> FlowConfig {
    CONFIG
        .flow
        .flows
        .iter()
        .find(|f| f.direction == direction && f.icp_token == icp_token && f.evm_token == evm_token)
        .unwrap()
        .clone()
}

fn icp_ledger_config(token: Token) -> icp::ledger::Config {
    CONFIG
        .icp
        .ledger
        .iter()
        .find(|c| c.token == token)
        .unwrap()
        .clone()
}

fn icrc_account(user: Principal) -> types::Account {
    types::Account::Icp(types::IcpAccount::ICRC(Account {
        owner: user,
        subaccount: None,
    }))
}

#[tokio::test]
async fn test_deposit_to_evm_ok() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let transfer = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(transfer.source.chain, Some(Chain::ICP));
    assert_eq!(transfer.source.account, Some(icrc_account(test.user)));
    assert_eq!(transfer.source.token, Some(Token::ICP));
    assert_eq!(Amount::try_from(transfer.source.amount).unwrap(), amount);
    assert!(transfer.source.tx.is_some());
    assert_eq!(transfer.destination.chain, Some(Chain::Base));
    assert_eq!(
        transfer
            .destination
            .account
            .unwrap()
            .as_evm()
            .unwrap()
            .address,
        test.evm.user.to_string(),
    );
    assert_eq!(transfer.destination.token, Some(Token::ICP));
    assert_eq!(transfer.destination.tx, None);
    assert_eq!(transfer.status, Some(Status::PendingDestinationTx));

    for _ in 0..100 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let balance2 = test.icp_balance_of(test.user).await;
    assert_eq!(balance2, balance1.sub(amount, ""));

    let transfer = test.get_transfer(test.user, transfer_id).await;
    assert_eq!(transfer.status, Some(Status::Succeeded));
    assert!(transfer.destination.tx.is_some());
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_amount_too_low() {
    let test = TestEnv::new().await;

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: ICP_LEDGER_FEE.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;
    match result {
        types::TransferResponse::Failed(error) => {
            assert!(
                error.error.contains("The amount is too low"),
                "{}",
                error.error
            );
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_amount_too_high() {
    let test = TestEnv::new().await;

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: u64::MAX.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    match result {
        types::TransferResponse::Failed(error) => {
            assert!(
                error.error.contains("The amount is too high"),
                "{}",
                error.error
            );
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_anonymous_caller() {
    let test = TestEnv::new().await;

    let result = test
        .transfer(
            Principal::anonymous(),
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(Principal::anonymous()),
                    token: types::Token::ICP,
                    amount: (10 * E8S).into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;
    match result {
        types::TransferResponse::Failed(error) => {
            assert!(error.error.contains("anonymous"), "{}", error.error);
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_invalid_destination() {
    let test = TestEnv::new().await;

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: (10 * E8S).into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: "0xfoobar".into(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;
    match result {
        types::TransferResponse::Failed(error) => {
            assert!(
                error
                    .error
                    .contains("address is not hex: Invalid string length"),
                "{}",
                error.error
            );
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_insufficient_allowance() {
    let test = TestEnv::new().await;

    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    for _ in 0..40 {
        test.tick().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.add(Amount::ONE, "").into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;
    match result {
        types::TransferResponse::Failed(error) => {
            assert!(error.error.contains("allowance"), "{}", error.error);
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_err_insufficient_funds() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    test.icp_approve(test.user, test.one_sec, balance0.add(Amount::new(10), ""))
        .await;
    let balance1 = test.icp_balance_of(test.user).await;

    for _ in 0..40 {
        test.tick().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: balance0.add(Amount::ONE, "").into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;
    match result {
        types::TransferResponse::Failed(error) => {
            assert!(error.error.contains("balance"), "{}", error.error);
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
    };
    let balance2 = test.icp_balance_of(test.user).await;
    assert_eq!(balance2, balance1);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_withdraw_event() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..100 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let transfer = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(transfer.status, Some(Status::Succeeded));
    assert!(transfer.destination.tx.is_some());

    let tx_receipt = test
        .evm
        .icp_burn(test.evm.icp, Wei::new(42), FixedBytes([1_u8; 32]))
        .await;

    assert!(tx_receipt.status());

    let logs = test
        .evm
        .icp_logs(test.evm.icp, tx_receipt.block_number.unwrap())
        .await;

    // The oICP.burn transaction emits two events:
    // - Transfer (by ERC20)
    // - Burn1

    let transfer = keccak256("Transfer(address,address,uint256)".as_bytes());
    assert_eq!(logs[0].data().topics()[0], transfer);

    let withdraw = keccak256("Burn1(address,uint256,bytes32)".as_bytes());

    assert_eq!(CONFIG.evm[0].ledger[0].logger_topics[0], withdraw.0);
    assert_eq!(logs[1].data().topics()[0], withdraw);
    assert_eq!(logs[1].data().topics().len(), 1);

    // All parameters of the emitted log are zero-extended to 32-bytes.

    let data = logs[1].data().data.to_vec();
    let entry: Vec<_> = data.chunks(32).collect();

    assert_eq!(entry[0][0..12], [0_u8; 12]);
    assert_eq!(entry[0][12..32], test.evm.user.0 .0);

    assert_eq!(entry[1][0..31], [0_u8; 31]);
    assert_eq!(entry[1][31], 42);

    assert_eq!(entry[2][0..32], [1_u8; 32]);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_new_api_deposit_and_withdraw_ok() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..200 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let balance2 = test.icp_balance_of(test.user).await;

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert!(tx.destination.tx.is_some());
    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::SignTx));
    assert_eq!(tx.trace.entries[2].event, Some(TraceEvent::SendTx));
    assert_eq!(
        tx.trace.entries[3].event,
        Some(TraceEvent::PendingConfirmTx)
    );
    assert_eq!(tx.trace.entries[4].event, Some(TraceEvent::ConfirmTx));

    let amount = Amount::new(2 * E8S);

    let tx_receipt = test
        .evm
        .icp_burn(
            test.evm.icp,
            Wei::new(amount.into_inner()),
            FixedBytes(
                encode_icp_account(
                    icrc_account(test.user)
                        .as_icp()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
                .try_into()
                .unwrap(),
            ),
        )
        .await;

    assert!(tx_receipt.status());

    let logs = test
        .evm
        .icp_logs(test.evm.icp, tx_receipt.block_number.unwrap())
        .await;

    // The oICP.burn transaction emits two events:
    // - Transfer (by ERC20)
    // - Burn1

    let transfer = keccak256("Transfer(address,address,uint256)".as_bytes());
    assert_eq!(logs[0].data().topics()[0], transfer);

    let withdraw = keccak256("Burn1(address,uint256,bytes32)".as_bytes());

    assert_eq!(CONFIG.evm[0].ledger[0].logger_topics[0], withdraw.0);
    assert_eq!(logs[1].data().topics()[0], withdraw);
    assert_eq!(logs[1].data().topics().len(), 1);

    let arg = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(EvmTx {
                hash: Hex32::from(tx_receipt.transaction_hash.0).to_string(),
                log_index: None,
            })),
        },
        destination: AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let result = test.transfer(test.user, arg.clone()).await;
    match result {
        types::TransferResponse::Fetching(_) => {}
        types::TransferResponse::Accepted(_) | types::TransferResponse::Failed(_) => {
            unreachable!("unexpected result: {:?}", result)
        }
    }

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..200 {
        test.tick().await;
        test.transfer(test.user, arg.clone()).await;
    }

    let result = test.transfer(test.user, arg).await;
    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
            unreachable!("unexpected result: {:?}", result)
        }
    };

    let withdraw_fee = amount.into_inner() as f64
        * flow_config(Direction::EvmToIcp, types::Token::ICP, types::Token::ICP)
            .fee
            .as_f64();
    let withdraw_fee = Amount::new(withdraw_fee.round() as u128);
    let total_fee = withdraw_fee.add(ICP_LEDGER_FEE, "");
    let withdrawn = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;
    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount {
            address: test.evm.user.to_string()
        }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount));
    assert_eq!(tx.destination.account, Some(icrc_account(test.user)));
    assert_eq!(tx.destination.amount, Nat::from(withdrawn));

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance3 = test.icp_balance_of(test.user).await;

    let credited = balance3.sub(balance2, "");

    assert_eq!(credited, withdrawn);

    let canister_calls = test.get_canister_calls(test.user).await;

    let expected_calls = vec![
        (Principal::management_canister(), "ecdsa_public_key"),
        (Principal::management_canister(), "sign_with_ecdsa"),
        (test.evm_rpc, "eth_fee_history"),
        (test.evm_rpc, "eth_get_block_by_number"),
        (test.evm_rpc, "eth_get_logs"),
        (test.evm_rpc, "eth_get_transaction_receipt"),
        (test.evm_rpc, "eth_send_raw_transaction"),
        (test.icp_ledger, "transfer"),
        (test.icp_ledger, "transferFrom"),
        (test.xrc, "get_exchange_rate"),
    ];

    for (canister, method) in expected_calls {
        assert!(
            canister_calls
                .iter()
                .any(|x| x.canister == canister && &x.method == method),
            "canister call {} {} not found",
            canister,
            method
        );
    }
    test.check_reproducibility().await;
}

// TODO: re-enable this test after enabling rate limits.
#[tokio::test]
#[ignore = "re-enable after benchmarking"]
async fn test_deposit_rate_limit_per_caller() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
    }

    let request = TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let _ = test.transfer(test.user, request.clone()).await;
    let result = test.transfer(test.user, request).await;

    match result {
        types::TransferResponse::Failed(error_message) => {
            assert!(
                error_message.error.contains("Please wait"),
                "{}",
                error_message.error
            );
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result);
        }
    }
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_rate_limit_overall() {
    let test = TestEnv::new().await;

    let approve = Amount::new(10_000 * E8S);
    let amount = Amount::new(10 * E8S);
    for user in test.users.clone() {
        test.icp_approve(user, test.one_sec, approve).await;
    }

    for _ in 0..40 {
        test.tick().await;
    }
    let request = |user| TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(user),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: None,
        },
    };

    for i in 0..CONFIG.flow.max_concurrent_flows {
        let result = test
            .transfer(test.users[i % 20], request(test.users[i % 20]))
            .await;
        match result {
            types::TransferResponse::Accepted(_) => {}
            types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
                unreachable!("Unexpected response: {:?}", result);
            }
        }
    }

    let result = test.transfer(test.user, request(test.user)).await;

    match result {
        types::TransferResponse::Failed(error_message) => {
            assert!(
                error_message.error.contains("The alpha version allows"),
                "{}",
                error_message.error
            );
        }
        types::TransferResponse::Accepted(_) | types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result);
        }
    }
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_deposit_to_evm_twice_ok() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount.add(amount, ""))
        .await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let transfer = TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let result = test.transfer(test.user, transfer.clone()).await;

    match result {
        types::TransferResponse::Accepted(_) => {}
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..100 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test.transfer(test.user, transfer).await;

    match result {
        types::TransferResponse::Accepted(_) => {}
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_usdc_deposit_to_icp_ok() {
    let test = TestEnv::new().await;

    let amount_u64 = 100 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let balance0 = test.usdc_balance_of(test.user).await;

    let tx = test
        .evm
        .usdc_deposit(
            amount,
            FixedBytes(
                encode_icp_account(
                    icrc_account(test.user)
                        .as_icp()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
                .try_into()
                .unwrap(),
            ),
        )
        .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: tx.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let deposit_fee = amount_u64 as f64
        * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let deposit_fee = Wei::new(deposit_fee.round() as u128);
    let total_fee = deposit_fee.add(USDC_LEDGER_FEE, "");
    let deposited = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount {
            address: test.evm.user.to_string()
        }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount),);
    assert_eq!(tx.destination.account, Some(icrc_account(test.user)));
    assert_eq!(tx.destination.amount, Nat::from(deposited),);

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance1 = test.usdc_balance_of(test.user).await;

    let credited = balance1.sub(balance0, "");

    assert_eq!(credited, deposited);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_usdc_deposit_and_withdraw_ok() {
    let test = TestEnv::new().await;

    let amount_u64 = 100 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let balance0 = test.usdc_balance_of(test.user).await;

    let tx = test
        .evm
        .usdc_deposit(
            amount,
            FixedBytes(
                encode_icp_account(
                    icrc_account(test.user)
                        .as_icp()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
                .try_into()
                .unwrap(),
            ),
        )
        .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: tx.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let deposit_fee = amount_u64 as f64
        * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let deposit_fee = Wei::new(deposit_fee.round() as u128);
    let total_fee = deposit_fee.add(USDC_LEDGER_FEE, "");
    let deposited = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount {
            address: test.evm.user.to_string()
        }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount),);
    assert_eq!(tx.destination.account, Some(icrc_account(test.user)));
    assert_eq!(tx.destination.amount, Nat::from(deposited));

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance1 = test.usdc_balance_of(test.user).await;

    let credited = balance1.sub(balance0, "");

    assert_eq!(credited, deposited);

    let amount_u64 = 50 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let balance0 = test.evm.usdc_balance_of(test.evm.user).await;

    test.usdc_approve(test.user, test.one_sec, amount).await;

    let transfer = TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::USDC,
            amount: None,
        },
    };

    let result = test.transfer(test.user, transfer).await;
    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..100 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let tx = test.get_transfer(test.user, transfer_id).await;
    assert_eq!(tx.status, Some(types::Status::Succeeded));

    let withdraw_fee = amount_u64 as f64
        * flow_config(Direction::IcpToEvm, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let withdraw_fee = Wei::new(withdraw_fee.round() as u128);
    let total_fee = withdraw_fee.add(USDC_LEDGER_FEE, "");
    let withdrawn = amount.sub(total_fee, "");

    let balance1 = test.evm.usdc_balance_of(test.evm.user).await;
    assert_eq!(balance1.sub(balance0, ""), withdrawn);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_transfer_fee_usdc() {
    let test = TestEnv::new().await;
    let config = icp_ledger_config(types::Token::USDC);

    let amount_u64 = 1_000 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let tx = test
        .evm
        .usdc_deposit(
            amount,
            FixedBytes(
                encode_icp_account(
                    icrc_account(test.user)
                        .as_icp()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
                .try_into()
                .unwrap(),
            ),
        )
        .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: tx.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let deposit_fee = amount_u64 as f64
        * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let deposit_fee = Wei::new(deposit_fee.round() as u128);
    let total_fee = deposit_fee.add(USDC_LEDGER_FEE, "");

    let tx = test.get_transfer(test.user, transfer_id).await;
    assert_eq!(tx.status, Some(types::Status::Succeeded));

    let balance_before = test.usdc_balance_of(config.fee_receiver).await.into_inner();
    test.advance_time(Duration::from_secs(6 * 3_600)).await;

    for _ in 0..50 {
        test.tick().await;
    }
    let balance_after = test.usdc_balance_of(config.fee_receiver).await.into_inner();

    assert_eq!(
        balance_after - balance_before,
        total_fee.into_inner() - config.transfer_fee.into_inner()
    );
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_transfer_fee_icp() {
    let test = TestEnv::new().await;
    let config = icp_ledger_config(types::Token::ICP);

    let amount = Amount::new(1_000 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..100 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let transfer = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(transfer.status, Some(Status::Succeeded));

    let evm_amount = Amount::try_from(transfer.destination.amount).unwrap();

    let total_fee =
        amount.into_inner() - evm_amount.into_inner() - config.transfer_fee.into_inner();

    let balance_before = test.icp_balance_of(config.fee_receiver).await.into_inner();
    test.advance_time(Duration::from_secs(6 * 3_600)).await;

    for _ in 0..50 {
        test.tick().await;
    }
    let balance_after = test.icp_balance_of(config.fee_receiver).await.into_inner();

    assert_eq!(
        balance_after - balance_before,
        total_fee - config.transfer_fee.into_inner()
    );
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_queue_position() {
    let test = TestEnv::new().await;

    let amount = Amount::new(1_000 * E8S);
    test.icp_approve(test.users[0], test.one_sec, amount)
        .await;
    test.icp_approve(test.users[1], test.one_sec, amount)
        .await;
    test.icp_approve(test.users[2], test.one_sec, amount)
        .await;

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    fn request(user: Principal, evm_user: String) -> TransferArg {
        TransferArg {
            source: types::Asset {
                chain: Chain::ICP,
                account: icrc_account(user),
                token: types::Token::ICP,
                amount: Nat::from(100 * E8S),
                tx: None,
            },
            destination: AssetRequest {
                chain: Chain::Base,
                account: types::Account::Evm(EvmAccount { address: evm_user }),
                token: types::Token::ICP,
                amount: None,
            },
        }
    }

    test.pause_task(
        test.controller,
        one_sec::task::TaskType::Evm {
            chain: EvmChain::Base,
            task: one_sec::evm::Task::Writer(one_sec::evm::writer::Task::NewTx),
        },
    )
    .await;

    let _ = test
        .transfer(
            test.users[0],
            request(test.users[0], test.evm.user.to_string()),
        )
        .await;

    let _ = test
        .transfer(
            test.users[1],
            request(test.users[1], test.evm.user.to_string()),
        )
        .await;

    let result3 = test
        .transfer(
            test.users[2],
            request(test.users[2], test.evm.user.to_string()),
        )
        .await;

    let transfer_id = match result3 {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result3)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let transfer = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(transfer.queue_position, Some(2_u64));
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_evm_to_icp_batch() {
    let test = TestEnv::new().await;
    let config = icp_ledger_config(types::Token::USDC);

    let balance0 = test.usdc_balance_of(test.user).await;

    let amount_u64 = 1_000_000;
    let amount = Wei::new(amount_u64);

    let mut txs = vec![];

    for _ in 0..50 {
        let tx = test
            .evm
            .usdc_deposit(
                amount,
                FixedBytes(
                    encode_icp_account(
                        icrc_account(test.user)
                            .as_icp()
                            .unwrap()
                            .try_into()
                            .unwrap(),
                    )
                    .try_into()
                    .unwrap(),
                ),
            )
            .await;
        txs.push(tx);
    }

    for _ in 0..100 {
        test.evm.mine_block().await;
    }
    for _ in 0..50 {
        test.tick().await;

        let transfer = TransferArg {
            source: Asset {
                chain: Chain::Base,
                account: types::Account::Evm(EvmAccount {
                    address: test.evm.user.to_string(),
                }),
                token: types::Token::USDC,
                amount: amount.into(),
                tx: Some(types::Tx::Evm(types::EvmTx {
                    hash: txs[0].transaction_hash.to_string(),
                    log_index: None,
                })),
            },
            destination: types::AssetRequest {
                chain: Chain::ICP,
                account: icrc_account(test.user),
                token: types::Token::USDC,
                amount: None,
            },
        };

        test.transfer(test.user, transfer).await;
    }

    let mut total_deposited = Amount128::ZERO;

    for tx in txs {
        let transfer = TransferArg {
            source: Asset {
                chain: Chain::Base,
                account: types::Account::Evm(EvmAccount {
                    address: test.evm.user.to_string(),
                }),
                token: types::Token::USDC,
                amount: amount.into(),
                tx: Some(types::Tx::Evm(types::EvmTx {
                    hash: tx.transaction_hash.to_string(),
                    log_index: None,
                })),
            },
            destination: types::AssetRequest {
                chain: Chain::ICP,
                account: icrc_account(test.user),
                token: types::Token::USDC,
                amount: None,
            },
        };

        let result = test.transfer(test.user, transfer).await;

        let transfer_id = match result {
            types::TransferResponse::Accepted(transfer_id) => transfer_id,
            types::TransferResponse::Fetching(_) => {
                unreachable!("Unexpected response: {:?}", result)
            }
            types::TransferResponse::Failed(error_message) => {
                unreachable!("Unexpected response: {:?}", error_message)
            }
        };

        let deposit_fee = amount_u64 as f64
            * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
                .fee
                .as_f64();
        let deposit_fee = Wei::new(deposit_fee.round() as u128);
        let total_fee = deposit_fee.add(Wei::new(config.transfer_fee.into_inner()), "");
        let deposited = amount.sub(total_fee, "");

        total_deposited = total_deposited.add(deposited, "");

        let tx = test.get_transfer(test.user, transfer_id).await;

        assert_eq!(tx.status, Some(types::Status::Succeeded));
        assert_eq!(
            tx.source.account,
            Some(types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string()
            }))
        );
        assert_eq!(tx.source.amount, Nat::from(amount),);
        assert_eq!(tx.destination.account, Some(icrc_account(test.user)));
        assert_eq!(tx.destination.amount, Nat::from(deposited),);

        assert!(tx.destination.tx.is_some());

        assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
        assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));
    }

    let balance1 = test.usdc_balance_of(test.user).await;

    let credited = balance1.sub(balance0, "");

    assert_eq!(credited, total_deposited);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_sign_tx_batch() {
    let test = TestEnv::new().await;

    let amount = Amount::new(10 * E8S);

    for user in test.users.clone() {
        test.icp_approve(user, test.one_sec, amount).await;
    }

    for _ in 0..40 {
        test.tick().await;
    }

    let request = |user| TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(user),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let mut awaiting = vec![];

    for i in 0..20 {
        awaiting.push(test.transfer(test.users[i], request(test.users[i])));
    }

    let results = future::join_all(awaiting).await;

    for _ in 0..150 {
        test.tick().await;
    }

    for _ in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..250 {
        test.tick().await;
    }

    for result in results {
        let id = match result {
            types::TransferResponse::Accepted(id) => id,
            types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
                unreachable!("Unexpected response: {:?}", result);
            }
        };

        let transfer = test.get_transfer(test.user, id).await;
        assert_eq!(transfer.status, Some(Status::Succeeded));
    }
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_concurrent_mint_and_burn() {
    let test = TestEnv::new().await;

    let amount = Amount::new(10 * E8S);

    test.icp_approve(test.user, test.one_sec, amount).await;

    let icp_balance1 = test.icp_balance_of(test.user).await;

    for _ in 0..40 {
        test.tick().await;
    }

    let request = TransferArg {
        source: types::Asset {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: None,
        },
        destination: AssetRequest {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let result = test.transfer(test.user, request).await;

    test.pause_task(
        test.controller,
        one_sec::task::TaskType::Evm {
            chain: EvmChain::Base,
            task: evm::Task::Writer(evm::writer::Task::PollTx),
        },
    )
    .await;

    let id1 = match result {
        types::TransferResponse::Accepted(id) => id,
        types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
            unreachable!("Unexpected response: {:?}", result);
        }
    };

    for _ in 0..150 {
        test.tick().await;
    }

    let amount = Amount::new(5 * E8S);

    let tx_receipt = test
        .evm
        .icp_burn(
            test.evm.icp,
            Wei::new(amount.into_inner()),
            FixedBytes(
                encode_icp_account(
                    icrc_account(test.user)
                        .as_icp()
                        .unwrap()
                        .try_into()
                        .unwrap(),
                )
                .try_into()
                .unwrap(),
            ),
        )
        .await;

    assert!(tx_receipt.status());

    let arg = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(EvmTx {
                hash: Hex32::from(tx_receipt.transaction_hash.0).to_string(),
                log_index: None,
            })),
        },
        destination: AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::ICP,
            amount: None,
        },
    };
    for _ in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..200 {
        test.tick().await;
        test.transfer(test.user, arg.clone()).await;
    }

    let result = test.transfer(test.user, arg).await;
    let id2 = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
            unreachable!("unexpected result: {:?}", result)
        }
    };
    let transfer2 = test.get_transfer(test.user, id2).await;
    assert_eq!(transfer2.status, Some(Status::Succeeded));

    test.resume_task(
        test.controller,
        one_sec::task::TaskType::Evm {
            chain: EvmChain::Base,
            task: evm::Task::Writer(evm::writer::Task::PollTx),
        },
    )
    .await;

    for _ in 0..200 {
        test.tick().await;
    }

    let transfer1 = test.get_transfer(test.user, id1).await;

    assert_eq!(transfer1.status, Some(Status::Succeeded));

    let icp_balance2 = test.icp_balance_of(test.user).await;

    let evm_balance = test.evm.icp_balance_of(test.evm.user).await;

    assert_eq!(
        icp_balance1.sub(icp_balance2, "").into_inner() as u64,
        u64::try_from(transfer1.source.amount.0).unwrap()
            - u64::try_from(transfer2.destination.amount.0).unwrap()
    );

    assert_eq!(
        evm_balance.into_inner() as u64,
        u64::try_from(transfer1.destination.amount.0).unwrap()
            - u64::try_from(transfer2.source.amount.0).unwrap()
    );
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_withdraw_icp_with_account_id() {
    let test = TestEnv::new().await;

    let balance0 = test.icp_balance_of(test.user).await;
    let amount = Amount::new(10 * E8S);
    test.icp_approve(test.user, test.one_sec, amount).await;

    let balance1 = test.icp_balance_of(test.user).await;
    assert_eq!(balance1, balance0.sub(ICP_LEDGER_FEE, ""));

    for _ in 0..40 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let result = test
        .transfer(
            test.user,
            TransferArg {
                source: types::Asset {
                    chain: Chain::ICP,
                    account: icrc_account(test.user),
                    token: types::Token::ICP,
                    amount: amount.into(),
                    tx: None,
                },
                destination: AssetRequest {
                    chain: Chain::Base,
                    account: types::Account::Evm(EvmAccount {
                        address: test.evm.user.to_string(),
                    }),
                    token: types::Token::ICP,
                    amount: None,
                },
            },
        )
        .await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    for _ in 0..200 {
        test.tick().await;
        test.evm.mine_block().await;
    }

    let balance2 = test.icp_balance_of(test.user).await;

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));

    let amount = Amount::new(2 * E8S);

    let icp_account =
        icp::IcpAccount::AccountId(AccountIdentifier::new(&test.user, &Subaccount([0_u8; 32])));

    let tx_receipt = test
        .evm
        .icp_burn(
            test.evm.icp,
            Wei::new(amount.into_inner()),
            FixedBytes(encode_icp_account(icp_account).try_into().unwrap()),
        )
        .await;

    assert!(tx_receipt.status());

    let arg = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::ICP,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(EvmTx {
                hash: Hex32::from(tx_receipt.transaction_hash.0).to_string(),
                log_index: None,
            })),
        },
        destination: AssetRequest {
            chain: Chain::ICP,
            account: types::Account::Icp(icp_account.into()),
            token: types::Token::ICP,
            amount: None,
        },
    };

    let result = test.transfer(test.user, arg.clone()).await;
    match result {
        types::TransferResponse::Fetching(_) => {}
        types::TransferResponse::Accepted(_) | types::TransferResponse::Failed(_) => {
            unreachable!("unexpected result: {:?}", result)
        }
    }

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..200 {
        test.tick().await;
        test.transfer(test.user, arg.clone()).await;
    }

    let result = test.transfer(test.user, arg).await;
    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) | types::TransferResponse::Failed(_) => {
            unreachable!("unexpected result: {:?}", result)
        }
    };

    let withdraw_fee = amount.into_inner() as f64
        * flow_config(Direction::EvmToIcp, types::Token::ICP, types::Token::ICP)
            .fee
            .as_f64();
    let withdraw_fee = Amount::new(withdraw_fee.round() as u128);
    let total_fee = withdraw_fee.add(ICP_LEDGER_FEE, "");
    let withdrawn = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;
    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount {
            address: test.evm.user.to_string()
        }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount));
    assert_eq!(
        tx.destination.account,
        Some(types::Account::Icp(icp_account.into()))
    );
    assert_eq!(tx.destination.amount, Nat::from(withdrawn));

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance3 = test.icp_balance_of(test.user).await;

    let credited = balance3.sub(balance2, "");

    assert_eq!(credited, withdrawn);
    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_cannot_deposit_usdc_with_account_id() {
    let test = TestEnv::new().await;

    let amount_u64 = 100 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let icp_account =
        icp::IcpAccount::AccountId(AccountIdentifier::new(&test.user, &Subaccount([0_u8; 32])));

    let tx = test
        .evm
        .usdc_deposit(
            amount,
            FixedBytes(encode_icp_account(icp_account).try_into().unwrap()),
        )
        .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: test.evm.user.to_string(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: tx.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: types::Account::Icp(icp_account.into()),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };
    let tx = test.get_transfer(test.user, transfer_id).await;

    match tx.status.as_ref().unwrap() {
        Status::Failed(error_message) => {
            assert!(
                error_message
                    .error
                    .contains("does not support account identifiers"),
                "{}",
                error_message.error
            );
        }
        _ => {
            unreachable!("unexpected status: {:?}", tx.status.unwrap());
        }
    }

    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_forwarding_address() {
    let test = TestEnv::new().await;

    let amount_u64 = 100 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let balance0 = test.usdc_balance_of(test.user).await;

    let receiver = types::IcpAccount::ICRC(Account {
        owner: test.user,
        subaccount: None,
    });

    let addr = test
        .get_forwarding_address(test.controller, receiver.clone())
        .await;

    let receipt = test
        .evm
        .usdc_transfer(Address::from_hex(&addr).unwrap(), amount)
        .await;

    assert!(receipt.status());

    let arg = ForwardEvmToIcpArg {
        chain: EvmChain::Base,
        token: Token::USDC,
        address: addr.clone(),
        receiver: receiver.clone(),
    };

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    assert_eq!(response.status, Some(ForwardingStatus::CheckingBalance));

    test.submit_forwarding_update(
        test.controller,
        ForwardingUpdate {
            chain: EvmChain::Base,
            balances: vec![],
            forwarded: vec![],
            to_sign: vec![UnsignedForwardingTx {
                token: arg.token,
                address: arg.address.clone(),
                receiver: arg.receiver.clone(),
                amount: amount.into(),
                nonce: 0,
                max_fee_per_gas: 1000000000,
                max_priority_fee_per_gas: 1000000000,
                requested_tx: types::RequestedTx::ApproveAndLock,
            }],
        },
    )
    .await;

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    assert_eq!(response.status, Some(ForwardingStatus::Forwarding));

    for _ in 0..50 {
        test.tick().await;
    }

    let txs = test
        .get_forwarding_transactions(test.user, EvmChain::Base)
        .await;
    let tx = txs[0].clone();

    assert_eq!(tx.address, addr);
    assert_eq!(tx.token, Token::USDC);

    test.evm
        .transfer_eth(Address::from_hex(&addr).unwrap(), "0.1")
        .await;

    let receipt = test
        .evm
        .eth_send_raw_transaction(&tx.approve_tx.unwrap().bytes)
        .await;
    assert!(receipt.status());

    let receipt = test
        .evm
        .eth_send_raw_transaction(&tx.lock_or_burn_tx.bytes)
        .await;
    assert!(receipt.status());

    test.submit_forwarding_update(
        test.controller,
        ForwardingUpdate {
            chain: EvmChain::Base,
            balances: vec![],
            to_sign: vec![],
            forwarded: vec![ForwardedTx {
                token: Token::USDC,
                address: addr.clone(),
                receiver,
                nonce: 0,
                total_tx_cost_in_wei: tx.total_tx_cost_in_wei,
                lock_or_burn_tx: EvmTx {
                    hash: receipt.transaction_hash.to_string(),
                    log_index: None,
                },
            }],
        },
    )
    .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: addr.clone(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: receipt.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    let metadata = test.get_metadata(test.user).await;

    assert_eq!(response.done, Some(transfer_id));
    assert_eq!(response.status, Some(ForwardingStatus::CheckingBalance));

    let deposit_fee = amount_u64 as f64
        * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let deposit_fee = Wei::new(deposit_fee.round() as u128);
    let wei_per_token = metadata
        .tokens
        .iter()
        .find(|x| x.token == Some(Token::USDC))
        .unwrap()
        .wei_per_token;
    let forwarding_fee = tx.total_tx_cost_in_wei as f64 / wei_per_token.round();
    let total_fee = deposit_fee
        .add(USDC_LEDGER_FEE, "")
        .add(Wei::new(forwarding_fee as u128), "");
    let deposited = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount { address: addr }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount),);
    assert_eq!(tx.destination.account, Some(icrc_account(test.user)));

    assert_eq!(tx.destination.amount, Nat::from(deposited));

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance1 = test.usdc_balance_of(test.user).await;

    let credited = balance1.sub(balance0, "");

    assert_eq!(credited, deposited);

    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_forwarding_address_with_subaccount() {
    let test = TestEnv::new().await;

    let amount_u64 = 100 * 1_000_000;
    let amount = Wei::new(amount_u64);

    let balance0 = test.usdc_balance_of(test.user).await;

    let receiver = types::IcpAccount::ICRC(Account {
        owner: test.user,
        subaccount: Some([1u8; 32]),
    });

    let addr = test
        .get_forwarding_address(test.controller, receiver.clone())
        .await;

    let receipt = test
        .evm
        .usdc_transfer(Address::from_hex(&addr).unwrap(), amount)
        .await;

    assert!(receipt.status());

    let arg = ForwardEvmToIcpArg {
        chain: EvmChain::Base,
        token: Token::USDC,
        address: addr.clone(),
        receiver: receiver.clone(),
    };

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    assert_eq!(response.status, Some(ForwardingStatus::CheckingBalance));

    test.submit_forwarding_update(
        test.controller,
        ForwardingUpdate {
            chain: EvmChain::Base,
            balances: vec![],
            forwarded: vec![],
            to_sign: vec![UnsignedForwardingTx {
                token: arg.token,
                address: arg.address.clone(),
                receiver: arg.receiver.clone(),
                amount: amount.into(),
                nonce: 0,
                max_fee_per_gas: 1000000000,
                max_priority_fee_per_gas: 1000000000,
                requested_tx: types::RequestedTx::ApproveAndLock,
            }],
        },
    )
    .await;

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    assert_eq!(response.status, Some(ForwardingStatus::Forwarding));

    for _ in 0..50 {
        test.tick().await;
    }

    let txs = test
        .get_forwarding_transactions(test.user, EvmChain::Base)
        .await;
    let tx = txs[0].clone();

    assert_eq!(tx.address, addr);
    assert_eq!(tx.token, Token::USDC);

    test.evm
        .transfer_eth(Address::from_hex(&addr).unwrap(), "0.1")
        .await;

    let receipt = test
        .evm
        .eth_send_raw_transaction(&tx.approve_tx.unwrap().bytes)
        .await;
    assert!(receipt.status());

    let receipt = test
        .evm
        .eth_send_raw_transaction(&tx.lock_or_burn_tx.bytes)
        .await;
    assert!(receipt.status());

    test.submit_forwarding_update(
        test.controller,
        ForwardingUpdate {
            chain: EvmChain::Base,
            balances: vec![],
            to_sign: vec![],
            forwarded: vec![ForwardedTx {
                token: Token::USDC,
                address: addr.clone(),
                receiver: receiver.clone(),
                nonce: 0,
                total_tx_cost_in_wei: tx.total_tx_cost_in_wei,
                lock_or_burn_tx: EvmTx {
                    hash: receipt.transaction_hash.to_string(),
                    log_index: None,
                },
            }],
        },
    )
    .await;

    let transfer = TransferArg {
        source: Asset {
            chain: Chain::Base,
            account: types::Account::Evm(EvmAccount {
                address: addr.clone(),
            }),
            token: types::Token::USDC,
            amount: amount.into(),
            tx: Some(types::Tx::Evm(types::EvmTx {
                hash: receipt.transaction_hash.to_string(),
                log_index: None,
            })),
        },
        destination: types::AssetRequest {
            chain: Chain::ICP,
            account: icrc_account(test.user),
            token: types::Token::USDC,
            amount: None,
        },
    };

    for _i in 0..100 {
        test.evm.mine_block().await;
    }

    for _ in 0..50 {
        test.tick().await;
        test.transfer(test.user, transfer.clone()).await;
    }

    let result = test.transfer(test.user, transfer).await;

    let transfer_id = match result {
        types::TransferResponse::Accepted(transfer_id) => transfer_id,
        types::TransferResponse::Fetching(_) => {
            unreachable!("Unexpected response: {:?}", result)
        }
        types::TransferResponse::Failed(error_message) => {
            unreachable!("Unexpected response: {:?}", error_message)
        }
    };

    let response = test.forward_evm_to_icp(test.controller, arg.clone()).await;
    let metadata = test.get_metadata(test.user).await;

    assert_eq!(response.done, Some(transfer_id));
    assert_eq!(response.status, Some(ForwardingStatus::CheckingBalance));

    let deposit_fee = amount_u64 as f64
        * flow_config(Direction::EvmToIcp, types::Token::USDC, types::Token::USDC)
            .fee
            .as_f64();
    let deposit_fee = Wei::new(deposit_fee.round() as u128);
    let wei_per_token = metadata
        .tokens
        .iter()
        .find(|x| x.token == Some(Token::USDC))
        .unwrap()
        .wei_per_token;
    let forwarding_fee = tx.total_tx_cost_in_wei as f64 / wei_per_token.round();
    let total_fee = deposit_fee
        .add(USDC_LEDGER_FEE, "")
        .add(Wei::new(forwarding_fee as u128), "");
    let deposited = amount.sub(total_fee, "");

    let tx = test.get_transfer(test.user, transfer_id).await;

    assert_eq!(tx.status, Some(types::Status::Succeeded));
    assert_eq!(
        tx.source.account,
        Some(types::Account::Evm(EvmAccount { address: addr }))
    );
    assert_eq!(tx.source.amount, Nat::from(amount),);
    assert_eq!(tx.destination.account, Some(types::Account::Icp(receiver)));

    assert_eq!(tx.destination.amount, Nat::from(deposited));

    assert!(tx.destination.tx.is_some());

    assert_eq!(tx.trace.entries[0].event, Some(TraceEvent::ConfirmTx));
    assert_eq!(tx.trace.entries[1].event, Some(TraceEvent::ConfirmTx));

    let balance1 = test
        .usdc_balance_with_subaccount(test.user, [1_u8; 32])
        .await;

    let credited = balance1.sub(balance0, "");

    assert_eq!(credited, deposited);

    test.check_reproducibility().await;
}

#[tokio::test]
async fn test_forwarding_address_guards() {
    let test = TestEnv::new().await;

    let receiver = types::IcpAccount::ICRC(Account {
        owner: test.user,
        subaccount: None,
    });

    let addr1 = test
        .get_forwarding_address(test.controller, receiver.clone())
        .await;

    let addr2 = test
        .get_forwarding_address(test.icp_ledger, receiver.clone())
        .await;

    assert_eq!(addr1, addr2);

    let addr3 = test
        .try_get_forwarding_address(test.user, receiver.clone())
        .await;

    assert!(addr3.is_err());
}
