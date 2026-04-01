//! This module defines various numeric types for type-safe arithmetic.
pub use amount128::Amount128;
pub use amount64::Amount64;
use candid::Nat;
use evm_rpc_types::Nat256;
use std::fmt::Display;

mod amount128;
mod amount64;

/// Amount of E8S per ICP.
pub const E8S: u128 = 100_000_000;
/// Amount of E6S per USDC.
pub const E6S: u128 = 1_000_000;

/// This represents token amount in smallest possible units for that token.
/// For example:
/// - E8S for ICP.
/// - Wei for ETH.
/// - Satoshi for BTC.
pub type Amount = Amount128<AmountTag>;
#[doc(hidden)]
pub enum AmountTag {}

/// ETH amount in the smallest possible units called Wei.
pub type Wei = Amount128<WeiTag>;
#[doc(hidden)]
pub enum WeiTag {}

/// Gas cost in Wei.
pub type WeiPerGas = Amount128<WeiPerGasUnit>;
#[doc(hidden)]
pub enum WeiPerGasUnit {}

/// Transaction nonce.
pub type TxNonce = Amount64<TxNonceTag>;
#[doc(hidden)]
pub enum TxNonceTag {}

/// Gas amount attached to a transaction for execution.
pub type GasAmount = Amount64<GasUnit>;
#[doc(hidden)]
pub enum GasUnit {}

/// Block number (block height).
pub type BlockNumber = Amount64<BlockNumberTag>;
#[doc(hidden)]
pub enum BlockNumberTag {}

/// Index of a event log in a transaction.
pub type TxLogIndex = Amount64<TxLogIndexTag>;
#[doc(hidden)]
pub enum TxLogIndexTag {}

/// Timestamp in milliseconds.
pub type Timestamp = Amount64<TimestampTag>;
#[doc(hidden)]
pub enum TimestampTag {}

/// The result of a ICRC2 transfer call: block index where the transfer
/// happened.
pub type BlockIndex = Amount64<BlockIndexTag>;
#[doc(hidden)]
pub enum BlockIndexTag {}

impl<Unit> TryFrom<Nat256> for Amount64<Unit> {
    type Error = String;

    fn try_from(value: Nat256) -> Result<Self, Self::Error> {
        let nat: Nat = value.into();
        let res: u64 = nat
            .0
            .try_into()
            .map_err(|err| format!("BUG: failed to convert to u64: {}", err))?;
        Ok(Self::new(res))
    }
}

impl<Unit> TryFrom<Nat256> for Amount128<Unit> {
    type Error = String;

    fn try_from(value: Nat256) -> Result<Self, Self::Error> {
        let nat: Nat = value.into();
        let res: u128 = nat
            .0
            .try_into()
            .map_err(|err| format!("BUG: failed to convert to u128: {}", err))?;
        Ok(Self::new(res))
    }
}

/// Percent implemented in integers.
#[derive(Clone, Copy, Debug)]
pub struct Percent {
    numerator: u64,
    denominator: u64,
}

impl Percent {
    /// The result is `x / 100`.
    pub fn from_percent(x: u64) -> Self {
        Self {
            numerator: x,
            denominator: 100,
        }
    }

    /// The result is `x / 1_000`.
    pub fn from_permille(x: u64) -> Self {
        Self {
            numerator: x,
            denominator: 1000,
        }
    }

    pub fn numerator(&self) -> u64 {
        self.numerator
    }

    pub fn denominator(&self) -> u64 {
        self.denominator
    }

    pub fn as_f64(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

impl Display for Percent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", self.as_f64() * 100.0)
    }
}

impl Default for Percent {
    fn default() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }
}

/// Tries to convert a 256-bit number to `u64`.
pub fn nat256_to_u64(x: Nat256) -> Option<u64> {
    let n: Nat = x.into();
    n.0.try_into().ok()
}
