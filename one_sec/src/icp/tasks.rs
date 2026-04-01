//! This module defines tasks related to the ICP state machine.
use candid::{CandidType, Principal};
use ic_secp256k1::PublicKey;
use ic_xrc_types::{Asset, AssetClass, GetExchangeRateRequest, GetExchangeRateResult};
use serde::Deserialize;
use std::time::Duration;

use crate::{
    api::types::Token,
    icp::state::ExchangeRate,
    metrics::CanisterCall,
    numeric::Wei,
    state::{mutate_state, read_state},
    task::{schedule_after, TaskType},
};

use super::ledger::{self};

/// A task of the ICP state machine.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    /// A task that initializes the ECDSA public key after canister startup.
    InitializeEcdsaPublicKey,

    /// A task that fetches new exchange rates from the exchange rate canister.
    RefreshExchangeRate,

    /// A task of a ledger state machine.
    Ledger { token: Token, task: ledger::Task },
}

impl Task {
    pub async fn run(self) -> Result<(), String> {
        match self {
            Task::InitializeEcdsaPublicKey => initialize_ecdsa_public_key_task().await,
            Task::RefreshExchangeRate => refresh_exchange_rate_task().await,
            Task::Ledger { token, task } => task.run(token).await,
        }
    }

    pub fn get_all_tasks() -> Vec<TaskType> {
        let mut tasks = vec![];
        tasks.push(TaskType::Icp(Task::InitializeEcdsaPublicKey));
        tasks.push(TaskType::Icp(Task::RefreshExchangeRate));
        let tokens: Vec<_> = read_state(|s| s.icp.ledger.keys().cloned().collect());
        for token in tokens {
            tasks.extend(&ledger::Task::get_all_tasks(token));
        }
        tasks
    }
}

async fn initialize_ecdsa_public_key_task() -> Result<(), String> {
    use ic_cdk::api::management_canister::ecdsa::{
        ecdsa_public_key, EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgument,
    };

    let key_name = read_state(|s| s.icp.config.ecdsa_key_name.clone());

    let cc = CanisterCall::new(Principal::management_canister(), "ecdsa_public_key", 0);

    let response = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![],
        key_id: EcdsaKeyId {
            curve: EcdsaCurve::Secp256k1,
            name: key_name,
        },
    })
    .await
    .map_err(|(code, err)| format!("failed to fetch ecdsa key: {:?} {}", code, err))?
    .0;

    let public_key = PublicKey::deserialize_sec1(&response.public_key)
        .map_err(|err| format!("failed to decode minter's public key: {:?}", err))?;

    cc.returned_ok();

    mutate_state(|s| {
        s.icp.ecdsa_public_key = Some(public_key);
        s.icp.chain_code = Some(response.chain_code);
    });

    Ok(())
}

/// Query the XRC canister to retrieve the last ICP/ETH rate.
/// https://github.com/dfinity/exchange-rate-canister
async fn fetch_exchange_rate(token: String) -> Result<ic_xrc_types::ExchangeRate, String> {
    const XRC_CALL_COST_CYCLES: u64 = 10_000_000_000;

    let base_asset = Asset {
        symbol: token,
        class: AssetClass::Cryptocurrency,
    };
    let quote_asset = Asset {
        symbol: "ETH".to_string(),
        class: AssetClass::Cryptocurrency,
    };

    let args = GetExchangeRateRequest {
        base_asset,
        quote_asset,
        timestamp: None,
    };

    let xrc_principal = read_state(|s| s.icp.config.xrc_canister_id);

    let cc = CanisterCall::new(xrc_principal, "get_exchange_rate", XRC_CALL_COST_CYCLES);

    let xrc_result: Result<(GetExchangeRateResult,), (i32, String)> =
        ic_cdk::api::call::call_with_payment(
            xrc_principal,
            "get_exchange_rate",
            (args,),
            XRC_CALL_COST_CYCLES,
        )
        .await
        .map_err(|(code, msg)| (code as i32, msg));

    let exchange_rate = xrc_result
        .map(|(r,)| r)
        .map_err(|(code, msg)| format!("Error while calling XRC canister ({:?}): {:?}", code, msg))?
        .map_err(|e| format!("{e:?}"))?;

    cc.returned_ok();

    Ok(exchange_rate)
}

fn normalize_to_decimals(exchange_rate: ic_xrc_types::ExchangeRate, token_decimals: i32) -> Wei {
    let rate = exchange_rate.rate as u128;
    let eth_decimals: i32 = 18;
    let rate_decimals: i32 = exchange_rate.metadata.decimals as i32;

    // The given rate is `rate / 10^(rate_decimals)`
    // We want to get ETH / TOKEN expressed in WEI:
    // `10^(eth_decimals) / 10^(token_decimals)`
    // Summing up all decimals we get:
    let decimals = eth_decimals - rate_decimals - token_decimals;

    let result = if decimals >= 0 {
        rate * 10u128.pow(decimals as u32)
    } else {
        rate / 10u128.pow((-decimals) as u32)
    };
    Wei::from(result)
}

async fn refresh_exchange_rate_for_token(token: Token, name: String) -> Result<(), String> {
    let token_decimals = read_state(|s| {
        s.icp
            .ledger
            .get(&token)
            .map(|ledger| ledger.config.decimals)
    })
    .ok_or_else(|| format!("BUG: token config not found for {:?}", token))?;

    let exchange_rate = fetch_exchange_rate(name.clone()).await?;

    let eth_per_token = normalize_to_decimals(exchange_rate, token_decimals as i32);
    mutate_state(|s| {
        s.icp
            .exchange_rate
            .insert(token, ExchangeRate { eth_per_token });
    });
    Ok(())
}

fn update_relative_exchange_rate(
    target_token: Token,
    ratio_numerator: u128,
    ratio_denominator: u128,
) -> Result<(), String> {
    mutate_state(|s| {
        let wei_per_usdc = s.icp.exchange_rate.get(&Token::USDC);
        if let Some(wei_per_usdc) = wei_per_usdc {
            let usdc_decimals = s
                .icp
                .ledger
                .get(&Token::USDC)
                .map(|ledger| ledger.config.decimals)
                .ok_or_else(|| format!("BUG: token config not found for {:?}", Token::USDC))?;

            let target_decimals = s
                .icp
                .ledger
                .get(&target_token)
                .map(|ledger| ledger.config.decimals)
                .ok_or_else(|| format!("BUG: token config not found for {:?}", target_token))?;

            // Adjust for decimal differences between USDC and target token
            // Apply the configurable ratio (numerator / denominator)
            let wei_per_target = if target_decimals >= usdc_decimals {
                let decimals_scaling_factor =
                    10_u128.pow(target_decimals.saturating_sub(usdc_decimals).into());
                wei_per_usdc
                    .eth_per_token
                    .checked_mul(ratio_numerator)
                    .ok_or_else(|| "overflow when multiplying ratio_numerator".to_string())?
                    .checked_div_floor(ratio_denominator)
                    .ok_or_else(|| {
                        "division by zero or underflow when dividing by ratio_denominator"
                            .to_string()
                    })?
                    .checked_div_floor(decimals_scaling_factor)
                    .ok_or_else(|| "underflow when adjusting for decimals scaling".to_string())?
            } else {
                let decimals_scaling_factor =
                    10_u128.pow(usdc_decimals.saturating_sub(target_decimals).into());
                wei_per_usdc
                    .eth_per_token
                    .checked_mul(ratio_numerator)
                    .ok_or_else(|| "overflow when multiplying ratio_numerator".to_string())?
                    .checked_div_floor(ratio_denominator)
                    .ok_or_else(|| {
                        "division by zero or underflow when dividing by ratio_denominator"
                            .to_string()
                    })?
                    .checked_mul(decimals_scaling_factor)
                    .ok_or_else(|| "underflow when adjusting for decimals scaling".to_string())?
            };

            s.icp.exchange_rate.insert(
                target_token,
                ExchangeRate {
                    eth_per_token: wei_per_target,
                },
            );
            Ok(())
        } else {
            Err("couldn't update eth_per_target because eth_per_usdc not found".to_string())
        }
    })
}

async fn refresh_exchange_rate_task() -> Result<(), String> {
    let result1 = refresh_exchange_rate_for_token(Token::ICP, "ICP".to_string()).await;
    let result2 = refresh_exchange_rate_for_token(Token::USDC, "USDC".to_string()).await;
    let result3 = refresh_exchange_rate_for_token(Token::USDT, "USDT".to_string()).await;
    let result4 = refresh_exchange_rate_for_token(Token::cbBTC, "BTC".to_string()).await;
    let result5 = refresh_exchange_rate_for_token(Token::ckBTC, "BTC".to_string()).await;
    result1?;
    result2?;
    result3?;
    result4?;
    result5?;
    update_relative_exchange_rate(Token::GLDT, 12, 10)?; // GLDT = 12/10 USDC
    update_relative_exchange_rate(Token::BOB, 29, 100)?; // BOB = 29/100 USDC
    update_relative_exchange_rate(Token::CHAT, 10, 100)?; // CHAT = 10/100 USDC
    let refresh_delay = Duration::from_secs(5 * 60);
    schedule_after(
        refresh_delay,
        TaskType::Icp(Task::RefreshExchangeRate),
        "recurring".into(),
    );
    Ok(())
}
