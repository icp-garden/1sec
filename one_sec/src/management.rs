use candid::Principal;
use ic_cdk::api::call::RejectionCode;
use ic_management_canister_types_private::DerivationPath;
use std::fmt;

use crate::{evm, metrics::CanisterCall};

/// Represents an error from a management canister call, such as
/// `sign_with_ecdsa`.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct CallError {
    method: String,
    reason: Reason,
}

impl fmt::Display for CallError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "management call '{}' failed: {}",
            self.method, self.reason
        )
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
/// The reason for the management call failure.
pub enum Reason {
    /// The call failed with an error.
    CanisterError(String),
    /// The management canister rejected the signature request (not enough
    /// cycles, the ECDSA subnet is overloaded, etc.).
    Rejected(String),
    /// The call failed with a transient error. Retrying may help.
    TransientInternalError(String),
    /// The call failed with a non-transient error. Retrying will not help.
    InternalError(String),
}

impl fmt::Display for Reason {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CanisterError(msg) => write!(fmt, "canister error: {}", msg),
            Self::Rejected(msg) => {
                write!(fmt, "the management canister rejected the call: {}", msg)
            }
            Reason::TransientInternalError(msg) => write!(fmt, "transient internal error: {}", msg),
            Reason::InternalError(msg) => write!(fmt, "internal error: {}", msg),
        }
    }
}

impl Reason {
    fn from_reject(reject_code: RejectionCode, reject_message: String) -> Self {
        match reject_code {
            RejectionCode::SysTransient => Self::TransientInternalError(reject_message),
            RejectionCode::CanisterError => Self::CanisterError(reject_message),
            RejectionCode::CanisterReject => Self::Rejected(reject_message),
            RejectionCode::NoError
            | RejectionCode::SysFatal
            | RejectionCode::DestinationInvalid
            | RejectionCode::Unknown => Self::InternalError(format!(
                "rejection code: {:?}, rejection message: {}",
                reject_code, reject_message
            )),
        }
    }
}

/// Signs a message hash using the tECDSA API.
pub async fn sign_with_ecdsa(
    key_name: String,
    derivation_path: DerivationPath,
    message_hash: evm::TxHash,
) -> Result<[u8; 64], CallError> {
    use ic_cdk::api::management_canister::ecdsa::{
        sign_with_ecdsa, EcdsaCurve, EcdsaKeyId, SignWithEcdsaArgument,
    };

    // This constant is hardcoded in sign_with_ecdsa.
    const SIGN_WITH_ECDSA_FEE: u64 = 26_153_846_153;

    let cc = CanisterCall::new(
        Principal::management_canister(),
        "sign_with_ecdsa",
        SIGN_WITH_ECDSA_FEE,
    );

    let result = sign_with_ecdsa(SignWithEcdsaArgument {
        message_hash: message_hash.0.to_vec(),
        derivation_path: derivation_path.into_inner(),
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name.clone(),
        },
    })
    .await;

    match result {
        Ok((reply,)) => {
            cc.returned_ok();
            let signature_length = reply.signature.len();
            Ok(<[u8; 64]>::try_from(reply.signature).unwrap_or_else(|_| {
                panic!(
                    "BUG: invalid signature from management canister. Expected 64 bytes but got {} bytes",
                    signature_length
                )
            }))
        }
        Err((code, msg)) => {
            cc.returned_err(&msg);
            Err(CallError {
                method: "sign_with_ecdsa".to_string(),
                reason: Reason::from_reject(code, msg),
            })
        }
    }
}
