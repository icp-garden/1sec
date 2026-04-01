use askama::Template;
use candid::Principal;

use crate::{api::types::Token, evm, numeric::Wei, state::State};

#[derive(Template)]
#[template(path = "dashboard.html")]
#[derive(Clone)]
pub struct DashboardTemplate {
    pub ecdsa_key_name: String,
    pub minter_address: String,
    pub eth_per_icp: Wei,
    pub eth_per_usdc: Wei,
    pub xrc_canister_id: Principal,
}

impl DashboardTemplate {
    pub fn from_state(state: &State) -> Self {
        DashboardTemplate {
            ecdsa_key_name: state.icp.config.ecdsa_key_name.clone(),
            minter_address: state
                .icp
                .ecdsa_public_key
                .clone()
                .map(|public_key| evm::derive_address_from_public_key(&public_key).to_string())
                .unwrap_or_default(),
            eth_per_icp: state
                .icp
                .exchange_rate
                .get(&Token::ICP)
                .map(|x| x.eth_per_token)
                .unwrap_or(Wei::ZERO),
            eth_per_usdc: state
                .icp
                .exchange_rate
                .get(&Token::USDC)
                .map(|x| x.eth_per_token)
                .unwrap_or(Wei::ZERO),
            xrc_canister_id: state.icp.config.xrc_canister_id,
        }
    }
}
