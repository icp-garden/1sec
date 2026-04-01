use std::time::Duration;

use crate::numeric::Amount;

#[derive(Debug, Clone)]
pub struct Config {
    pub request_expiry: Duration,
    pub approve_amount: Amount,
    pub batch_size: usize,
    pub max_pending_count: usize,
}
