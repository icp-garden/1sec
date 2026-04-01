use std::time::Duration;

use crate::{
    api::types::EvmChain,
    evm::{state::read_evm_state, tx::TxHash},
    numeric::{BlockNumber, Percent, Timestamp},
};

#[derive(Debug, Clone)]
pub struct Config {
    pub safety_margin: BlockNumber,
    pub block_time_min: Duration,
    pub block_time_avg: Duration,
    pub block_time_max: Duration,
    pub block_time_after_miss: Percent,
    pub block_time_after_hit: Percent,
}

#[derive(Debug, Default, Clone)]
pub struct Head {
    pub block_number: BlockNumber,
    pub block_hash: TxHash,
    pub fetch_time: Timestamp,
}

#[derive(Debug)]
pub struct State {
    pub latest: Option<Head>,
    pub safe: Option<Head>,
    pub hint: Option<Head>,
    pub block_time_ms: u64,
    pub config: Config,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            latest: None,
            safe: None,
            hint: None,
            block_time_ms: config.block_time_avg.as_millis() as u64,
            config,
        }
    }
}

pub fn estimated_time_to_safe_block(chain: EvmChain) -> Duration {
    let safe_blocks = read_evm_state(chain, |s| s.prover.head.config.safety_margin);
    estimated_time_for_n_blocks(chain, safe_blocks)
}

pub fn estimated_time_for_n_blocks(chain: EvmChain, n: BlockNumber) -> Duration {
    const MAX_ESTIMATE_MS: u64 = 3_600 * 1_000;
    let time = read_evm_state(chain, |s| {
        s.prover.head.config.block_time_avg.as_millis() as u64
    });
    Duration::from_millis(time.checked_mul(n.into_inner()).unwrap_or(MAX_ESTIMATE_MS))
}
