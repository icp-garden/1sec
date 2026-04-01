//! This module defines an exchange rate helper to convert ETH to tokens.
use crate::{
    api::types::Token,
    numeric::{Amount, Wei},
    state::read_state,
};

/// Converts the given amount of ETH to the given token using the current
/// exchange rate.
pub fn convert_eth_to_token(eth: Wei, token: Token) -> Result<Amount, String> {
    let exchange_rate =
        read_state(|s| s.icp.exchange_rate.get(&token).cloned()).ok_or("no exchange rate")?;

    exchange_rate
        .eth_to_token(eth)
        .ok_or_else(|| format!("BUG: failed to convert ETH to {:?}", token))
}
