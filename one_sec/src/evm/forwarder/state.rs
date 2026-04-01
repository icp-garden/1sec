use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ic_ethereum_types::Address;
use ic_management_canister_types_private::DerivationPath;
use ic_secp256k1::PublicKey;

use crate::{
    api::types::{RequestedTx, Token},
    evm::{forwarder::config::Config, tx::SignedEip1559TransactionRequest, TxFee, TxHash},
    icp::IcpAccount,
    numeric::{Amount, Timestamp, TxNonce},
};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ForwardingAddress {
    pub token: Token,
    pub address: Address,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unconfirmed {
    pub address: ForwardingAddress,
    pub receiver: IcpAccount,
    pub time: Timestamp,
}

#[derive(Clone, Debug)]
pub struct SigningData {
    pub token: Token,
    pub sender: Address,
    pub receiver: IcpAccount,
    pub ecdsa_public_key: PublicKey,
    pub derivation_path: DerivationPath,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SigningArgs {
    pub nonce: TxNonce,
    pub amount: Amount,
    pub fee: TxFee,
    pub requested_tx: RequestedTx,
}

#[derive(Clone, Debug)]
pub struct Signing {
    pub data: SigningData,
    pub args: BTreeMap<TxNonce, VecDeque<SigningArgs>>,
}

#[derive(Clone, Debug)]
pub struct Signed {
    pub nonce: u64,
    pub receiver: IcpAccount,
    pub total_tx_cost_in_wei: u64,
    pub approve_tx: Option<SignedEip1559TransactionRequest>,
    pub lock_or_burn_tx: SignedEip1559TransactionRequest,
}

#[derive(Clone, Debug)]
pub struct Forwarded {
    pub nonce: u64,
    pub total_tx_cost_in_wei: u64,
    pub lock_or_burn_tx: TxHash,
}

/// The state of the EVM writer state machine.
#[derive(Debug)]
pub struct State {
    pub unconfirmed_queue: VecDeque<Unconfirmed>,
    pub unconfirmed_set: BTreeSet<ForwardingAddress>,

    pub balance: BTreeMap<ForwardingAddress, Amount>,

    pub signing_queue: VecDeque<ForwardingAddress>,
    pub signing_map: BTreeMap<ForwardingAddress, Signing>,

    pub signed: BTreeMap<ForwardingAddress, Vec<Signed>>,

    pub forwarded: BTreeMap<ForwardingAddress, Vec<Forwarded>>,

    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            unconfirmed_queue: Default::default(),
            unconfirmed_set: Default::default(),
            balance: Default::default(),
            signing_queue: Default::default(),
            signing_map: Default::default(),
            signed: Default::default(),
            forwarded: Default::default(),
            config,
        }
    }
}
