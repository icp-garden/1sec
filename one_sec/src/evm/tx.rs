//! This module defines standard EVM types.

use crate::numeric::{BlockNumber, GasAmount, TxNonce, Wei, WeiPerGas};
use candid::Nat;
use ethnum::u256;
use evm_rpc_types::Nat256;
use ic_ethereum_types::Address;
use ic_secp256k1::{PublicKey, RecoveryId};
use minicbor::{Decode, Encode};
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter, LowerHex, UpperHex};

#[derive(Clone, Default, Eq, PartialEq, Hash, Debug, Decode, Encode)]
#[cbor(transparent)]
pub struct AccessList(#[n(0)] pub Vec<AccessListItem>);

#[derive(Clone, Eq, PartialEq, Hash, Debug, Decode, Encode)]
#[cbor(transparent)]
pub struct StorageKey(#[cbor(n(0), with = "minicbor::bytes")] pub [u8; 32]);

#[derive(Clone, Eq, PartialEq, Hash, Debug, Decode, Encode)]
pub struct AccessListItem {
    /// Accessed address
    #[n(0)]
    pub address: Address,
    /// Accessed storage keys
    #[n(1)]
    pub storage_keys: Vec<StorageKey>,
}

/// <https://eips.ethereum.org/EIPS/eip-1559>
#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct Eip1559TransactionRequest {
    #[n(0)]
    pub chain_id: u64,
    #[n(1)]
    pub nonce: TxNonce,
    #[n(2)]
    pub max_priority_fee_per_gas: WeiPerGas,
    #[n(3)]
    pub max_fee_per_gas: WeiPerGas,
    #[n(4)]
    pub gas_limit: GasAmount,
    #[n(5)]
    pub destination: Address,
    #[n(6)]
    pub amount: Wei,
    #[cbor(n(7), with = "minicbor::bytes")]
    pub data: Vec<u8>,
    #[n(8)]
    pub access_list: AccessList,
}

impl Eip1559TransactionRequest {
    pub fn transaction_type(&self) -> u8 {
        const EIP1559_TX_ID: u8 = 2;
        EIP1559_TX_ID
    }

    /// Hash of EIP-1559 transaction is computed as
    /// keccak256(0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list])),
    /// where `||` denotes string concatenation.
    pub fn hash(&self) -> TxHash {
        use rlp::Encodable;
        let mut bytes = self.rlp_bytes().to_vec();
        bytes.insert(0, self.transaction_type());
        TxHash(ic_sha3::Keccak256::hash(bytes))
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Default, Decode, Encode)]
pub struct Eip1559Signature {
    #[n(0)]
    pub signature_y_parity: bool,
    #[cbor(n(1), with = "icrc_cbor::u256")]
    pub r: u256,
    #[cbor(n(2), with = "icrc_cbor::u256")]
    pub s: u256,
}

#[derive(Clone, Eq, PartialEq, Debug, Decode, Encode)]
pub struct InnerSignedEip1559TransactionRequest {
    #[n(0)]
    pub transaction: Eip1559TransactionRequest,
    #[n(1)]
    pub signature: Eip1559Signature,
}

impl InnerSignedEip1559TransactionRequest {
    /// An EIP-1559 transaction is encoded as follows
    /// 0x02 || rlp([chain_id, nonce, max_priority_fee_per_gas, max_fee_per_gas, gas_limit, destination, amount, data, access_list, signature_y_parity, signature_r, signature_s]),
    /// where `||` denotes string concatenation.
    pub fn raw_bytes(&self) -> Vec<u8> {
        use rlp::Encodable;
        let mut rlp = self.rlp_bytes().to_vec();
        rlp.insert(0, self.transaction.transaction_type());
        rlp
    }
}

/// Immutable signed EIP-1559 transaction.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct SignedEip1559TransactionRequest {
    pub inner: InnerSignedEip1559TransactionRequest,
    /// Hash of the signed transaction. Since computation of the hash is an expensive operation,
    /// which involves RLP encoding and Keccak256, the value is computed once upon instantiation
    /// and memoized. It is safe to memoize the hash because the transaction is immutable.
    /// Note: Serialization should ignore this field and deserialization should call
    /// the constructor to create the correct value.
    pub memoized_hash: TxHash,
}

impl SignedEip1559TransactionRequest {
    pub fn new(transaction: Eip1559TransactionRequest, signature: Eip1559Signature) -> Self {
        let inner = InnerSignedEip1559TransactionRequest {
            transaction,
            signature,
        };
        let hash = TxHash(ic_sha3::Keccak256::hash(inner.raw_bytes()));
        Self {
            inner,
            memoized_hash: hash,
        }
    }

    pub fn raw_transaction_hex(&self) -> String {
        format!("0x{}", hex::encode(self.inner.raw_bytes()))
    }

    pub fn rlp(&self) -> Vec<u8> {
        self.inner.raw_bytes()
    }

    /// If included in a block, this hash value is used as reference to this transaction.
    pub fn hash(&self) -> TxHash {
        self.memoized_hash
    }

    pub fn transaction(&self) -> &Eip1559TransactionRequest {
        &self.inner.transaction
    }

    pub fn nonce(&self) -> TxNonce {
        self.transaction().nonce
    }
}

impl<C> minicbor::Encode<C> for SignedEip1559TransactionRequest {
    fn encode<W: minicbor::encode::Write>(
        &self,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>> {
        e.encode_with(&self.inner, ctx)?;
        Ok(())
    }
}

impl<'b, C> minicbor::Decode<'b, C> for SignedEip1559TransactionRequest {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.decode_with(ctx)
            .map(|inner: InnerSignedEip1559TransactionRequest| {
                Self::new(inner.transaction, inner.signature)
            })
    }
}

impl AsRef<Eip1559TransactionRequest> for SignedEip1559TransactionRequest {
    fn as_ref(&self) -> &Eip1559TransactionRequest {
        &self.inner.transaction
    }
}

/// A transaction hash.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Decode,
    Default,
    Deserialize,
    Encode,
    Serialize,
)]
#[serde(transparent)]
#[cbor(transparent)]
pub struct TxHash(
    #[serde(with = "ic_ethereum_types::serde_data")]
    #[cbor(n(0), with = "minicbor::bytes")]
    pub [u8; 32],
);

impl Debug for TxHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self)
    }
}

impl Display for TxHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:x}", self)
    }
}

impl LowerHex for TxHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl UpperHex for TxHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode_upper(self.0))
    }
}

impl std::str::FromStr for TxHash {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.starts_with("0x") {
            return Err("Ethereum hash doesn't start with 0x".to_string());
        }
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(&s[2..], &mut bytes)
            .map_err(|e| format!("failed to decode hash from hex: {}", e))?;
        Ok(Self(bytes))
    }
}

/// A transaction receipt.
#[derive(Clone, Eq, PartialEq, Debug, Decode, Deserialize, Encode, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TxReceipt {
    #[n(0)]
    pub tx_hash: TxHash,
    #[n(1)]
    pub status: TxStatus,
    #[n(2)]
    pub block_number: BlockNumber,
}

/// A transaction status.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Decode, Deserialize, Encode, Serialize)]
#[serde(try_from = "ethnum::u256", into = "ethnum::u256")]
pub enum TxStatus {
    /// Transaction was mined and executed successfully.
    #[n(0)]
    Success,

    /// Transaction was mined but execution failed (e.g., out-of-gas error).
    /// The amount of the transaction is returned to the sender but gas is consumed.
    /// Note that this is different from a transaction that is not mined at all: a failed transaction
    /// is part of the blockchain and the next transaction from the same sender should have an incremented
    /// transaction nonce.
    #[n(1)]
    Failure,
}

impl From<TxStatus> for ethnum::u256 {
    fn from(value: TxStatus) -> Self {
        match value {
            TxStatus::Failure => ethnum::U256::ZERO,
            TxStatus::Success => ethnum::u256::ONE,
        }
    }
}

impl TryFrom<ethnum::u256> for TxStatus {
    type Error = String;

    fn try_from(value: ethnum::u256) -> Result<Self, Self::Error> {
        match value {
            ethnum::u256::ZERO => Ok(TxStatus::Failure),
            ethnum::u256::ONE => Ok(TxStatus::Success),
            _ => Err(format!("invalid transaction status: {}", value)),
        }
    }
}

impl TryFrom<Option<Nat256>> for TxStatus {
    type Error = String;

    fn try_from(value: Option<Nat256>) -> Result<Self, Self::Error> {
        if let Some(nat256) = value.clone() {
            let nat: Nat = nat256.into();
            if nat.0.is_zero() {
                return Ok(TxStatus::Failure);
            }
            if nat.0.is_one() {
                return Ok(TxStatus::Success);
            }
        }
        Err(format!("invalid transaction status: {:?}", value))
    }
}

impl TryFrom<evm_rpc_types::TransactionReceipt> for TxReceipt {
    type Error = String;

    fn try_from(r: evm_rpc_types::TransactionReceipt) -> Result<Self, Self::Error> {
        Ok(Self {
            tx_hash: TxHash(r.transaction_hash.into()),
            status: TxStatus::try_from(r.status)?,
            block_number: BlockNumber::new(
                u64::try_from(Nat::from(r.block_number).0).map_err(|err| err.to_string())?,
            ),
        })
    }
}

impl Display for TxStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Success => write!(f, "Success"),
            TxStatus::Failure => write!(f, "Failure"),
        }
    }
}

/// Converts the given raw signature into [Eip1559Signature].
pub fn wrap_signature(
    signature: [u8; 64],
    hash: TxHash,
    ecdsa_public_key: PublicKey,
) -> Result<Eip1559Signature, String> {
    let id = compute_recovery_id(&hash.0, &signature, ecdsa_public_key)?;
    if id.is_x_reduced() {
        return Err("BUG: affine x-coordinate of r is reduced which is so unlikely to happen that it's probably a bug".to_string());
    }
    let (r_bytes, s_bytes) = split_in_two(signature);
    let r = u256::from_be_bytes(r_bytes);
    let s = u256::from_be_bytes(s_bytes);
    Ok(Eip1559Signature {
        signature_y_parity: id.is_y_odd(),
        r,
        s,
    })
}

fn compute_recovery_id(
    digest: &[u8; 32],
    signature: &[u8],
    ecdsa_public_key: PublicKey,
) -> Result<RecoveryId, String> {
    debug_assert!(
        ecdsa_public_key.verify_signature_prehashed(digest, signature),
        "failed to verify signature prehashed, digest: {:?}, signature: {:?}, public_key: {:?}",
        hex::encode(digest),
        hex::encode(signature),
        hex::encode(ecdsa_public_key.serialize_sec1(true)),
    );
    ecdsa_public_key
        .try_recovery_from_digest(digest, signature)
        .map_err(|e| {
            format!(
                "BUG: failed to recover public key {:?} from digest {:?} and signature {:?}: {:?}",
                hex::encode(ecdsa_public_key.serialize_sec1(true)),
                hex::encode(digest),
                hex::encode(signature),
                e
            )
        })
}

fn split_in_two(array: [u8; 64]) -> ([u8; 32], [u8; 32]) {
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&array[..32]);
    s.copy_from_slice(&array[32..]);
    (r, s)
}

/// Computes the EVM address from the given public key.
pub fn derive_address_from_public_key(pubkey: &PublicKey) -> Address {
    fn keccak(bytes: &[u8]) -> [u8; 32] {
        ic_sha3::Keccak256::hash(bytes)
    }
    let key_bytes = pubkey.serialize_sec1(/*compressed=*/ false);
    debug_assert_eq!(key_bytes[0], 0x04);
    let hash = keccak(&key_bytes[1..]);
    let mut addr = [0u8; 20];
    addr[..].copy_from_slice(&hash[12..32]);
    Address::new(addr)
}
