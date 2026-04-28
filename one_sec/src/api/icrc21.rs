//! ICRC-21 consent message generation for hardware-wallet (Ledger) signing.

use candid::{decode_one, Principal};
use icrc_ledger_types_021::icrc21::{
    errors::{ErrorInfo, Icrc21Error},
    requests::{ConsentMessageMetadata, ConsentMessageRequest},
    responses::{ConsentInfo, ConsentMessage, FieldsDisplay, Value},
};
use num_traits::ToPrimitive;
use serde::Deserialize;

use crate::api::types::{
    Chain, EvmAccount, EvmChain, IcpAccount, Token, TransferArg, TransferIcpToEvmArg,
};
use crate::state::read_state;

pub const MAX_CONSENT_MESSAGE_ARG_SIZE_BYTES: u16 = 500;

#[derive(candid::CandidType, Deserialize)]
pub struct SupportedStandard {
    pub name: String,
    pub url: String,
}

fn icrc21_error<E: std::fmt::Display>(e: E) -> Icrc21Error {
    Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
        description: e.to_string(),
    })
}

fn token_symbol(token: Token) -> String {
    format!("{:?}", token)
}

fn evm_chain_name(chain: EvmChain) -> String {
    format!("{:?}", chain)
}

fn icp_token_decimals(token: Token) -> Result<u8, Icrc21Error> {
    read_state(|s| s.icp.ledger.get(&token).map(|l| l.config.decimals)).ok_or_else(|| {
        Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: format!("Token not supported on ICP: {:?}.", token),
        })
    })
}

fn evm_token_decimals(chain: EvmChain, token: Token) -> Result<u8, Icrc21Error> {
    read_state(|s| {
        s.evm
            .get(&chain)
            .and_then(|e| e.ledger.get(&token))
            .map(|l| l.config.decimals)
    })
    .ok_or_else(|| {
        Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: format!("Token not supported on {:?}: {:?}.", chain, token),
        })
    })
}

fn text(content: impl Into<String>) -> Value {
    Value::Text {
        content: content.into(),
    }
}

fn token_amount(amount: u64, decimals: u8, symbol: impl Into<String>) -> Value {
    Value::TokenAmount {
        amount,
        decimals,
        symbol: symbol.into(),
    }
}

fn nat_to_u64(n: candid::Nat) -> Result<u64, Icrc21Error> {
    n.0.to_u64().ok_or_else(|| {
        Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: "Amount does not fit into u64.".to_string(),
        })
    })
}

fn icp_account_text(account: &IcpAccount) -> String {
    match account {
        IcpAccount::ICRC(a) => a.to_string(),
        IcpAccount::AccountId(s) => s.clone(),
    }
}

pub fn icrc21_canister_call_consent_message(
    _caller: Principal,
    r: ConsentMessageRequest,
) -> Result<ConsentInfo, Icrc21Error> {
    if r.arg.len() > MAX_CONSENT_MESSAGE_ARG_SIZE_BYTES as usize {
        return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: format!(
                "The argument size is too large. The maximum allowed size is {} bytes.",
                MAX_CONSENT_MESSAGE_ARG_SIZE_BYTES
            ),
        }));
    }

    // NOTE: caller may be anonymous: per Icrc21Agent the consent message is
    // fetched anonymously by spec.

    let fields_display = match r.method.as_str() {
        "transfer_icp_to_evm" => {
            let args = decode_one::<TransferIcpToEvmArg>(&r.arg).map_err(icrc21_error)?;
            message_for_transfer_icp_to_evm(args)?
        }
        "transfer" => {
            let args = decode_one::<TransferArg>(&r.arg).map_err(icrc21_error)?;
            message_for_transfer(args)?
        }
        _ => {
            return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
                description: format!("The call provided is not supported: {}.", r.method),
            }));
        }
    };

    Ok(ConsentInfo {
        metadata: ConsentMessageMetadata {
            language: "en".to_string(),
            utc_offset_minutes: r.user_preferences.metadata.utc_offset_minutes,
        },
        consent_message: ConsentMessage::FieldsDisplayMessage(fields_display),
    })
}

pub fn message_for_transfer_icp_to_evm(
    args: TransferIcpToEvmArg,
) -> Result<FieldsDisplay, Icrc21Error> {
    let symbol = token_symbol(args.token);
    let decimals = icp_token_decimals(args.token)?;
    let amount = nat_to_u64(args.icp_amount)?;

    let fields = vec![
        ("Source token".to_string(), text(symbol.clone())),
        (
            "Destination chain".to_string(),
            text(evm_chain_name(args.evm_chain)),
        ),
        (
            "Destination address".to_string(),
            text(args.evm_account.address),
        ),
        (
            "Amount".to_string(),
            token_amount(amount, decimals, symbol),
        ),
    ];

    Ok(FieldsDisplay {
        intent: "Transfer ICP to EVM".to_string(),
        fields,
    })
}

pub fn message_for_transfer(args: TransferArg) -> Result<FieldsDisplay, Icrc21Error> {
    if args.source.token != args.destination.token {
        return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
            description: "Source and destination tokens must match.".to_string(),
        }));
    }

    let token = args.source.token;
    let symbol = token_symbol(token);
    let amount = nat_to_u64(args.source.amount)?;

    if args.source.chain == Chain::ICP {
        let evm_chain: EvmChain = args
            .destination
            .chain
            .try_into()
            .map_err(icrc21_error)?;
        let evm_account: EvmAccount = args.destination.account.as_evm().map_err(icrc21_error)?;
        let decimals = icp_token_decimals(token)?;

        Ok(FieldsDisplay {
            intent: "Transfer ICP to EVM".to_string(),
            fields: vec![
                ("Source token".to_string(), text(symbol.clone())),
                (
                    "Destination chain".to_string(),
                    text(evm_chain_name(evm_chain)),
                ),
                (
                    "Destination address".to_string(),
                    text(evm_account.address),
                ),
                (
                    "Amount".to_string(),
                    token_amount(amount, decimals, symbol),
                ),
            ],
        })
    } else {
        let evm_chain: EvmChain = args.source.chain.try_into().map_err(icrc21_error)?;
        if args.destination.chain != Chain::ICP {
            return Err(Icrc21Error::UnsupportedCanisterCall(ErrorInfo {
                description: "Destination chain must be ICP for an EVM-to-ICP transfer."
                    .to_string(),
            }));
        }
        let icp_account = args.destination.account.as_icp().map_err(icrc21_error)?;
        let decimals = evm_token_decimals(evm_chain, token)?;

        Ok(FieldsDisplay {
            intent: "Transfer EVM to ICP".to_string(),
            fields: vec![
                (
                    "Source chain".to_string(),
                    text(evm_chain_name(evm_chain)),
                ),
                ("Source token".to_string(), text(symbol.clone())),
                (
                    "Destination address".to_string(),
                    text(icp_account_text(&icp_account)),
                ),
                (
                    "Amount".to_string(),
                    token_amount(amount, decimals, symbol),
                ),
            ],
        })
    }
}
