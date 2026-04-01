//! This module defines tasks related to the EVM state machine.
use candid::CandidType;
use serde::Deserialize;

use crate::api::types::EvmChain;

use super::{forwarder, prover, reader, writer};

/// A task related to the EVM state machine.
#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum Task {
    /// A task of the reader state machine.
    Reader(reader::Task),
    /// A task of the writer state machine.
    Writer(writer::Task),
    /// A task of the prover state machine.
    Prover(prover::Task),
    /// A task of the forwarder state machine.
    Forwarder(forwarder::Task),
}

impl Task {
    pub async fn run(self, chain: EvmChain) -> Result<(), String> {
        match self {
            Task::Reader(task) => task.run(chain).await,
            Task::Writer(task) => task.run(chain).await,
            Task::Prover(task) => task.run(chain).await,
            Task::Forwarder(task) => task.run(chain).await,
        }
    }

    pub fn get_all_tasks(chain: EvmChain) -> Vec<crate::task::TaskType> {
        let mut tasks = vec![];
        tasks.extend(reader::Task::get_all_tasks(chain));
        tasks.extend(writer::Task::get_all_tasks(chain));
        tasks.extend(prover::Task::get_all_tasks(chain));
        tasks.extend(forwarder::Task::get_all_tasks(chain));
        tasks
    }
}
