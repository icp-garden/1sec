//! The state machine interacts with other canisters on ICP.
//!
//! It has the following components:
//! - [ledger]: one state machine per ICP [Token]
//!   - maintains the token balance
//!   - supports lock, unlock, mint, and burn operations.
//!   - communicates with the ledger canister.
//! - [exchange_rate]: fetches exchange [Token] to ETH rates.
//! - the ECDSA public key that is fetched on startup.
//!
use crate::api;
#[cfg(doc)]
use crate::api::types::Token;

pub use config::Config;
pub use event::{apply_event, Event};
use ic_ledger_types::AccountIdentifier;
use icrc_ledger_types::icrc1::account::Account as IcrcAccount;
use minicbor::{Decode, Encode};
use serde::Deserialize;
pub use state::State;
pub use tasks::Task;

pub mod exchange_rate;
pub mod ledger;

mod config;
mod event;
mod state;
mod tasks;

#[derive(Copy, Clone, Debug, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum IcpAccount {
    #[n(0)]
    ICRC(#[cbor(n(0), with = "crate::cbor::account")] IcrcAccount),
    #[n(1)]
    AccountId(#[cbor(n(0), with = "crate::cbor::account_id")] AccountIdentifier),
}

impl TryFrom<api::types::IcpAccount> for IcpAccount {
    type Error = String;

    fn try_from(value: api::types::IcpAccount) -> Result<Self, Self::Error> {
        match value {
            api::types::IcpAccount::ICRC(account) => Ok(IcpAccount::ICRC(account)),
            api::types::IcpAccount::AccountId(id) => {
                let account_id = AccountIdentifier::from_hex(&id).map_err(|err| err.to_string())?;
                Ok(IcpAccount::AccountId(account_id))
            }
        }
    }
}

impl From<IcpAccount> for api::types::IcpAccount {
    fn from(value: IcpAccount) -> Self {
        match value {
            IcpAccount::ICRC(account) => api::types::IcpAccount::ICRC(account),
            IcpAccount::AccountId(account_id) => {
                api::types::IcpAccount::AccountId(account_id.to_hex())
            }
        }
    }
}
