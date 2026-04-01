//! This module defines the default configuration of the global state machine.
use std::collections::BTreeSet;
use std::str::FromStr;
use std::time::Duration;

use candid::Principal;
use evm_rpc_types::{EthMainnetService, L2MainnetService, RpcApi, RpcServices};
use ic_ethereum_types::Address;

use crate::{
    api::types::{EvmChain, Token},
    evm::{self, TxFee},
    flow::{self, config::FlowConfig, event::Direction},
    icp::{self},
    numeric::{Amount, BlockNumber, GasAmount, Percent, TxNonce, Wei, WeiPerGas, E6S, E8S},
};

/// The maximum number of blocks supported in `eth_getLogs`.
const GET_LOGS_MAX_BLOCK_RANGE: usize = 3_000;

/// The operating mode of an ICP or EVM ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    /// The ledger is minting and burning tokens.
    Minter,
    /// The ledger is locking and unlocking tokens.
    Locker,
}

/// The configuration parameters of the global state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The configuration parameters of the ICP state machine.
    pub icp: icp::Config,
    /// The configuration parameters of the EVM state machines.
    pub evm: Vec<evm::Config>,
    /// The configuration parameters of the flow state machine.
    pub flow: flow::config::Config,
}

impl Config {
    pub fn mainnet() -> Self {
        Self {
            icp: icp::Config {
                ledger: vec![
                    icp::ledger::Config {
                        token: Token::ICP,
                        operating_mode: OperatingMode::Locker,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
                        index_canister: None,
                        supports_account_id: true,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(100_000_000),
                        transfer_batch: 40,
                        transfer_fee: Amount::new(10_000),
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::GLDT,
                        operating_mode: OperatingMode::Locker,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("6c7su-kiaaa-aaaar-qaira-cai").unwrap(),
                        index_canister: Some(
                            Principal::from_text("oo6x4-xiaaa-aaaap-abrza-cai").unwrap(),
                        ),
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(1_000_000_000),
                        transfer_batch: 40,
                        transfer_fee: Amount::new(10_000_000),
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::ckBTC,
                        operating_mode: OperatingMode::Locker,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai").unwrap(),
                        index_canister: Some(
                            Principal::from_text("n5wcd-faaaa-aaaar-qaaea-cai").unwrap(),
                        ),
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(1_000_000),
                        transfer_batch: 40,
                        transfer_fee: Amount::new(10),
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::BOB,
                        operating_mode: OperatingMode::Locker,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("7pail-xaaaa-aaaas-aabmq-cai").unwrap(),
                        index_canister: None,
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(E8S),
                        transfer_batch: 40,
                        transfer_fee: Amount::new(1_000_000),
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::CHAT,
                        operating_mode: OperatingMode::Locker,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("2ouva-viaaa-aaaaq-aaamq-cai").unwrap(),
                        index_canister: None,
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(E8S),
                        transfer_batch: 40,
                        transfer_fee: Amount::new(100_000),
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::USDC,
                        operating_mode: OperatingMode::Minter,
                        decimals: 6,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("53nhb-haaaa-aaaar-qbn5q-cai").unwrap(),
                        index_canister: Some(
                            Principal::from_text("f4e7e-pqaaa-aaaar-qbpgq-cai").unwrap(),
                        ),
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(1_000_000),
                        transfer_fee: Amount::new(0),
                        transfer_batch: 40,
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::USDT,
                        operating_mode: OperatingMode::Minter,
                        decimals: 6,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("ij33n-oiaaa-aaaar-qbooa-cai").unwrap(),
                        index_canister: Some(
                            Principal::from_text("fvhuy-zyaaa-aaaar-qbpha-cai").unwrap(),
                        ),
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(1_000_000),
                        transfer_fee: Amount::new(0),
                        transfer_batch: 40,
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                    icp::ledger::Config {
                        token: Token::cbBTC,
                        operating_mode: OperatingMode::Minter,
                        decimals: 8,
                        initial_balance: Amount::ZERO,
                        canister: Principal::from_text("io25z-dqaaa-aaaar-qbooq-cai").unwrap(),
                        index_canister: Some(
                            Principal::from_text("fsgsm-uaaaa-aaaar-qbphq-cai").unwrap(),
                        ),
                        supports_account_id: false,
                        fee_receiver: Principal::from_text("54mbv-kyaaa-aaaar-qbn5a-cai").unwrap(),
                        fee_threshold: Amount::new(1_000_000),
                        transfer_fee: Amount::new(0),
                        transfer_batch: 40,
                        transfer_task_busy_delay: Duration::from_secs(2),
                        transfer_task_idle_delay: Duration::from_secs(600),
                        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
                    },
                ],
                ecdsa_key_name: "key_1".into(),
                xrc_canister_id: Principal::from_text("uf6dk-hyaaa-aaaaq-qaaaq-cai").unwrap(),
                min_cycles_balance: 1_000_000_000_000,
                relayers: vec![
                    Principal::from_text(
                        "sjs7y-dzeg2-jvei6-a2msh-43lpd-tja3b-qhv5m-d3obo-ffxue-aw6hv-iae",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "7b2jp-mcwvo-3d7ug-elgwv-kgtfz-hz2en-y7kla-papjg-izhxj-xf5sx-3ae",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "a64o7-4robr-otexu-zlm2m-igbb2-usxuw-ygiwr-i4qa7-iw4f3-ouovn-7qe",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "7vjuu-p7ll7-yvsdl-526ve-bklqs-omwsm-u7an6-lbevf-5eojq-lx4dt-xqe",
                    )
                    .unwrap(),
                ],
                market_makers: BTreeSet::from([
                    Principal::from_text(
                        "e4i4s-3uiue-x5ogl-43a5p-wi3uh-uacyf-lzd3y-wnabu-h7ocz-425ne-rqe",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "rkbl7-i632v-iurs6-zl22u-wczrb-nzkpj-ugjdh-hrkwv-2opdz-th7fg-xae",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "t5mxu-fkvds-ogrcy-r3gdk-gqqvt-v3c4x-4xowv-rw27j-rpui3-zvlz6-cqe",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "4jiij-z5bmg-wreov-eoug6-tb3qh-rls27-qm56z-ivvrw-5k2qq-eozht-3qe",
                    )
                    .unwrap(),
                    Principal::from_text(
                        "3dgs5-rimrr-mvmwo-zihci-4lwxj-jc4db-gik5o-lavk5-qtxf3-teeyz-bqe",
                    )
                    .unwrap(),
                ]),
            },
            evm: vec![
                evm::Config {
                    chain: EvmChain::Base,
                    chain_id: 8453,
                    reader: crate::evm::reader::Config {
                        initial_block: None,
                        num_blocks_to_fetch_initially: 499,
                        max_num_blocks_to_fetch_per_call: GET_LOGS_MAX_BLOCK_RANGE,
                        fetch_tx_logs_task_delay: Duration::from_secs(500),
                    },
                    writer: crate::evm::writer::Config {
                        initial_nonce: TxNonce::ZERO,
                        initial_fee_estimate: TxFee {
                            max_fee_per_gas: WeiPerGas::new(1_200_000_000),
                            max_priority_fee_per_gas: WeiPerGas::new(1_100_000_000),
                        },
                        tx_fee_bump: Percent::from_percent(10),
                        tx_fee_margin: Percent::from_percent(250),
                        tx_sign_batch: 4,
                        tx_resubmit_delay: Duration::from_secs(30),
                        tx_resend_delay: Duration::from_secs(10),
                        tx_sign_to_send_delay: Duration::from_secs(15),
                        tx_sign_to_poll_delay: Duration::from_secs(20),
                        tx_send_to_poll_delay: Duration::from_secs(5),
                        fetch_fee_estimate_delay: Duration::from_secs(60),
                    },
                    prover: crate::evm::prover::Config {
                        head: crate::evm::prover::head::Config {
                            safety_margin: BlockNumber::new(10),
                            block_time_min: Duration::from_millis(1_000),
                            block_time_avg: Duration::from_millis(1_900),
                            block_time_max: Duration::from_millis(1_000_000),
                            block_time_after_miss: Percent::from_percent(110),
                            block_time_after_hit: Percent::from_percent(99),
                        },
                    },
                    forwarder: crate::evm::forwarder::Config {
                        request_expiry: Duration::from_secs(3_600),
                        approve_amount: Amount::new(u128::MAX / 2),
                        batch_size: 8,
                        max_pending_count: 1_000,
                    },
                    evm_rpc: crate::evm::evm_rpc::Config {
                        rpc_services: RpcServices::BaseMainnet(Some(vec![
                            L2MainnetService::Alchemy,
                            L2MainnetService::Ankr,
                            L2MainnetService::BlockPi,
                        ])),
                        rpc_service: RpcServices::BaseMainnet(Some(vec![L2MainnetService::Ankr])),
                        evm_rpc_canister_id: Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai")
                            .unwrap(),
                        evm_rpc_canister_cycles: 500_000_000_000,
                        consensus_threshold: 2,
                        get_logs_max_block_range: GET_LOGS_MAX_BLOCK_RANGE,
                    },
                    ledger: vec![
                        evm::ledger::Config {
                            token: Token::ICP,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::ckBTC,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::BOB,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::CHAT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::GLDT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::USDC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 6,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::cbBTC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(100_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(100_000),
                            gas_limit_for_approve: GasAmount::new(100_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                    ],
                },
                evm::Config {
                    chain: EvmChain::Arbitrum,
                    chain_id: 42161,
                    reader: crate::evm::reader::Config {
                        initial_block: None,
                        num_blocks_to_fetch_initially: 499,
                        max_num_blocks_to_fetch_per_call: GET_LOGS_MAX_BLOCK_RANGE,
                        fetch_tx_logs_task_delay: Duration::from_secs(500),
                    },
                    writer: crate::evm::writer::Config {
                        initial_nonce: TxNonce::ZERO,
                        initial_fee_estimate: TxFee {
                            max_fee_per_gas: WeiPerGas::new(1_200_000_000),
                            max_priority_fee_per_gas: WeiPerGas::new(1_100_000_000),
                        },
                        tx_sign_batch: 4,
                        tx_fee_bump: Percent::from_percent(10),
                        tx_fee_margin: Percent::from_percent(250),
                        tx_resubmit_delay: Duration::from_secs(30),
                        tx_resend_delay: Duration::from_secs(10),
                        tx_sign_to_send_delay: Duration::from_secs(15),
                        tx_sign_to_poll_delay: Duration::from_secs(20),
                        tx_send_to_poll_delay: Duration::from_secs(5),
                        fetch_fee_estimate_delay: Duration::from_secs(60),
                    },
                    prover: crate::evm::prover::Config {
                        head: crate::evm::prover::head::Config {
                            safety_margin: BlockNumber::new(80),
                            block_time_min: Duration::from_millis(100),
                            block_time_avg: Duration::from_millis(240),
                            block_time_max: Duration::from_millis(1_000_000),
                            block_time_after_miss: Percent::from_percent(110),
                            block_time_after_hit: Percent::from_percent(99),
                        },
                    },
                    forwarder: crate::evm::forwarder::Config {
                        request_expiry: Duration::from_secs(3_600),
                        approve_amount: Amount::new(u128::MAX / 2),
                        batch_size: 8,
                        max_pending_count: 1_000,
                    },
                    evm_rpc: crate::evm::evm_rpc::Config {
                        rpc_services: RpcServices::ArbitrumOne(Some(vec![
                            L2MainnetService::Alchemy,
                            L2MainnetService::Ankr,
                            L2MainnetService::BlockPi,
                        ])),
                        rpc_service: RpcServices::ArbitrumOne(Some(vec![
                            L2MainnetService::Alchemy,
                        ])),
                        evm_rpc_canister_id: Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai")
                            .unwrap(),
                        evm_rpc_canister_cycles: 500_000_000_000,
                        consensus_threshold: 2,
                        get_logs_max_block_range: GET_LOGS_MAX_BLOCK_RANGE,
                    },
                    ledger: vec![
                        evm::ledger::Config {
                            token: Token::ICP,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::GLDT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::ckBTC,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::BOB,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::CHAT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::USDC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 6,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::cbBTC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(300_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(300_000),
                            gas_limit_for_approve: GasAmount::new(300_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                    ],
                },
                evm::Config {
                    chain: EvmChain::Ethereum,
                    chain_id: 1,
                    reader: crate::evm::reader::Config {
                        initial_block: None,
                        num_blocks_to_fetch_initially: 499,
                        max_num_blocks_to_fetch_per_call: GET_LOGS_MAX_BLOCK_RANGE,
                        fetch_tx_logs_task_delay: Duration::from_secs(500),
                    },
                    writer: crate::evm::writer::Config {
                        initial_nonce: TxNonce::ZERO,
                        initial_fee_estimate: TxFee {
                            max_fee_per_gas: WeiPerGas::new(9_000_000_000),
                            max_priority_fee_per_gas: WeiPerGas::new(1_000_000_000),
                        },
                        tx_sign_batch: 4,
                        tx_fee_bump: Percent::from_percent(10),
                        tx_fee_margin: Percent::from_percent(50),
                        tx_resubmit_delay: Duration::from_secs(30),
                        tx_resend_delay: Duration::from_secs(10),
                        tx_sign_to_send_delay: Duration::from_secs(15),
                        tx_sign_to_poll_delay: Duration::from_secs(40),
                        tx_send_to_poll_delay: Duration::from_secs(30),
                        fetch_fee_estimate_delay: Duration::from_secs(200),
                    },
                    prover: crate::evm::prover::Config {
                        head: crate::evm::prover::head::Config {
                            safety_margin: BlockNumber::new(4),
                            block_time_min: Duration::from_millis(1_000),
                            block_time_avg: Duration::from_millis(12_000),
                            block_time_max: Duration::from_millis(1_000_000),
                            block_time_after_miss: Percent::from_percent(110),
                            block_time_after_hit: Percent::from_percent(99),
                        },
                    },
                    forwarder: crate::evm::forwarder::Config {
                        request_expiry: Duration::from_secs(3_600),
                        approve_amount: Amount::new(u128::MAX / 2),
                        batch_size: 8,
                        max_pending_count: 1_000,
                    },
                    evm_rpc: crate::evm::evm_rpc::Config {
                        rpc_services: RpcServices::EthMainnet(Some(vec![
                            EthMainnetService::Alchemy,
                            EthMainnetService::Ankr,
                            EthMainnetService::BlockPi,
                        ])),
                        rpc_service: RpcServices::EthMainnet(Some(vec![
                            EthMainnetService::Alchemy,
                        ])),
                        evm_rpc_canister_id: Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai")
                            .unwrap(),
                        evm_rpc_canister_cycles: 500_000_000_000,
                        consensus_threshold: 2,
                        get_logs_max_block_range: GET_LOGS_MAX_BLOCK_RANGE,
                    },
                    ledger: vec![
                        evm::ledger::Config {
                            token: Token::ICP,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::GLDT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::CHAT,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::ckBTC,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::BOB,
                            operating_mode: OperatingMode::Minter,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Burn1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::USDC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 6,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::USDT,
                            operating_mode: OperatingMode::Locker,
                            decimals: 6,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xdAC17F958D2ee523a2206206994597C13D831ec7",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0xc5AC945a0af0768929301A27D6f2a7770995fAeb",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                        evm::ledger::Config {
                            token: Token::cbBTC,
                            operating_mode: OperatingMode::Locker,
                            decimals: 8,
                            initial_balance: Amount::ZERO,
                            erc20_address: Address::from_str(
                                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                            )
                            .unwrap(),
                            gas_limit_for_unlock_or_mint: GasAmount::new(80_000),
                            gas_limit_for_lock_or_burn: GasAmount::new(80_000),
                            gas_limit_for_approve: GasAmount::new(80_000),
                            max_tx_cost: Wei::new(100_000_000_000_000_000),
                            logger_address: Address::from_str(
                                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                            )
                            .unwrap(),
                            logger_topics: [
                                ic_sha3::Keccak256::hash(
                                    "Lock1(address,uint256,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock2(address,uint256,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes(),
                                ),
                                ic_sha3::Keccak256::hash(
                                    "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)"
                                        .as_bytes(),
                                ),
                            ],
                        },
                    ],
                },
            ],
            flow: flow::config::Config {
                max_concurrent_flows: 100,
                flows: vec![
                    // ICP
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(50_000_000),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(50_000_000),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(50_000_000),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(50_000_000),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ICP,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::ICP,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(10_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    // GLDT
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::GLDT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::GLDT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(25_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    // ckBTC
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::ckBTC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::ckBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(2),
                    },
                    // CHAT
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::CHAT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::CHAT,
                        min_amount: Amount::new(E8S),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(2),
                    },
                    // BOB
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::BOB,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::BOB,
                        min_amount: Amount::new(10_000_000),
                        max_amount: Amount::new(75_000 * E8S),
                        fee: Percent::from_permille(5),
                    },
                    // USDC
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::USDC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::USDC,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    // USDT
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::USDT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::USDT,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::USDT,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::USDT,
                        min_amount: Amount::new(E6S),
                        max_amount: Amount::new(25_000 * E6S),
                        fee: Percent::from_permille(1),
                    },
                    // cbBTC
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::IcpToEvm,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Base,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Ethereum,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                    FlowConfig {
                        direction: Direction::EvmToIcp,
                        icp_token: Token::cbBTC,
                        evm_chain: EvmChain::Arbitrum,
                        evm_token: Token::cbBTC,
                        min_amount: Amount::new(1_000),
                        max_amount: Amount::new(25_000_000),
                        fee: Percent::from_permille(5),
                    },
                ],
            },
        }
    }

    pub fn testnet() -> Self {
        let mut config = Self::mainnet();
        for evm in config.evm.iter_mut() {
            for ledger in evm.ledger.iter_mut() {
                match (evm.chain, ledger.token) {
                    (EvmChain::Base, Token::ICP) => {
                        ledger.erc20_address =
                            Address::from_str("0xa96496d9Ef442a3CF8F3e24B614b87a70ddf74f3")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xa96496d9Ef442a3CF8F3e24B614b87a70ddf74f3")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::USDC) => {
                        ledger.logger_address =
                            Address::from_str("0x38200DD4c3adbE86Be49717ccA8a3fD08466Cba6")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::USDT) => {
                        unreachable!("not supported");
                    }
                    (EvmChain::Base, Token::cbBTC) => {
                        ledger.logger_address =
                            Address::from_str("0xd543007D8415169756e8a61b2cc079369d4aB6a8")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::ckBTC) => {
                        ledger.erc20_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::GLDT) => {
                        ledger.erc20_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::BOB) => {
                        ledger.erc20_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::ICP) => {
                        ledger.erc20_address =
                            Address::from_str("0xC79221a2152136FE680f86562D0659706d23946A")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xC79221a2152136FE680f86562D0659706d23946A")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::USDC) => {
                        ledger.logger_address =
                            Address::from_str("0x3a9238e29Fe809df8f392e4DfB8606EB102C5e98")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::USDT) => {
                        unreachable!("not supported");
                    }
                    (EvmChain::Arbitrum, Token::cbBTC) => {
                        ledger.logger_address =
                            Address::from_str("0xd543007D8415169756e8a61b2cc079369d4aB6a8")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::ckBTC) => {
                        ledger.erc20_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::GLDT) => {
                        ledger.erc20_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                    }
                    (EvmChain::Arbitrum, Token::BOB) => {
                        ledger.erc20_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::ICP) => {
                        ledger.erc20_address =
                            Address::from_str("0xeBC37fa86e87C912B3f7b98FF0211992EDF42257")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xeBC37fa86e87C912B3f7b98FF0211992EDF42257")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::USDC) => {
                        ledger.logger_address =
                            Address::from_str("0xd060B59875c7eD702D48f4c35a122191379D4f85")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::USDT) => {
                        ledger.logger_address =
                            Address::from_str("0x205E3f1001bbE91971D25349ac3aA949D9Be5079")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::cbBTC) => {
                        ledger.logger_address =
                            Address::from_str("0xd543007D8415169756e8a61b2cc079369d4aB6a8")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::ckBTC) => {
                        ledger.erc20_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::GLDT) => {
                        ledger.erc20_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xB5A497b709703eC987B6879f064B02017998De1d")
                                .unwrap();
                    }
                    (EvmChain::Ethereum, Token::BOB) => {
                        ledger.erc20_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                        ledger.logger_address =
                            Address::from_str("0xc6d02fa25bC437E38099476a6856225aE5ac2C75")
                                .unwrap();
                    }
                    (EvmChain::Base, Token::CHAT)
                    | (EvmChain::Arbitrum, Token::CHAT)
                    | (EvmChain::Ethereum, Token::CHAT) => {}
                }
            }
        }

        for ledger in config.icp.ledger.iter_mut() {
            match ledger.token {
                Token::ICP => {
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::USDC => {
                    ledger.canister = Principal::from_text("7csws-aiaaa-aaaar-qaqpa-cai").unwrap();
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::USDT => {
                    ledger.canister = Principal::from_text("n4dku-tiaaa-aaaar-qboqa-cai").unwrap();
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::cbBTC => {
                    ledger.canister = Principal::from_text("n3cma-6qaaa-aaaar-qboqq-cai").unwrap();
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::ckBTC => {
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::GLDT => {
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::BOB => {
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
                Token::CHAT => {
                    ledger.fee_receiver =
                        Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
                }
            }
        }

        config
    }

    pub fn local() -> Self {
        let mut config = Self::mainnet();
        config.icp.ecdsa_key_name = "dfx_test_key".into();
        for (i, evm) in config.evm.iter_mut().enumerate() {
            evm.chain_id = 31337 + i as u64;
            evm.evm_rpc.rpc_services = RpcServices::Custom {
                chain_id: 31337 + i as u64,
                services: vec![RpcApi {
                    url: format!("http://127.0.0.1:{}", 8545 + i),
                    headers: None,
                }],
            };
            evm.evm_rpc.rpc_service = RpcServices::Custom {
                chain_id: 31337 + i as u64,
                services: vec![RpcApi {
                    url: format!("http://127.0.0.1:{}", 8545 + i),
                    headers: None,
                }],
            };
            evm.evm_rpc.consensus_threshold = 1;
            evm.prover.head.safety_margin = BlockNumber::new(10);
            evm.prover.head.block_time_after_miss = Percent::from_percent(150);
            evm.prover.head.block_time_after_hit = Percent::from_percent(50);
            evm.prover.head.block_time_min = Duration::from_millis(10);
            evm.prover.head.block_time_avg = Duration::from_millis(100);
            evm.prover.head.block_time_max = Duration::from_millis(10_000);
        }
        config
    }

    pub fn test() -> Self {
        let mut config = Self::local();
        for evm in config.evm.iter_mut() {
            evm.writer.tx_sign_to_send_delay = Duration::from_millis(0);
            evm.writer.tx_sign_to_poll_delay = Duration::from_millis(0);
            evm.writer.tx_send_to_poll_delay = Duration::from_millis(0);
        }
        for flow in config.flow.flows.iter_mut() {
            match flow.icp_token {
                Token::ICP => flow.max_amount = Amount::new(1_000_000 * E8S),
                Token::USDC => flow.max_amount = Amount::new(1_000_000 * 1_000_000),
                _ => continue,
            }
        }
        config.evm.retain(|s| s.chain == EvmChain::Base);
        config
    }
}
