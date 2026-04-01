//! The candid types of the canister.
use candid::{CandidType, Nat, Principal};
use ic_ethereum_types::Address;
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{
    flow::trace::Trace,
    numeric::{Amount, BlockNumber},
    state::{
        InitEvmInput, InitEvmTokenInput, InitIcpInput, InitIcpTokenInput, InitInput, UpgradeInput,
    },
};

/// The chains supported by the canister.
#[derive(
    Copy,
    Clone,
    CandidType,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
)]
pub enum Chain {
    #[n(0)]
    ICP,
    #[n(1)]
    Base,
    #[n(2)]
    Arbitrum,
    #[n(3)]
    Ethereum,
}

impl TryFrom<u32> for Chain {
    type Error = String;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Chain::ICP),
            1 => Ok(Chain::Base),
            2 => Ok(Chain::Arbitrum),
            3 => Ok(Chain::Ethereum),
            _ => Err("unknown chain".to_string()),
        }
    }
}

impl From<Chain> for u32 {
    fn from(value: Chain) -> Self {
        match value {
            Chain::ICP => 0,
            Chain::Base => 1,
            Chain::Arbitrum => 2,
            Chain::Ethereum => 3,
        }
    }
}

impl From<EvmChain> for Chain {
    fn from(value: EvmChain) -> Self {
        match value {
            EvmChain::Base => Chain::Base,
            EvmChain::Arbitrum => Chain::Arbitrum,
            EvmChain::Ethereum => Chain::Ethereum,
        }
    }
}

/// All EVM compatible chains.
#[derive(
    Copy,
    Clone,
    CandidType,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
)]
pub enum EvmChain {
    #[n(0)]
    Base,
    #[n(1)]
    Arbitrum,
    #[n(2)]
    Ethereum,
}

impl TryFrom<Chain> for EvmChain {
    type Error = String;

    fn try_from(chain: Chain) -> Result<Self, Self::Error> {
        match chain {
            Chain::ICP => Err(format!("Not an EVM chain: {:?}", chain)),
            Chain::Base => Ok(EvmChain::Base),
            Chain::Arbitrum => Ok(EvmChain::Arbitrum),
            Chain::Ethereum => Ok(EvmChain::Ethereum),
        }
    }
}

/// All tokens supported by the canister.
#[allow(non_camel_case_types)]
#[derive(
    CandidType, Copy, Clone, Debug, Encode, Decode, PartialEq, Eq, Ord, PartialOrd, Deserialize,
)]
pub enum Token {
    #[n(0)]
    ICP,
    #[n(1)]
    USDC,
    #[n(2)]
    USDT,
    #[n(3)]
    cbBTC,
    #[n(4)]
    ckBTC,
    #[n(5)]
    BOB,
    #[n(6)]
    GLDT,
    #[n(7)]
    CHAT,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum IcpAccount {
    ICRC(IcrcAccount),
    AccountId(String),
}

/// A text representation of an EVM address.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EvmAccount {
    pub address: String,
}

/// Represents either an EVM account or and ICP account.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Account {
    Evm(EvmAccount),
    Icp(IcpAccount),
}

impl Account {
    pub fn as_icp(&self) -> Result<IcpAccount, String> {
        match self {
            Account::Evm(_) => Err(format!("expected an ICP account, got: {:?}", self)),
            Account::Icp(account) => Ok(account.clone()),
        }
    }

    pub fn as_evm(&self) -> Result<EvmAccount, String> {
        match self {
            Account::Evm(account) => Ok(account.clone()),
            Account::Icp(_) => Err(format!("expected an EVM account, got: {:?}", self)),
        }
    }
}

/// Represents an EVM transaction by its hash.
/// Optionally, it may also refer to a specific log event within that
/// transaction by the index of the log event.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct EvmTx {
    pub hash: String,
    pub log_index: Option<u64>,
}

/// Represents an ICP transaction in a ledger by its block index.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct IcpTx {
    pub ledger: Principal,
    pub block_index: u64,
}

/// Represents either an EVM transaction or an ICP transaction.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Tx {
    Evm(EvmTx),
    Icp(IcpTx),
}

impl Tx {
    pub fn as_icp(&self) -> Result<IcpTx, String> {
        match self {
            Tx::Evm(_) => Err(format!("expected ICP transaction, got: {:?}", self)),
            Tx::Icp(tx) => Ok(tx.clone()),
        }
    }

    pub fn as_evm(&self) -> Result<EvmTx, String> {
        match self {
            Tx::Evm(tx) => Ok(tx.clone()),
            Tx::Icp(_) => Err(format!("expected EVM transaction, got: {:?}", self)),
        }
    }
}

/// Details of an asset for bridging.
/// This is used only as input of endpoints, so no need to mark variant
/// fields as optional for backwards-compatibility.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub chain: Chain,
    pub account: Account,
    pub token: Token,
    pub amount: Nat,
    /// The transaction that's relevant for the user.
    /// - In case of a source asset this is the transaction that locked or burned
    ///   the token.
    /// - In case of a destination asset this is the transaction that unlocked or
    ///   minted the token.
    pub tx: Option<Tx>,
}

/// Details of an destination asset for bridging when its transaction hasn't
/// happened or is unknown yet.
/// This is used only as input of endpoints, so no need to mark variant
/// fields as optional for backwards-compatibility.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AssetRequest {
    pub chain: Chain,
    pub account: Account,
    pub token: Token,
    pub amount: Option<Nat>,
}

/// Details of an asset for bridging for returning to the user.
/// This is used as output of endpoints.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct AssetInfo {
    pub chain: Option<Chain>,
    pub account: Option<Account>,
    pub token: Option<Token>,
    pub amount: Nat,
    /// The transaction that's relevant for the user.
    /// - In case of a source asset this is the transaction that locked or burned
    ///   the token.
    /// - In case of a destination asset this is the transaction that unlocked or
    ///   minted the token.
    pub tx: Option<Tx>,
}

/// The current status of bridging.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum Status {
    PendingSourceTx,
    PendingDestinationTx,
    PendingRefundTx,
    Succeeded,
    Failed(ErrorMessage),
    Refunded(Tx),
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ErrorMessage {
    pub error: String,
}

#[derive(CandidType, Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TransferId {
    pub id: u64,
}

/// Details about an ongoing bridging.
#[derive(CandidType, Clone, Debug, Deserialize)]
pub struct Transfer {
    pub source: AssetInfo,
    pub destination: AssetInfo,
    pub status: Option<Status>,
    pub trace: Trace,
    pub queue_position: Option<u64>,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl Transfer {
    pub fn duration_ms(&self) -> Option<u64> {
        Some(self.end? - self.start?)
    }
}

/// An ICP to EVM transfer request.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TransferIcpToEvmArg {
    pub token: Token,
    pub icp_account: IcpAccount,
    pub icp_amount: Nat,
    pub evm_chain: EvmChain,
    pub evm_account: EvmAccount,
    pub evm_amount: Option<Nat>,
}

/// An EVM to ICP transfer request.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TransferEvmToIcpArg {
    pub token: Token,
    pub evm_chain: EvmChain,
    pub evm_account: EvmAccount,
    pub evm_amount: Nat,
    pub evm_tx: EvmTx,
    pub icp_account: IcpAccount,
    pub icp_amount: Option<Nat>,
}

/// A bridging request provided by the user.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TransferArg {
    pub source: Asset,
    pub destination: AssetRequest,
}

#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct FetchedBlock {
    pub block_height: u64,
}

/// The response of the `transfer()` endpoint.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum TransferResponse {
    /// Bridging has started and the given id can be used to look up details
    /// about it.
    Accepted(TransferId),
    /// The canister hasn't fetched event logs in order to start the requested
    /// EVM to ICP bridging. The given block number is the block number for
    /// which the canister has fetched the event logs.
    Fetching(FetchedBlock),
    /// The bridging request has failed with this error message.
    Failed(ErrorMessage),
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct ForwardEvmToIcpArg {
    pub token: Token,
    pub chain: EvmChain,
    pub address: String,
    pub receiver: IcpAccount,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub enum ForwardingStatus {
    CheckingBalance,
    LowBalance { balance: Nat, min_amount: Nat },
    Forwarding,
    Forwarded(EvmTx),
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct ForwardingResponse {
    pub done: Option<TransferId>,
    pub status: Option<ForwardingStatus>,
}

/// The filter argument of the `get_transfers()` query.
#[derive(CandidType, Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct GetTransfersArg {
    /// Returns known transfers for these accounts.
    /// The current limitation of the implementation: only one ICP account and
    /// one EVM account are supported.
    pub accounts: Vec<Account>,
    /// This is useful for pagination of the results.
    /// The canister returns `count` latest transfer excluding the first {skip} ones.
    pub skip: u64,
    /// The number of transfers to return.
    pub count: u64,
}

/// Information about a token and its ledger.
#[derive(Deserialize, CandidType, PartialEq, PartialOrd, Clone, Debug)]
pub struct TokenMetadata {
    pub token: Option<Token>,
    pub chain: Option<Chain>,
    /// The ICRC2 ledger canister or the ERC20 contract of the token.
    pub contract: String,
    /// The locker contract of the token.
    pub locker: Option<String>,
    /// The log event topics of the token.
    pub topics: Vec<Vec<u8>>,
    pub decimals: u8,
    pub balance: Nat,
    pub queue_size: u64,
    pub wei_per_token: f64,
}

#[derive(Deserialize, CandidType, PartialEq, PartialOrd, Clone, Debug)]
pub struct IcpTokenMetadata {
    pub token: Option<Token>,
    pub ledger: Principal,
    pub index: Option<Principal>,
}

/// Information about an EVM chain.
#[derive(Deserialize, CandidType, PartialEq, PartialOrd, Clone, Debug)]
pub struct EvmChainMetadata {
    pub chain: Option<EvmChain>,
    pub chain_id: u64,
    pub nonce: u64,
    pub block_time_ms: u64,
    pub block_number_safe: Option<u64>,
    pub fetch_time_safe_ms: Option<u64>,
    pub block_number_latest: Option<u64>,
    pub fetch_time_latest_ms: Option<u64>,
    pub max_fee_per_gas: u64,
    pub max_priority_fee_per_gas: u64,
    pub max_fee_per_gas_average: u64,
    pub max_priority_fee_per_gas_average: u64,
}

/// Information about the threshold ECDSA key of the canister.
#[derive(Deserialize, CandidType, PartialEq, PartialOrd, Clone, Debug)]
pub struct EcdsaMetadata {
    pub public_key_pem: String,
    pub chain_code_hex: String,
}

#[derive(Deserialize, CandidType, PartialEq, PartialOrd, Clone, Debug)]
pub struct Metadata {
    pub cycle_balance: Nat,
    pub stable_memory_bytes: u64,
    pub wasm_memory_bytes: u64,
    pub event_count: u64,
    pub event_bytes: u64,
    pub last_upgrade_time: u64,
    pub ecdsa: Option<EcdsaMetadata>,
    pub tokens: Vec<TokenMetadata>,
    pub evm_chains: Vec<EvmChainMetadata>,
}

/// The number of inter-canister call results with the given outcome.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct CanisterCallResult {
    pub label: String,
    pub count: u64,
}

/// Telemetry information about an inter-canister call.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct CanisterCalls {
    pub canister: Principal,
    pub method: String,
    pub duration_in_ms: Vec<u64>,
    pub cost_in_cycles: Vec<u64>,
    pub response_in_bytes: Vec<u64>,
    pub results: Vec<CanisterCallResult>,
}

/// A task for an off-chain relayer.
/// Currently only send-transaction tasks are supported.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub enum RelayTask {
    SendEvmTransaction { id: u64, tx: RLP },
    FetchEvmBlock { block_number: u64 },
}

/// An RLP encoded data.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct RLP {
    pub bytes: Vec<u8>,
}

/// A proof that the given `value` at the given `index` is included in an
/// Merkle-Patricia Trie (MPT) with the given root hash.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct TrieProof {
    pub root_hash: [u8; 32],
    pub index: u64,
    pub value: RLP,
    /// The nodes along the path from the leaf value to the root in the MPT.
    pub nodes: Vec<RLP>,
}

/// A proof that can be submitted by an off-chain relayer.
#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub enum RelayProof {
    /// A proof of a transaction receipt.
    EvmTransactionReceipt {
        /// The id of the original send-transaction task.
        id: u64,
        /// The hash of the block in which the transaction and its receipt are
        /// included.
        block_hash: [u8; 32],
        /// A proof that the transaction is included in a Merkle-Patricia Trie.
        tx: TrieProof,
        /// A proof that the receipt is included in a Merkle-Patricia Trie.
        receipt: TrieProof,
    },
    /// A proof that the hash of the given block header is equal to the given
    /// hash.
    EvmBlockHeader {
        block_hash: [u8; 32],
        block_header: Vec<u8>,
        hint_fee_per_gas: Option<u64>,
        hint_priority_fee_per_gas: Option<u64>,
    },

    /// A hint that a block contains transaction event logs.
    EvmBlockWithTxLogs { block_number: u64 },
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
pub struct ForwardingAccount {
    pub token: Token,
    pub chain: EvmChain,
    pub address: String,
    pub receiver: IcpAccount,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct ForwardingUpdate {
    pub chain: EvmChain,
    pub balances: Vec<ForwardingBalance>,
    pub to_sign: Vec<UnsignedForwardingTx>,
    pub forwarded: Vec<ForwardedTx>,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct ForwardingBalance {
    pub token: Token,
    pub address: String,
    pub balance: Nat,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Copy, Debug)]
pub enum RequestedTx {
    Burn,
    Lock,
    ApproveAndLock,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct UnsignedForwardingTx {
    pub token: Token,
    pub address: String,
    pub receiver: IcpAccount,
    pub amount: Nat,
    pub nonce: u64,
    pub max_fee_per_gas: u64,
    pub max_priority_fee_per_gas: u64,
    pub requested_tx: RequestedTx,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct SignedForwardingTx {
    pub token: Token,
    pub address: String,
    pub receiver: IcpAccount,
    pub nonce: u64,
    pub total_tx_cost_in_wei: u64,
    pub approve_tx: Option<RLP>,
    pub lock_or_burn_tx: RLP,
}

#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct ForwardedTx {
    pub token: Token,
    pub address: String,
    pub receiver: IcpAccount,
    pub nonce: u64,
    pub total_tx_cost_in_wei: u64,
    pub lock_or_burn_tx: EvmTx,
}

/// A type of the canister deployment.
#[derive(
    Deserialize, CandidType, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug,
)]
pub enum Deployment {
    #[n(0)]
    Local,
    #[n(1)]
    Testnet,
    #[n(2)]
    Mainnet,
    #[n(3)]
    Test,
}

/// ICP ledger parameters.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct InitIcpTokenArg {
    pub token: Token,
    pub initial_balance: Option<u64>,
}

/// Initialization data for the ICP chain.
/// This overrides the default data in the ICP config.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct InitIcpArg {
    pub ledger: Vec<InitIcpTokenArg>,
}

/// EVM ledger parameters.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct InitEvmTokenArg {
    pub token: Token,
    pub erc20_address: Option<String>,
    pub logger_address: Option<String>,
    pub initial_balance: Option<u64>,
}

/// Initialization data for an EVM chain.
/// This overrides the default data in the EVM config.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct InitEvmArg {
    pub chain: EvmChain,
    pub initial_nonce: Option<u64>,
    pub initial_block: Option<u64>,
    pub ledger: Vec<InitEvmTokenArg>,
}

/// An argument for canister init.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct InitArg {
    pub deployment: Deployment,
    pub icp: Option<InitIcpArg>,
    pub evm: Vec<InitEvmArg>,
}

/// An argument for canister upgrade.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct UpgradeArg {
    pub deployment: Deployment,
}

#[derive(Deserialize, CandidType)]
pub enum InitOrUpgradeArg {
    Init(InitArg),
    Upgrade(UpgradeArg),
}

/// Information about recently fetched EVM chain blocks.
#[derive(Deserialize, CandidType, PartialEq, Eq, Clone, Debug)]
pub struct EvmBlockStats {
    pub chain: Option<EvmChain>,
    pub block_time_ms: u64,
    pub block_number_safe: Option<u64>,
    pub fetch_time_safe_ms: Option<u64>,
    pub block_number_latest: Option<u64>,
    pub fetch_time_latest_ms: Option<u64>,
}

/// Transfer fee per each source/destination chain/token.
#[derive(Deserialize, CandidType, PartialEq, Clone, Debug)]
pub struct TransferFee {
    pub source_chain: Option<Chain>,
    pub source_token: Option<Token>,
    pub destination_chain: Option<Chain>,
    pub destination_token: Option<Token>,

    pub min_amount: Nat,
    pub max_amount: Nat,
    pub available: Option<Nat>,

    /// The latest transaction fee in source token amount.
    pub latest_transfer_fee_in_tokens: Nat,

    /// The average transaction fee in source token amount.
    pub average_transfer_fee_in_tokens: Nat,

    /// The protocol fee in percent as a floating-point number:
    /// - 100% is 1.0,
    /// - 50% is 0.5.
    pub protocol_fee_in_percent: f64,
}

#[derive(Deserialize, CandidType, PartialEq, Clone, Debug)]
pub struct TransferStats {
    pub source_chain: Option<Chain>,
    pub source_token: Option<Token>,
    pub destination_chain: Option<Chain>,
    pub destination_token: Option<Token>,

    pub count: u64,
    pub duration_ms_avg: u64,
    pub duration_ms_max: u64,
}

impl TryFrom<TransferArg> for TransferIcpToEvmArg {
    type Error = String;

    fn try_from(arg: TransferArg) -> Result<Self, Self::Error> {
        if arg.source.token != arg.destination.token {
            return Err(format!(
                "source and destination tokens must match: {:?} vs {:?}",
                arg.source.token, arg.destination.token
            ));
        }
        let token = arg.source.token;
        if arg.source.chain != Chain::ICP {
            return Err(format!("source chain is not ICP: {:?}", arg.source.chain));
        }
        let evm_chain: EvmChain = arg.destination.chain.try_into()?;
        let icp_account = arg.source.account.as_icp()?;
        let evm_account = arg.destination.account.as_evm()?;

        Ok(Self {
            token,
            icp_account,
            icp_amount: arg.source.amount,
            evm_chain,
            evm_account,
            evm_amount: arg.destination.amount,
        })
    }
}

impl TryFrom<TransferArg> for TransferEvmToIcpArg {
    type Error = String;

    fn try_from(arg: TransferArg) -> Result<Self, Self::Error> {
        if arg.source.token != arg.destination.token {
            return Err(format!(
                "source and destination tokens must match: {:?} vs {:?}",
                arg.source.token, arg.destination.token
            ));
        }
        let token = arg.source.token;
        let evm_chain: EvmChain = arg.source.chain.try_into()?;
        let evm_account = arg.source.account.as_evm()?;
        let evm_tx = arg
            .source
            .tx
            .ok_or("missing source transaction")?
            .as_evm()?;

        if arg.destination.chain != Chain::ICP {
            return Err(format!(
                "destination chain is not ICP: {:?}",
                arg.destination.chain
            ));
        }
        let icp_account = arg.destination.account.as_icp()?;

        Ok(Self {
            token,
            evm_chain,
            evm_account,
            evm_amount: arg.source.amount,
            evm_tx,
            icp_account,
            icp_amount: arg.destination.amount,
        })
    }
}

impl TryFrom<InitIcpTokenArg> for InitIcpTokenInput {
    type Error = String;

    fn try_from(arg: InitIcpTokenArg) -> Result<Self, Self::Error> {
        Ok(InitIcpTokenInput {
            token: arg.token,
            initial_balance: arg.initial_balance.map(|x| Amount::new(x as u128)),
        })
    }
}

impl TryFrom<InitEvmTokenArg> for InitEvmTokenInput {
    type Error = String;

    fn try_from(arg: InitEvmTokenArg) -> Result<Self, Self::Error> {
        Ok(InitEvmTokenInput {
            token: arg.token,
            erc20_address: arg
                .erc20_address
                .map(|x| Address::from_str(&x))
                .transpose()?,
            logger_address: arg
                .logger_address
                .map(|x| Address::from_str(&x))
                .transpose()?,
            initial_balance: arg.initial_balance.map(|x| Amount::new(x as u128)),
        })
    }
}

impl TryFrom<InitIcpArg> for InitIcpInput {
    type Error = String;

    fn try_from(arg: InitIcpArg) -> Result<Self, Self::Error> {
        let ledger: Result<Vec<_>, String> = arg
            .ledger
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        Ok(InitIcpInput { ledger: ledger? })
    }
}

impl TryFrom<InitEvmArg> for InitEvmInput {
    type Error = String;

    fn try_from(arg: InitEvmArg) -> Result<Self, Self::Error> {
        let ledger: Result<Vec<_>, String> = arg
            .ledger
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Vec<_>>()
            .into_iter()
            .collect();
        Ok(InitEvmInput {
            chain: arg.chain,
            initial_nonce: arg.initial_nonce,
            initial_block: arg.initial_block.map(BlockNumber::new),
            ledger: ledger?,
        })
    }
}

impl TryFrom<InitArg> for InitInput {
    type Error = String;

    fn try_from(arg: InitArg) -> Result<Self, Self::Error> {
        let evm: Result<Vec<_>, String> = arg
            .evm
            .into_iter()
            .map(|x| x.try_into())
            .collect::<Vec<_>>()
            .into_iter()
            .collect();

        Ok(InitInput {
            deployment: arg.deployment,
            icp: arg.icp.map(|x| x.try_into()).transpose()?,
            evm: evm?,
        })
    }
}

impl From<UpgradeArg> for UpgradeInput {
    fn from(value: UpgradeArg) -> Self {
        Self {
            deployment: value.deployment,
        }
    }
}
