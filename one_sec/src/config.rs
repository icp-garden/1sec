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

const FEE_RECEIVER: &str = "52mp3-qiaaa-aaaar-qbzja-cai";
const EVM_RPC_CANISTER: &str = "7hfb6-caaaa-aaaar-qadga-cai";
const MAX_TX_COST: Wei = Wei::new(100_000_000_000_000_000); // 0.1 ETH

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

// ---------------------------------------------------------------------------
// Helper: ICP ledger config
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn icp_ledger(
    token: Token,
    operating_mode: OperatingMode,
    decimals: u8,
    canister: &str,
    index_canister: Option<&str>,
    supports_account_id: bool,
    fee_threshold: Amount,
    transfer_fee: Amount,
) -> icp::ledger::Config {
    icp::ledger::Config {
        token,
        operating_mode,
        decimals,
        initial_balance: Amount::ZERO,
        canister: Principal::from_text(canister).unwrap(),
        index_canister: index_canister.map(|id| Principal::from_text(id).unwrap()),
        supports_account_id,
        fee_receiver: Principal::from_text(FEE_RECEIVER).unwrap(),
        fee_threshold,
        transfer_batch: 40,
        transfer_fee,
        transfer_task_busy_delay: Duration::from_secs(2),
        transfer_task_idle_delay: Duration::from_secs(600),
        transfer_fee_task_delay: Duration::from_secs(6 * 3_600),
    }
}

// ---------------------------------------------------------------------------
// Helper: EVM ledger config
// ---------------------------------------------------------------------------

fn burn_topics() -> [[u8; 32]; 4] {
    [
        ic_sha3::Keccak256::hash("Burn1(address,uint256,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash("Burn2(address,uint256,bytes32,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash("Burn3(address,uint256,bytes32,bytes32,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash(
            "Burn4(address,uint256,bytes32,bytes32,bytes32,bytes32)".as_bytes(),
        ),
    ]
}

fn lock_topics() -> [[u8; 32]; 4] {
    [
        ic_sha3::Keccak256::hash("Lock1(address,uint256,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash("Lock2(address,uint256,bytes32,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash("Lock3(address,uint256,bytes32,bytes32,bytes32)".as_bytes()),
        ic_sha3::Keccak256::hash(
            "Lock4(address,uint256,bytes32,bytes32,bytes32,bytes32)".as_bytes(),
        ),
    ]
}

/// Creates an EVM ledger config where erc20 and logger share the same address (minter tokens).
fn evm_ledger_minter(
    token: Token,
    decimals: u8,
    address: &str,
    gas_limit: u64,
) -> evm::ledger::Config {
    let addr = Address::from_str(address).unwrap();
    evm::ledger::Config {
        token,
        operating_mode: OperatingMode::Minter,
        decimals,
        initial_balance: Amount::ZERO,
        erc20_address: addr,
        gas_limit_for_unlock_or_mint: GasAmount::new(gas_limit),
        gas_limit_for_lock_or_burn: GasAmount::new(gas_limit),
        gas_limit_for_approve: GasAmount::new(gas_limit),
        max_tx_cost: MAX_TX_COST,
        logger_address: addr,
        logger_topics: burn_topics(),
    }
}

/// Creates an EVM ledger config for locker tokens (erc20 and logger may differ).
fn evm_ledger_locker(
    token: Token,
    decimals: u8,
    erc20_address: &str,
    logger_address: &str,
    gas_limit: u64,
) -> evm::ledger::Config {
    evm::ledger::Config {
        token,
        operating_mode: OperatingMode::Locker,
        decimals,
        initial_balance: Amount::ZERO,
        erc20_address: Address::from_str(erc20_address).unwrap(),
        gas_limit_for_unlock_or_mint: GasAmount::new(gas_limit),
        gas_limit_for_lock_or_burn: GasAmount::new(gas_limit),
        gas_limit_for_approve: GasAmount::new(gas_limit),
        max_tx_cost: MAX_TX_COST,
        logger_address: Address::from_str(logger_address).unwrap(),
        logger_topics: lock_topics(),
    }
}

// ---------------------------------------------------------------------------
// Helper: flow config for a token across all chains
// ---------------------------------------------------------------------------

fn flow_both_directions(
    token: Token,
    chain: EvmChain,
    min_amount: Amount,
    max_amount: Amount,
    fee: Percent,
) -> [FlowConfig; 2] {
    [
        FlowConfig {
            direction: Direction::IcpToEvm,
            icp_token: token,
            evm_chain: chain,
            evm_token: token,
            min_amount,
            max_amount,
            fee,
        },
        FlowConfig {
            direction: Direction::EvmToIcp,
            icp_token: token,
            evm_chain: chain,
            evm_token: token,
            min_amount,
            max_amount,
            fee,
        },
    ]
}

/// Generate flows for a token across all three chains with the same parameters.
fn flows_all_chains(
    token: Token,
    min_amount: Amount,
    max_amount: Amount,
    fee: Percent,
) -> Vec<FlowConfig> {
    let chains = [EvmChain::Base, EvmChain::Arbitrum, EvmChain::Ethereum];
    chains
        .into_iter()
        .flat_map(|chain| flow_both_directions(token, chain, min_amount, max_amount, fee))
        .collect()
}

// ---------------------------------------------------------------------------
// EVM chain configs
// ---------------------------------------------------------------------------

fn evm_rpc(
    rpc_services: RpcServices,
    rpc_service: RpcServices,
    consensus_threshold: usize,
) -> crate::evm::evm_rpc::Config {
    crate::evm::evm_rpc::Config {
        rpc_services,
        rpc_service,
        evm_rpc_canister_id: Principal::from_text(EVM_RPC_CANISTER).unwrap(),
        evm_rpc_canister_cycles: 500_000_000_000,
        consensus_threshold,
        get_logs_max_block_range: GET_LOGS_MAX_BLOCK_RANGE,
    }
}

fn default_reader() -> crate::evm::reader::Config {
    crate::evm::reader::Config {
        initial_block: None,
        num_blocks_to_fetch_initially: 499,
        max_num_blocks_to_fetch_per_call: GET_LOGS_MAX_BLOCK_RANGE,
        fetch_tx_logs_task_delay: Duration::from_secs(500),
    }
}

fn default_forwarder() -> crate::evm::forwarder::Config {
    crate::evm::forwarder::Config {
        request_expiry: Duration::from_secs(3_600),
        approve_amount: Amount::new(u128::MAX / 2),
        batch_size: 8,
        max_pending_count: 1_000,
    }
}

fn base_config() -> evm::Config {
    evm::Config {
        chain: EvmChain::Base,
        chain_id: 8453,
        reader: default_reader(),
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
                safety_margin: BlockNumber::new(63),
                block_time_min: Duration::from_millis(1_000),
                block_time_avg: Duration::from_millis(1_900),
                block_time_max: Duration::from_millis(1_000_000),
                block_time_after_miss: Percent::from_percent(110),
                block_time_after_hit: Percent::from_percent(99),
            },
        },
        forwarder: default_forwarder(),
        evm_rpc: evm_rpc(
            RpcServices::BaseMainnet(Some(vec![
                L2MainnetService::Alchemy,
                L2MainnetService::Ankr,
                L2MainnetService::BlockPi,
            ])),
            RpcServices::BaseMainnet(Some(vec![L2MainnetService::Ankr])),
            2,
        ),
        ledger: vec![
            evm_ledger_minter(
                Token::ICP,
                8,
                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                100_000,
            ),
            evm_ledger_minter(
                Token::ckBTC,
                8,
                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                100_000,
            ),
            evm_ledger_minter(
                Token::BOB,
                8,
                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                100_000,
            ),
            evm_ledger_minter(
                Token::CHAT,
                8,
                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                100_000,
            ),
            evm_ledger_minter(
                Token::GLDT,
                8,
                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                100_000,
            ),
            evm_ledger_locker(
                Token::USDC,
                6,
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                100_000,
            ),
            evm_ledger_locker(
                Token::cbBTC,
                8,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                100_000,
            ),
        ],
    }
}

fn arbitrum_config() -> evm::Config {
    evm::Config {
        chain: EvmChain::Arbitrum,
        chain_id: 42161,
        reader: default_reader(),
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
                safety_margin: BlockNumber::new(500),
                block_time_min: Duration::from_millis(100),
                block_time_avg: Duration::from_millis(240),
                block_time_max: Duration::from_millis(1_000_000),
                block_time_after_miss: Percent::from_percent(110),
                block_time_after_hit: Percent::from_percent(99),
            },
        },
        forwarder: default_forwarder(),
        evm_rpc: evm_rpc(
            RpcServices::ArbitrumOne(Some(vec![
                L2MainnetService::Alchemy,
                L2MainnetService::Ankr,
                L2MainnetService::BlockPi,
            ])),
            RpcServices::ArbitrumOne(Some(vec![L2MainnetService::Alchemy])),
            2,
        ),
        ledger: vec![
            evm_ledger_minter(
                Token::ICP,
                8,
                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                300_000,
            ),
            evm_ledger_minter(
                Token::GLDT,
                8,
                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                300_000,
            ),
            evm_ledger_minter(
                Token::ckBTC,
                8,
                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                300_000,
            ),
            evm_ledger_minter(
                Token::BOB,
                8,
                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                300_000,
            ),
            evm_ledger_minter(
                Token::CHAT,
                8,
                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                300_000,
            ),
            evm_ledger_locker(
                Token::USDC,
                6,
                "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                300_000,
            ),
            evm_ledger_locker(
                Token::cbBTC,
                8,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                300_000,
            ),
        ],
    }
}

fn ethereum_config() -> evm::Config {
    evm::Config {
        chain: EvmChain::Ethereum,
        chain_id: 1,
        reader: default_reader(),
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
                safety_margin: BlockNumber::new(10),
                block_time_min: Duration::from_millis(1_000),
                block_time_avg: Duration::from_millis(12_000),
                block_time_max: Duration::from_millis(1_000_000),
                block_time_after_miss: Percent::from_percent(110),
                block_time_after_hit: Percent::from_percent(99),
            },
        },
        forwarder: default_forwarder(),
        evm_rpc: evm_rpc(
            RpcServices::EthMainnet(Some(vec![
                EthMainnetService::Alchemy,
                EthMainnetService::Ankr,
                EthMainnetService::BlockPi,
            ])),
            RpcServices::EthMainnet(Some(vec![EthMainnetService::Alchemy])),
            2,
        ),
        ledger: vec![
            evm_ledger_minter(
                Token::ICP,
                8,
                "0x00f3C42833C3170159af4E92dbb451Fb3F708917",
                80_000,
            ),
            evm_ledger_minter(
                Token::GLDT,
                8,
                "0x86856814e74456893Cfc8946BedcBb472b5fA856",
                80_000,
            ),
            evm_ledger_minter(
                Token::CHAT,
                8,
                "0xDb95092C454235E7e666c4E226dBBbCdeb499d25",
                80_000,
            ),
            evm_ledger_minter(
                Token::ckBTC,
                8,
                "0x919A41Ea07c26f0001859Bc5dcb8754068718Fb7",
                80_000,
            ),
            evm_ledger_minter(
                Token::BOB,
                8,
                "0xecc5f868AdD75F4ff9FD00bbBDE12C35BA2C9C89",
                80_000,
            ),
            evm_ledger_locker(
                Token::USDC,
                6,
                "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
                "0xAe2351B15cFf68b5863c6690dCA58Dce383bf45A",
                80_000,
            ),
            evm_ledger_locker(
                Token::USDT,
                6,
                "0xdAC17F958D2ee523a2206206994597C13D831ec7",
                "0xc5AC945a0af0768929301A27D6f2a7770995fAeb",
                80_000,
            ),
            evm_ledger_locker(
                Token::cbBTC,
                8,
                "0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf",
                "0x7744c6a83E4b43921f27d3c94a742bf9cd24c062",
                80_000,
            ),
        ],
    }
}

// ---------------------------------------------------------------------------
// Config constructors
// ---------------------------------------------------------------------------

impl Config {
    pub fn mainnet() -> Self {
        let mut flows = Vec::new();
        flows.extend(flows_all_chains(
            Token::ICP,
            Amount::new(50_000_000),
            Amount::new(10_000 * E8S),
            Percent::from_permille(2),
        ));
        // ICP on Ethereum has a higher min_amount.
        for f in flows.iter_mut() {
            if f.icp_token == Token::ICP && f.evm_chain == EvmChain::Ethereum {
                f.min_amount = Amount::new(E8S);
            }
        }
        flows.extend(flows_all_chains(
            Token::GLDT,
            Amount::new(E8S),
            Amount::new(25_000 * E8S),
            Percent::from_permille(5),
        ));
        flows.extend(flows_all_chains(
            Token::ckBTC,
            Amount::new(1_000),
            Amount::new(25_000_000),
            Percent::from_permille(2),
        ));
        flows.extend(flows_all_chains(
            Token::CHAT,
            Amount::new(E8S),
            Amount::new(75_000 * E8S),
            Percent::from_permille(2),
        ));
        flows.extend(flows_all_chains(
            Token::BOB,
            Amount::new(10_000_000),
            Amount::new(75_000 * E8S),
            Percent::from_permille(5),
        ));
        flows.extend(flows_all_chains(
            Token::USDC,
            Amount::new(E6S),
            Amount::new(25_000 * E6S),
            Percent::from_permille(1),
        ));
        // USDT only on Ethereum.
        flows.extend(flow_both_directions(
            Token::USDT,
            EvmChain::Ethereum,
            Amount::new(E6S),
            Amount::new(25_000 * E6S),
            Percent::from_permille(1),
        ));
        flows.extend(flows_all_chains(
            Token::cbBTC,
            Amount::new(1_000),
            Amount::new(25_000_000),
            Percent::from_permille(5),
        ));

        Self {
            icp: icp::Config {
                ledger: vec![
                    icp_ledger(
                        Token::ICP,
                        OperatingMode::Locker,
                        8,
                        "ryjl3-tyaaa-aaaaa-aaaba-cai",
                        None,
                        true,
                        Amount::new(100_000_000),
                        Amount::new(10_000),
                    ),
                    icp_ledger(
                        Token::GLDT,
                        OperatingMode::Locker,
                        8,
                        "6c7su-kiaaa-aaaar-qaira-cai",
                        Some("oo6x4-xiaaa-aaaap-abrza-cai"),
                        false,
                        Amount::new(1_000_000_000),
                        Amount::new(10_000_000),
                    ),
                    icp_ledger(
                        Token::ckBTC,
                        OperatingMode::Locker,
                        8,
                        "mxzaz-hqaaa-aaaar-qaada-cai",
                        Some("n5wcd-faaaa-aaaar-qaaea-cai"),
                        false,
                        Amount::new(1_000_000),
                        Amount::new(10),
                    ),
                    icp_ledger(
                        Token::BOB,
                        OperatingMode::Locker,
                        8,
                        "7pail-xaaaa-aaaas-aabmq-cai",
                        None,
                        false,
                        Amount::new(E8S),
                        Amount::new(1_000_000),
                    ),
                    icp_ledger(
                        Token::CHAT,
                        OperatingMode::Locker,
                        8,
                        "2ouva-viaaa-aaaaq-aaamq-cai",
                        None,
                        false,
                        Amount::new(E8S),
                        Amount::new(100_000),
                    ),
                    icp_ledger(
                        Token::USDC,
                        OperatingMode::Minter,
                        6,
                        "53nhb-haaaa-aaaar-qbn5q-cai",
                        Some("f4e7e-pqaaa-aaaar-qbpgq-cai"),
                        false,
                        Amount::new(1_000_000),
                        Amount::ZERO,
                    ),
                    icp_ledger(
                        Token::USDT,
                        OperatingMode::Minter,
                        6,
                        "ij33n-oiaaa-aaaar-qbooa-cai",
                        Some("fvhuy-zyaaa-aaaar-qbpha-cai"),
                        false,
                        Amount::new(1_000_000),
                        Amount::ZERO,
                    ),
                    icp_ledger(
                        Token::cbBTC,
                        OperatingMode::Minter,
                        8,
                        "io25z-dqaaa-aaaar-qbooq-cai",
                        Some("fsgsm-uaaaa-aaaar-qbphq-cai"),
                        false,
                        Amount::new(1_000_000),
                        Amount::ZERO,
                    ),
                ],
                ecdsa_key_name: "key_1".into(),
                xrc_canister_id: Principal::from_text("uf6dk-hyaaa-aaaaq-qaaaq-cai").unwrap(),
                min_cycles_balance: 10_000_000_000_000,
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
            evm: vec![base_config(), arbitrum_config(), ethereum_config()],
            flow: flow::config::Config {
                max_concurrent_flows: 100,
                flows,
            },
        }
    }

    pub fn testnet() -> Self {
        let mut config = Self::mainnet();

        let testnet_fee_receiver = Principal::from_text("vy5lt-daaaa-aaaar-qblwa-cai").unwrap();
        for ledger in config.icp.ledger.iter_mut() {
            ledger.fee_receiver = testnet_fee_receiver;
            match ledger.token {
                Token::USDC => {
                    ledger.canister = Principal::from_text("7csws-aiaaa-aaaar-qaqpa-cai").unwrap();
                }
                Token::USDT => {
                    ledger.canister = Principal::from_text("n4dku-tiaaa-aaaar-qboqa-cai").unwrap();
                }
                Token::cbBTC => {
                    ledger.canister = Principal::from_text("n3cma-6qaaa-aaaar-qboqq-cai").unwrap();
                }
                _ => {}
            }
        }

        // Testnet EVM contract address overrides.
        // Tokens that share addresses across chains are grouped.
        let testnet_overrides: &[(&str, &[(EvmChain, Token)])] = &[
            (
                "0xa96496d9Ef442a3CF8F3e24B614b87a70ddf74f3",
                &[(EvmChain::Base, Token::ICP)],
            ),
            (
                "0xC79221a2152136FE680f86562D0659706d23946A",
                &[(EvmChain::Arbitrum, Token::ICP)],
            ),
            (
                "0xeBC37fa86e87C912B3f7b98FF0211992EDF42257",
                &[(EvmChain::Ethereum, Token::ICP)],
            ),
            (
                "0x9D8dE8E7Cd748F760C81199AD3b902798DA7E7bC",
                &[
                    (EvmChain::Base, Token::ckBTC),
                    (EvmChain::Arbitrum, Token::ckBTC),
                    (EvmChain::Ethereum, Token::ckBTC),
                ],
            ),
            (
                "0xB5A497b709703eC987B6879f064B02017998De1d",
                &[
                    (EvmChain::Base, Token::GLDT),
                    (EvmChain::Arbitrum, Token::GLDT),
                    (EvmChain::Ethereum, Token::GLDT),
                ],
            ),
            (
                "0xc6d02fa25bC437E38099476a6856225aE5ac2C75",
                &[
                    (EvmChain::Base, Token::BOB),
                    (EvmChain::Arbitrum, Token::BOB),
                    (EvmChain::Ethereum, Token::BOB),
                ],
            ),
        ];

        for (address, pairs) in testnet_overrides {
            let addr = Address::from_str(address).unwrap();
            for (chain, token) in *pairs {
                if let Some(evm) = config.evm.iter_mut().find(|e| e.chain == *chain) {
                    if let Some(ledger) = evm.ledger.iter_mut().find(|l| l.token == *token) {
                        ledger.erc20_address = addr;
                        ledger.logger_address = addr;
                    }
                }
            }
        }

        // Logger-only overrides (locker tokens where erc20 stays the same).
        let logger_overrides: &[(EvmChain, Token, &str)] = &[
            (
                EvmChain::Base,
                Token::USDC,
                "0x38200DD4c3adbE86Be49717ccA8a3fD08466Cba6",
            ),
            (
                EvmChain::Base,
                Token::cbBTC,
                "0xd543007D8415169756e8a61b2cc079369d4aB6a8",
            ),
            (
                EvmChain::Arbitrum,
                Token::USDC,
                "0x3a9238e29Fe809df8f392e4DfB8606EB102C5e98",
            ),
            (
                EvmChain::Arbitrum,
                Token::cbBTC,
                "0xd543007D8415169756e8a61b2cc079369d4aB6a8",
            ),
            (
                EvmChain::Ethereum,
                Token::USDC,
                "0xd060B59875c7eD702D48f4c35a122191379D4f85",
            ),
            (
                EvmChain::Ethereum,
                Token::USDT,
                "0x205E3f1001bbE91971D25349ac3aA949D9Be5079",
            ),
            (
                EvmChain::Ethereum,
                Token::cbBTC,
                "0xd543007D8415169756e8a61b2cc079369d4aB6a8",
            ),
        ];

        for (chain, token, address) in logger_overrides {
            if let Some(evm) = config.evm.iter_mut().find(|e| e.chain == *chain) {
                if let Some(ledger) = evm.ledger.iter_mut().find(|l| l.token == *token) {
                    ledger.logger_address = Address::from_str(address).unwrap();
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
