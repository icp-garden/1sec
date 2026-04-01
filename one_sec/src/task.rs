//! This module defines the tasks of the canister that are scheduled and
//! executed on timer.

use candid::CandidType;
use ic_canister_log::log;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use crate::logs::ERROR;
use crate::{
    api::types::EvmChain,
    evm,
    guards::TaskGuard,
    icp,
    logs::{DEBUG, INFO},
    numeric::Timestamp,
    state::{mutate_state, read_state},
};

thread_local! {
    static TASKS: RefCell<TaskQueue> = RefCell::default();
}

/// This enum contains all the tasks of a canister.
/// There can be at most one running instance of a task at any given time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, CandidType, Deserialize)]
pub enum TaskType {
    /// Tasks related to [the ICP state machine](icp).
    Icp(icp::Task),
    /// Tasks related to [an EVM state machine](evm).
    Evm { chain: EvmChain, task: evm::Task },
}

/// A task that is scheduled to run at the given time.
#[derive(Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub struct Task {
    pub execute_at: Timestamp,
    pub task_type: TaskType,
    pub reason: String,
    pub delay: Duration,
}

/// Schedules a task for execution after the given delay.
pub fn schedule_after(delay: Duration, work: TaskType, reason: String) {
    let now = timestamp_ms();
    let execute_at = now.add(
        Timestamp::new(delay.as_millis() as u64),
        "BUG: overflow in schedule_after: execute_at",
    );

    let execution_time =
        TASKS.with(|t| t.borrow_mut().schedule_at(execute_at, work, reason, delay));
    set_global_timer(execution_time.into_inner());
}

/// Schedules a task for immediate execution.
pub fn schedule_now(work: TaskType, reason: String) {
    schedule_after(Duration::from_secs(0), work, reason)
}

/// Schedules a task as soon as possible ensuring that there is at least the
/// given delay passed since the last execution.
pub fn schedule_soon(delay: Duration, work: TaskType, reason: String) {
    let last_execution = read_state(|s| {
        s.last_task_execution
            .get(&work)
            .cloned()
            .unwrap_or(Timestamp::ZERO)
    });
    let next_execution = last_execution.add(
        Timestamp::new(delay.as_millis() as u64),
        "BUG: overflow in last_execution + delay",
    );
    let now = timestamp_ms();
    if next_execution <= now {
        schedule_now(work, reason);
    } else {
        schedule_after(
            Duration::from_millis(
                next_execution
                    .sub(now, "BUG: underflow in next_execution - now")
                    .into_inner(),
            ),
            work,
            reason,
        );
    }
}

pub fn get_all_tasks() -> Vec<TaskType> {
    let mut tasks = vec![];
    tasks.extend(&icp::Task::get_all_tasks());
    let evm_chains: Vec<_> = read_state(|s| s.evm.keys().cloned().collect());
    for chain in evm_chains {
        tasks.extend(evm::Task::get_all_tasks(chain));
    }
    tasks
}

/// Kicks off tasks after startup.
pub fn schedule_all_tasks(reason: &str) {
    for task in get_all_tasks() {
        schedule_now(task, reason.to_string());
    }
}

/// Pauses all tasks.
pub fn pause_all_tasks() {
    for task in get_all_tasks() {
        pause_task(task);
    }
}

/// Returns when the task needs to be retried after it has failed.
fn retry_delay_on_error(task_type: TaskType) -> Duration {
    let default_delay = Duration::from_secs(30);
    match task_type {
        TaskType::Icp(_) | TaskType::Evm { .. } => default_delay,
    }
}

/// Returns true if the task has been paused.
pub fn is_task_paused(task: TaskType) -> bool {
    read_state(|s| s.paused_tasks.contains(&task))
}

/// Returns all paused tasks.
pub fn paused_tasks() -> Vec<TaskType> {
    read_state(|s| s.paused_tasks.iter().cloned().collect())
}

/// Pauses the given task.
pub fn pause_task(task: TaskType) {
    mutate_state(|s| s.paused_tasks.insert(task));
}

/// Resumes the given task (undoing `pause_task()`).
pub fn resume_task(task: TaskType) {
    mutate_state(|s| s.paused_tasks.remove(&task));
    schedule_now(task, "resumed".into());
}

/// Resumes all paused tasks.
pub fn resume_all_paused_tasks() {
    for task in paused_tasks() {
        resume_task(task);
    }
}

/// Executed the first task that was scheduled to run now (or earlier).
pub fn run_tasks_on_timer() {
    let panic_err: Result<(), String> = Err("Panic in task".to_string());

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run tasks: {}",
            ic_cdk::api::canister_balance128()
        );
        return;
    }

    if let Some(task) = pop_if_ready() {
        let task_type = task.task_type;
        if is_task_paused(task_type) {
            log!(DEBUG, "skipping paused task: {:?}", task);
        } else {
            ic_cdk::spawn(async move {
                mutate_state(|s| s.last_task_execution.insert(task_type, timestamp_ms()));
                let _task_guard = match TaskGuard::new(task_type) {
                    Ok(guard) => guard,
                    Err(_err) => {
                        log!(DEBUG, "guard prevented task: {:?}", task);
                        return;
                    }
                };
                let _panic_guard = scopeguard::guard((), |_| {
                    log_and_reschedule_on_error(task_type, panic_err);
                });
                if read_state(|s| s.debug_tracing) {
                    log!(
                        DEBUG,
                        "task: {:?} delay {}s, reason: {}",
                        task.task_type,
                        task.delay.as_secs(),
                        task.reason
                    );
                }
                let start = timestamp_ms();

                let result = match task_type {
                    TaskType::Icp(task) => task.run().await,
                    TaskType::Evm { chain, task } => task.run(chain).await,
                };

                let end = timestamp_ms();

                if read_state(|s| s.debug_tracing) {
                    log!(
                        DEBUG,
                        "task: {:?} took {}ms",
                        task.task_type,
                        end.into_inner() - start.into_inner(),
                    );
                }

                log_and_reschedule_on_error(task_type, result);
                scopeguard::ScopeGuard::into_inner(_panic_guard);
            });
        }
    }
}

fn log_and_reschedule_on_error(task_type: TaskType, result: Result<(), String>) {
    if let Err(msg) = result {
        log!(ERROR, "[{:?}]: {}", task_type, msg);
        schedule_after(
            retry_delay_on_error(task_type),
            task_type,
            format!("retry after error: {}", msg),
        );
    }
}

#[derive(Clone, Debug, Default)]
struct TaskQueue {
    queue: BTreeSet<Task>,
    task_by_type: BTreeMap<TaskType, Task>,
}

impl TaskQueue {
    /// Schedules the given task at the specified time.  Returns the
    /// time that the caller should pass to the set_global_timer
    /// function.
    ///
    /// NOTE: The queue keeps only one copy of each task. If the
    /// caller submits multiple identical tasks with the same
    /// deadline, the queue keeps the task with the earliest deadline.
    pub fn schedule_at(
        &mut self,
        execute_at: Timestamp,
        task_type: TaskType,
        reason: String,
        delay: Duration,
    ) -> Timestamp {
        let old_task = self.task_by_type.get(&task_type).cloned();

        let old_deadline = old_task
            .as_ref()
            .map(|t| t.execute_at)
            .unwrap_or(Timestamp::new(u64::MAX));

        if execute_at <= old_deadline {
            if let Some(old_task) = old_task {
                self.queue.remove(&old_task);
            }
            let new_task = Task {
                execute_at,
                task_type,
                reason,
                delay,
            };
            self.task_by_type.insert(task_type, new_task.clone());
            self.queue.insert(new_task);
        }

        self.next_execution_timestamp().unwrap_or(execute_at)
    }

    fn next_execution_timestamp(&self) -> Option<Timestamp> {
        self.queue.first().map(|t| t.execute_at)
    }

    /// Removes the first task from the queue that's ready for
    /// execution.
    pub fn pop_if_ready(&mut self, now: Timestamp) -> Option<Task> {
        if self.queue.first()?.execute_at <= now {
            let task = self
                .queue
                .pop_first()
                .expect("BUG: attempt to pop from empty queue");
            self.task_by_type.remove(&task.task_type);
            Some(task)
        } else {
            None
        }
    }
}

/// Dequeues the next task ready for execution from the minter task queue.
fn pop_if_ready() -> Option<Task> {
    let now = timestamp_ms();
    let task = TASKS.with(|t| t.borrow_mut().pop_if_ready(now));
    if let Some(next_execution) = TASKS.with(|t| t.borrow().next_execution_timestamp()) {
        set_global_timer(next_execution.into_inner());
    }
    task
}

#[cfg(not(target_arch = "wasm32"))]
fn set_global_timer(_ts: u64) {}

#[cfg(target_arch = "wasm32")]
fn set_global_timer(ts: u64) {
    // SAFETY: setting the global timer is always safe; it does not
    // mutate any canister memory.
    unsafe {
        ic0::global_timer_set(ts as i64);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn timestamp_ms() -> Timestamp {
    const NANOS_PER_MS: u64 = 1_000_000;
    Timestamp::new(ic_cdk::api::time() / NANOS_PER_MS)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn timestamp_ms() -> Timestamp {
    use std::time::SystemTime;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Timestamp::new(timestamp)
}
