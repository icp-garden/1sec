//! This module defines the state of the flow state machine.
use ic_ethereum_types::Address;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    api::types::{self, Chain, ErrorMessage, EvmChain, Token, Transfer},
    evm::{self, TxHash},
    icp::{self, IcpAccount},
    numeric::{Amount64, Timestamp, TxLogIndex},
    state::read_state,
};

use super::{
    config::{Config, FlowConfig},
    event::{Direction, Input, InvalidInput, Operation, TxId},
    trace::{self, Trace},
};

pub type FlowId = Amount64<FlowUnit>;
pub enum FlowUnit {}

/// "Flow" is a short name for a bridging transfer request.
/// This struct stores the state of a flow.
#[derive(Debug, Clone)]
pub struct Flow {
    /// The unique id of the request.
    pub id: FlowId,
    /// The input provided by the user.
    pub input: Input,
    /// The execution steps:
    /// 1. Lock or burn.
    /// 2. Mint or unlock (correspondingly).
    ///
    /// In case of a burn of an ICP token and unlock of an EVM token, if there
    /// is not enough EVM token available on the target chain, then the second
    /// step will be a refund of the burned ICP token.
    pub step: [Step; 2],
}

impl Flow {
    pub fn is_refund(&self) -> bool {
        self.step[0].chain == self.step[1].chain
    }
}

/// One step of the execution of a flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The chain on which the step is executed.
    pub chain: Chain,
    /// The operation performed by the step.
    pub op: Operation,
    /// The progress of execution.
    pub progress: Progress,
    /// When the step started running.
    pub start: Option<Timestamp>,
    /// When the step finished running.
    pub end: Option<Timestamp>,
}

/// The progress in executing a step.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Progress {
    /// The step is planned, but hasn't started yet.
    Planned,

    /// The step is currently being executed.
    Running,

    /// The step has succeeded at the given transaction.
    Succeeded(TxId),

    /// The step has failed with the given error message
    /// at the given transaction (which may be empty).
    Failed { tx: Option<TxId>, err: String },
}

/// A bridging transfer that has invalid arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidFlow {
    pub input: InvalidInput,
}

/// The state of the flow state machine.
#[derive(Debug)]
pub struct State {
    /// An id counter that is always increasing.
    pub next_flow_id: FlowId,

    /// The flow are currently pending.
    pub pending: BTreeSet<FlowId>,

    /// All valid flows.
    pub flow: BTreeMap<FlowId, Flow>,

    /// Invalid flows.
    pub invalid_flow: BTreeMap<FlowId, InvalidFlow>,

    pub flow_by_icp_account: BTreeMap<IcpAccount, Vec<FlowId>>,
    pub flow_by_evm_account: BTreeMap<Address, Vec<FlowId>>,
    pub flow_by_tx_hash: BTreeMap<TxHash, Vec<(TxLogIndex, FlowId)>>,

    /// A trace of operations / canister calls that were executed for the given
    /// flow. It is used to improve UX and show detailed progress.
    pub traces: BTreeMap<FlowId, Trace>,

    /// The maximum number of pending flows at any time (immutable).
    pub max_concurrent_flows: usize,

    /// The configuration parameters for a source and destination token pairs
    /// (immutable).
    pub config: BTreeMap<(Direction, Token, EvmChain, Token), FlowConfig>,
}

impl State {
    pub fn new(config: Config) -> Self {
        Self {
            next_flow_id: FlowId::ZERO,
            pending: Default::default(),
            flow: Default::default(),
            invalid_flow: Default::default(),
            flow_by_icp_account: Default::default(),
            flow_by_evm_account: Default::default(),
            flow_by_tx_hash: Default::default(),
            traces: Default::default(),
            max_concurrent_flows: config.max_concurrent_flows,
            config: config
                .flows
                .into_iter()
                .map(|c| ((c.direction, c.icp_token, c.evm_chain, c.evm_token), c))
                .collect(),
        }
    }
}

/// Passes the current state of the state machine to the given function.
pub fn read_flow_config<F, R>(
    direction: Direction,
    icp_token: Token,
    evm_chain: EvmChain,
    evm_token: Token,
    f: F,
) -> R
where
    F: FnOnce(&FlowConfig) -> R,
{
    read_state(|s| {
        let config = s
            .flow
            .config
            .get(&(direction, icp_token, evm_chain, evm_token))
            .unwrap_or_else(|| {
                unreachable!(
                    "BUG: cannot find flow config: {:?} {:?} {:?} {:?}",
                    direction, icp_token, evm_chain, evm_token
                )
            });
        f(config)
    })
}

impl Flow {
    pub fn transfer_light(&self) -> Transfer {
        let (source, destination) = match self.input.direction {
            Direction::IcpToEvm => (self.icp(), self.evm()),
            Direction::EvmToIcp => (self.evm(), self.icp()),
        };
        Transfer {
            source,
            destination,
            status: Some(self.status()),
            trace: Trace { entries: vec![] },
            queue_position: None,
            start: self.step[0].start.map(|x| x.into_inner()),
            end: self.step[1].end.map(|x| x.into_inner()),
        }
    }

    pub fn transfer_detailed(&self) -> Transfer {
        let mut result = self.transfer_light();
        result.trace = trace::lookup(self.id);
        result.queue_position = self.queue_position();
        result
    }

    fn evm(&self) -> types::AssetInfo {
        let mut result = types::AssetInfo {
            chain: Some(self.input.evm_chain.into()),
            account: Some(types::Account::Evm(types::EvmAccount {
                address: self.input.evm_account.to_string(),
            })),
            token: Some(self.input.evm_token),
            amount: self.input.evm_amount.into(),
            tx: None,
        };

        for s in self.step.iter() {
            match s.progress {
                Progress::Succeeded(tx_id)
                | Progress::Failed {
                    tx: Some(tx_id), ..
                } => {
                    if let TxId::Evm(tx) = tx_id {
                        result.tx = Some(types::Tx::Evm(types::EvmTx {
                            hash: tx.tx_hash.to_string(),
                            log_index: Some(tx.index.into_inner()),
                        }));
                    }
                }
                _ => {}
            }
        }

        result
    }

    fn icp(&self) -> types::AssetInfo {
        let mut result = types::AssetInfo {
            chain: Some(Chain::ICP),
            account: Some(types::Account::Icp(self.input.icp_account.into())),
            token: Some(self.input.icp_token),
            amount: self.input.icp_amount.into(),
            tx: None,
        };
        for s in self.step.iter() {
            match s.progress {
                Progress::Succeeded(tx_id)
                | Progress::Failed {
                    tx: Some(tx_id), ..
                } => {
                    if let TxId::Icp(tx) = tx_id {
                        let ledger = icp::ledger::read_ledger_state(self.input.icp_token, |s| {
                            s.config.canister
                        });
                        result.tx = Some(types::Tx::Icp(types::IcpTx {
                            ledger,
                            block_index: tx.into_inner(),
                        }));
                    }
                }
                _ => {}
            }
        }
        result
    }

    fn status(&self) -> types::Status {
        match &self.step[0].progress {
            Progress::Succeeded(..) => {
                // Nothing to do here since the result depends on the next step.
            }
            Progress::Planned | Progress::Running => return types::Status::PendingSourceTx,
            Progress::Failed { err, .. } => {
                return types::Status::Failed(ErrorMessage { error: err.clone() })
            }
        }

        match &self.step[1].progress {
            Progress::Succeeded(tx) => {
                if self.is_refund() {
                    let tx = match tx {
                        TxId::Icp(block_index) => {
                            let ledger =
                                icp::ledger::read_ledger_state(self.input.icp_token, |s| {
                                    s.config.canister
                                });
                            types::Tx::Icp(types::IcpTx {
                                ledger,
                                block_index: block_index.into_inner(),
                            })
                        }
                        TxId::Evm(tx) => types::Tx::Evm(types::EvmTx {
                            hash: tx.tx_hash.to_string(),
                            log_index: Some(tx.index.into_inner()),
                        }),
                    };
                    types::Status::Refunded(tx)
                } else {
                    types::Status::Succeeded
                }
            }
            Progress::Planned | Progress::Running => {
                if self.is_refund() {
                    types::Status::PendingRefundTx
                } else {
                    types::Status::PendingDestinationTx
                }
            }
            Progress::Failed { err, .. } => {
                types::Status::Failed(ErrorMessage { error: err.clone() })
            }
        }
    }

    fn queue_position(&self) -> Option<u64> {
        match &self.step[1].progress {
            Progress::Running => match self.input.direction {
                Direction::IcpToEvm => {
                    if self.is_refund() {
                        icp::ledger::queue_position(self.input.icp_token, self.id)
                    } else {
                        evm::writer::queue_position(self.input.evm_chain, self.id)
                    }
                }
                Direction::EvmToIcp => icp::ledger::queue_position(self.input.icp_token, self.id),
            },
            Progress::Planned | Progress::Failed { .. } | Progress::Succeeded(_) => None,
        }
    }
}
