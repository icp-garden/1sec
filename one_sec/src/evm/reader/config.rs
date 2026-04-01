use std::time::Duration;

use crate::numeric::BlockNumber;

/// The configuration parameters of the reader state machine.
#[derive(Debug, Clone)]
pub struct Config {
    /// The initial block for fetching event logs after canister installation.
    pub initial_block: Option<BlockNumber>,

    /// If `initial_block` is not specified, the this is used to compute the
    /// initial block going back from the current safe block.
    pub num_blocks_to_fetch_initially: usize,

    /// The largest range of blocks for which the event logs are queried in a
    /// single call.
    pub max_num_blocks_to_fetch_per_call: usize,

    /// The delay for scheduling the event log fetching task.
    pub fetch_tx_logs_task_delay: Duration,
}
