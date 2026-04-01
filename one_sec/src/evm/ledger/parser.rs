use std::array::TryFromSliceError;

use candid::Principal;
use ethnum::U256;
use ic_ethereum_types::Address;
use ic_ledger_types::AccountIdentifier;
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;

use crate::{
    api::types::{EvmChain, Token},
    evm::{
        forwarder::{Forwarded, ForwardingAddress},
        mutate_evm_state,
        reader::{TxLog, TxLogId},
    },
    flow::{
        config::FlowConfig,
        event::{Direction, Input},
        state::read_flow_config,
    },
    icp::{self, IcpAccount},
    numeric::{Amount, Wei},
    state::read_state,
};

// A partially parsed log event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreParsedTxLog {
    /// The transaction hash and the log index.
    pub id: TxLogId,
    /// The EVM account that called the lock/burn method of the contract.
    pub evm_account: Option<Address>,
    /// The amount being locked or burned.
    pub evm_amount: Option<Amount>,
    /// The recipient of the ICP tokens that will be unlocked/minted.
    pub data1: Option<[u8; 32]>,
    pub data2: Option<[u8; 32]>,
    pub data3: Option<[u8; 32]>,
    pub data4: Option<[u8; 32]>,
}

/// Partially parses the given raw log event.
pub fn pre_parse_tx_log(tx_log: TxLog) -> PreParsedTxLog {
    let mut result = PreParsedTxLog {
        id: tx_log.id,
        evm_account: None,
        evm_amount: None,
        data1: None,
        data2: None,
        data3: None,
        data4: None,
    };

    let mut chunks = tx_log.data.chunks(32);
    result.evm_account = fixed32(chunks.next()).and_then(parse_address);
    result.evm_amount = fixed32(chunks.next()).and_then(parse_wei);
    result.data1 = fixed32(chunks.next());
    result.data2 = fixed32(chunks.next());
    result.data3 = fixed32(chunks.next());
    result.data4 = fixed32(chunks.next());
    result
}

/// Completes parsing of the partially parsed log event.
pub fn parse_tx_log(
    evm_chain: EvmChain,
    evm_token: Token,
    tx_log: PreParsedTxLog,
) -> Result<Input, String> {
    let evm_account = tx_log
        .evm_account
        .ok_or_else(|| "Missing source address".to_string())?;

    let evm_amount = tx_log
        .evm_amount
        .ok_or_else(|| "Missing amount".to_string())?;

    let data1 = tx_log
        .data1
        .ok_or_else(|| "Missing destination address".to_string())?;

    let icp_account = decode_icp_account(data1, tx_log.data2.unwrap_or_default())?;

    let icp_token = evm_token;

    let direction = Direction::EvmToIcp;

    let config = read_flow_config(direction, icp_token, evm_chain, evm_token, |c| c.clone());

    let forwarded = mutate_evm_state(evm_chain, |s| {
        let key = ForwardingAddress {
            token: evm_token,
            address: evm_account,
        };
        let candidates = s.forwarder.forwarded.get_mut(&key)?;
        for c in candidates.iter() {
            if c.lock_or_burn_tx == tx_log.id.tx_hash {
                let result = c.clone();
                candidates.retain(|x| x.lock_or_burn_tx != tx_log.id.tx_hash);
                return Some(result);
            }
        }
        None
    });

    let ledger_fee = icp::ledger::read_ledger_state(icp_token, |s| s.config.transfer_fee);
    let is_market_maker = if let IcpAccount::ICRC(account) = icp_account {
        read_state(|s| s.icp.config.market_makers.contains(&account.owner))
    } else {
        false
    };
    let icp_amount = validate_and_apply_fees(
        evm_token,
        evm_amount,
        &config,
        ledger_fee,
        forwarded,
        is_market_maker,
    )?;

    Ok(Input {
        direction,
        icp_account,
        icp_token,
        icp_amount,
        evm_chain,
        evm_account,
        evm_token,
        evm_amount,
    })
}

const TAG_ICRC: u8 = 0;
const TAG_ACCOUNT_ID: u8 = 1;

/// Decodes an ICP account from EVM transaction log data.
/// Format:
/// - `data1[0]`: the tag byte that tells what is stored in the data
///    - `0`: it is a standard ICRC account: `principal + [subaccount]`.
///    - `1`: it is an ICP ledger specific account identifier.
/// - `data1[1..32] + data2`: the encoded ICP account.
pub fn decode_icp_account(data1: [u8; 32], data2: [u8; 32]) -> Result<IcpAccount, String> {
    let tag = data1[0];
    match tag {
        TAG_ICRC => {
            let principal = decode_principal(
                data1[1..32]
                    .try_into()
                    .map_err(|err: TryFromSliceError| err.to_string())?,
            )?;
            Ok(IcpAccount::ICRC(IcrcAccount {
                owner: principal,
                subaccount: if data2.iter().any(|b| *b != 0) {
                    Some(data2)
                } else {
                    None
                },
            }))
        }
        TAG_ACCOUNT_ID => {
            if data1[29..32].iter().any(|b| *b != 0) {
                return Err(format!(
                    "unexpected trailing non-zero bytes in account identifier: {:?}",
                    data1
                ));
            }

            if data2.iter().any(|b| *b != 0) {
                return Err(format!(
                    "unexpected non-zero word after account identifier: {:?}",
                    data2
                ));
            }

            Ok(IcpAccount::AccountId(
                AccountIdentifier::from_slice(&data1[1..29]).map_err(|err| err.to_string())?,
            ))
        }
        _ => Err(format!("unknown tag byte: {}", tag)),
    }
}

/// Encodes the given ICP account into bytes to be used in EVM transaction log
/// data. See `decode_icp_account` for description of the format.
pub fn encode_icp_account(account: IcpAccount) -> Vec<u8> {
    let mut result = vec![];
    match account {
        IcpAccount::ICRC(account) => {
            result.push(TAG_ICRC);
            result.extend_from_slice(&encode_principal(account.owner));
            if let Some(subaccount) = account.subaccount {
                result.extend_from_slice(&subaccount);
            }
        }
        IcpAccount::AccountId(account_id) => {
            result.push(TAG_ACCOUNT_ID);
            // Skip the first 4 bytes that correspond to CRC32 checksum.
            result.extend_from_slice(&account_id.as_bytes()[4..]);
            result.extend_from_slice(&[0, 0, 0]);
        }
    }
    assert_eq!(
        result.len() % 32,
        0,
        "BUG: incorrect encoding of {:?}",
        account
    );
    result
}

/// Decodes the given 31 bytes as a principal.
///
/// - `bytes[0]` = the actual length of the principal.
/// - `bytes[1..length+1]` = the principal itself.
/// - `bytes[length+1..32]` = zeros.
pub fn decode_principal(bytes: [u8; 31]) -> Result<Principal, String> {
    let length = bytes[0] as usize;
    if length == 0 || length > 29 {
        return Err(format!("invalid principal length: {}", length));
    }
    let principal_bytes = &bytes[1..length + 1];
    let zeros = &bytes[length + 1..];
    if zeros.iter().any(|b| *b != 0) {
        return Err("invalid trailing bytes in principal".to_string());
    }
    Principal::try_from_slice(principal_bytes).map_err(|err| err.to_string())
}

/// Encodes the given principal into 31 bytes to be used in log events.
fn encode_principal(p: Principal) -> [u8; 31] {
    let bytes = p.as_slice();
    let length = bytes.len();
    let mut result = [0_u8; 31];
    result[0] = bytes
        .len()
        .try_into()
        .unwrap_or_else(|_| panic!("BUG: principal doesn't fit into 31 bytes"));
    result[1..length + 1].copy_from_slice(bytes);
    result
}

fn get_forwarding_fee(token: Token, forwarded: Option<Forwarded>) -> Option<Amount> {
    let forwarded = forwarded?;
    let exchange_rate = read_state(|s| s.icp.exchange_rate.get(&token).cloned())?;
    exchange_rate.eth_to_token(Wei::new(forwarded.total_tx_cost_in_wei as u128))
}

fn validate_and_apply_fees(
    token: Token,
    amount: Amount,
    config: &FlowConfig,
    ledger_fee: Amount,
    forwarded: Option<Forwarded>,
    is_market_maker: bool,
) -> Result<Amount, String> {
    if amount > config.max_amount {
        return Err(format!(
            "amount too high: {}, max={}",
            amount, config.max_amount
        ));
    }
    if amount < config.min_amount {
        return Err(format!(
            "amount too low: {}, min={}",
            amount, config.min_amount
        ));
    }

    let fee_percent = if is_market_maker {
        config.fee.as_f64() / 10.0
    } else {
        config.fee.as_f64()
    };

    let protocol_fee = Amount::new((amount.as_f64() * fee_percent).round() as u128);
    let forwarding_fee = get_forwarding_fee(token, forwarded).unwrap_or_default();

    let total_fee = ledger_fee
        .add(protocol_fee, "BUG: overflow in ledger_fee + protocol_fee")
        .add(
            forwarding_fee,
            "BUG: overflow in ledger_fee + forwarding_fee",
        );

    if amount <= total_fee {
        return Err(format!("amount too low: {}, fee={}", amount, total_fee,));
    }
    let amount_after_fees = amount.sub(total_fee, "BUG: impossible");

    Ok(amount_after_fees)
}

fn fixed32(chunk: Option<&[u8]>) -> Option<[u8; 32]> {
    chunk?.try_into().ok()
}

fn parse_address(bytes: [u8; 32]) -> Option<Address> {
    Address::try_from(&bytes).ok()
}

fn parse_wei(bytes: [u8; 32]) -> Option<Amount> {
    let v128 = u128::try_from(U256::from_be_bytes(bytes)).ok()?;
    Some(Amount::new(v128))
}

#[cfg(test)]
mod tests {
    use candid::Principal;
    use ic_ledger_types::Subaccount;

    use super::*;

    #[test]
    fn test_encoding_of_icrc_account() {
        let principal =
            Principal::from_text("7e5g4-xptqs-cr6lw-37w4t-ykbwh-nsnd3-ntxta-euv3c-64gid-vjgww-7qe")
                .unwrap();
        let account = IcpAccount::ICRC(IcrcAccount {
            owner: principal,
            subaccount: None,
        });
        let encoded = encode_icp_account(account);
        let data1: [u8; 32] = encoded[0..32].try_into().unwrap();
        let data2: [u8; 32] = if encoded.len() > 32 {
            encoded[32..64].try_into().unwrap()
        } else {
            [0_u8; 32]
        };
        assert_eq!(account, decode_icp_account(data1, data2).unwrap());

        let account = IcpAccount::ICRC(IcrcAccount {
            owner: principal,
            subaccount: Some([42_u8; 32]),
        });
        let encoded = encode_icp_account(account);
        let data1: [u8; 32] = encoded[0..32].try_into().unwrap();
        let data2: [u8; 32] = if encoded.len() > 32 {
            encoded[32..64].try_into().unwrap()
        } else {
            [0_u8; 32]
        };
        assert_eq!(account, decode_icp_account(data1, data2).unwrap())
    }

    #[test]
    fn test_encoding_of_account_id() {
        let principal =
            Principal::from_text("7e5g4-xptqs-cr6lw-37w4t-ykbwh-nsnd3-ntxta-euv3c-64gid-vjgww-7qe")
                .unwrap();
        let account =
            IcpAccount::AccountId(AccountIdentifier::new(&principal, &Subaccount([0_u8; 32])));
        let encoded = encode_icp_account(account);
        let data1: [u8; 32] = encoded[0..32].try_into().unwrap();
        let data2 = [0_u8; 32];
        assert_eq!(encoded.len(), 32);
        assert_eq!(account, decode_icp_account(data1, data2).unwrap());

        let account =
            IcpAccount::AccountId(AccountIdentifier::new(&principal, &Subaccount([42_u8; 32])));
        let encoded = encode_icp_account(account);
        let data1: [u8; 32] = encoded[0..32].try_into().unwrap();
        let data2 = [0_u8; 32];
        assert_eq!(encoded.len(), 32);
        assert_eq!(account, decode_icp_account(data1, data2).unwrap())
    }
}
