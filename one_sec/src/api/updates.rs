//! The update endpoints of the canister.

use candid::Principal;
use ic_canister_log::log;
use ic_cdk::{api::is_controller, caller};

use crate::{
    api::types::{
        ForwardEvmToIcpArg, ForwardingResponse, ForwardingUpdate, TransferEvmToIcpArg,
        TransferIcpToEvmArg,
    },
    config::Config,
    event::{process_event, replay_events, EventType, RootEvent},
    evm::{forwarder, prover},
    flow::{self},
    logs::INFO,
    state::{mutate_state, read_state, replace_state, InitInput, State},
    storage::{self, total_event_count},
    task::{self, is_task_paused, timestamp_ms, TaskType},
};

use super::{
    is_endpoint_paused,
    queries::get_paused_endpoints,
    types::{
        Chain, Deployment, ErrorMessage, EvmChain, InitOrUpgradeArg, RelayProof, TransferArg,
        TransferResponse,
    },
    Endpoint,
};

/// Initializes the canister after creation.
pub fn init(arg: InitOrUpgradeArg) {
    let now = timestamp_ms();
    match arg {
        InitOrUpgradeArg::Init(arg) => {
            let input: InitInput = arg.try_into().unwrap();
            storage::record_event(&RootEvent {
                timestamp: now,
                event: EventType::Init(input.clone()),
            });
            log!(INFO, "[init] Initialized canister with args: {input:?}");
            let config = match input.deployment {
                Deployment::Local => Config::local(),
                Deployment::Testnet => Config::testnet(),
                Deployment::Mainnet => Config::mainnet(),
                Deployment::Test => Config::test(),
            };
            replace_state(State::new(input, config));
        }
        InitOrUpgradeArg::Upgrade(_) => ic_cdk::trap("expected init args, got upgrade"),
    }
    task::schedule_all_tasks("init");
}

/// Initializes the canister after an upgrade.
pub fn post_upgrade(arg: InitOrUpgradeArg) {
    match arg {
        InitOrUpgradeArg::Init(_) => ic_cdk::trap("expected upgrade args, got init"),
        InitOrUpgradeArg::Upgrade(arg) => {
            let start = ic_cdk::api::instruction_counter();

            let config = match arg.deployment {
                Deployment::Local => Config::local(),
                Deployment::Testnet => Config::testnet(),
                Deployment::Mainnet => Config::mainnet(),
                Deployment::Test => Config::test(),
            };

            replace_state(replay_events(config));

            process_event(EventType::Upgrade(arg.into()));

            let end = ic_cdk::api::instruction_counter();

            let event_count = total_event_count();
            let instructions_consumed = end - start;

            log!(
                INFO,
                "[upgrade]: replaying {event_count} events consumed {instructions_consumed} instructions ({} instructions per event on average)",
                instructions_consumed / event_count
            );
            task::schedule_all_tasks("post_upgrade");
        }
    }
}

/// Dispatches the scheduled tasks on timer.
pub fn timer() {
    task::run_tasks_on_timer();
}

/// The main bridging endpoint of the canister.
///
/// It is used both for ICP to EVM and EVM to ICP bridging.
/// See the top-level crate documentation for a description of bridging flows.
///
/// # Parameters
/// - `arg`: information about the source and destination assets.
///
/// # Returns
/// - the transfer id if the canister successfully started bridging.
///   The `get_transfer()` query endpoint can be used to get details about the
///   progress of bridging.
pub async fn transfer(arg: TransferArg) -> TransferResponse {
    if is_endpoint_paused(Endpoint::Transfer) {
        return TransferResponse::Failed(ErrorMessage {
            error: "Transfers are tentatively paused. Please retry later".to_string(),
        });
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run transfer: {}",
            ic_cdk::api::canister_balance128()
        );
        return TransferResponse::Failed(ErrorMessage {
            error: "Canister's cycle balance is to low.".to_string(),
        });
    }

    async fn do_transfer(arg: TransferArg) -> Result<TransferResponse, String> {
        if arg.source.chain == Chain::ICP {
            flow::endpoint::icp_to_evm(arg.try_into()?).await
        } else {
            flow::endpoint::evm_to_icp(arg.try_into()?)
        }
    }

    match do_transfer(arg).await {
        Ok(result) => result,
        Err(error) => TransferResponse::Failed(ErrorMessage { error }),
    }
}

pub async fn transfer_icp_to_evm(arg: TransferIcpToEvmArg) -> TransferResponse {
    if is_endpoint_paused(Endpoint::TransferIcpToEvm) {
        return TransferResponse::Failed(ErrorMessage {
            error: "Transfers are tentatively paused. Please retry later".to_string(),
        });
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run transfer: {}",
            ic_cdk::api::canister_balance128()
        );
        return TransferResponse::Failed(ErrorMessage {
            error: "Canister's cycle balance is to low.".to_string(),
        });
    }

    match flow::endpoint::icp_to_evm(arg).await {
        Ok(response) => response,
        Err(err) => TransferResponse::Failed(ErrorMessage { error: err }),
    }

    // async fn do_transfer(arg: TransferIcpToEvmArg) -> Result<TransferResponse, String> {
    //     let flows: Vec<_> = read_state(|s| s.flow.config.keys().cloned().collect());

    //     for (direction, icp_token, evm_chain, evm_token) in flows {
    //         if direction == Direction::IcpToEvm
    //             && arg.token == icp_token
    //             && arg.token == evm_token
    //             && arg.evm_chain == evm_chain
    //         {
    //             return flow::endpoint::icp_to_evm(arg).await;
    //         }
    //     }
    //     Err("Unsupported source and destination pair".to_string())
    // }
}

pub fn transfer_evm_to_icp(arg: TransferEvmToIcpArg) -> TransferResponse {
    if is_endpoint_paused(Endpoint::TransferEvmToIcp) {
        return TransferResponse::Failed(ErrorMessage {
            error: "Transfers are tentatively paused. Please retry later".to_string(),
        });
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run transfer: {}",
            ic_cdk::api::canister_balance128()
        );
        return TransferResponse::Failed(ErrorMessage {
            error: "Canister's cycle balance is to low.".to_string(),
        });
    }

    match flow::endpoint::evm_to_icp(arg) {
        Ok(result) => result,
        Err(error) => TransferResponse::Failed(ErrorMessage { error }),
    }
}

pub fn is_allowed_relayer(caller: &Principal) -> bool {
    read_state(|s| s.icp.config.relayers.contains(caller))
}

/// This endpoint is used by relayers to submit proofs.
/// See the documentation of [evm::prover] for more details.
pub fn submit_relay_proof(chain: EvmChain, proofs: Vec<RelayProof>) -> Result<(), String> {
    if is_endpoint_paused(Endpoint::SubmitRelayProof) {
        return Err("This endpoint is paused.".to_string());
    }

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return Err("Only a controller or a relayer can call this endpoint".to_string());
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run submit_relay_proof: {}",
            ic_cdk::api::canister_balance128()
        );
        return Err("Canister's cycle balance is to low.".to_string());
    }

    prover::endpoint::submit_relay_proofs(chain, proofs)?;
    Ok(())
}

pub fn forward_evm_to_icp(arg: ForwardEvmToIcpArg) -> Result<ForwardingResponse, String> {
    if is_endpoint_paused(Endpoint::ForwardEvmToIcp) {
        return Err("This endpoint is paused.".into());
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run add_forwarding_account: {}",
            ic_cdk::api::canister_balance128()
        );
        return Err("Canister's cycle balance is to low.".into());
    }
    forwarder::endpoint::forward_evm_to_icp(arg)
}

pub fn submit_forwarding_update(arg: ForwardingUpdate) -> Result<(), String> {
    if is_endpoint_paused(Endpoint::SubmitForwardingUpdate) {
        return Err("This endpoint is paused.".to_string());
    }

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return Err("Only a controller or a relayer can call this endpoint".to_string());
    }

    if ic_cdk::api::canister_balance128() < read_state(|s| s.icp.config.min_cycles_balance) {
        log!(
            INFO,
            "Canister's cycle balance is below threshold, refusing to run sign_forwarding_transaction: {}",
            ic_cdk::api::canister_balance128()
        );
        return Err("Canister's cycle balance is to low.".to_string());
    }
    forwarder::endpoint::submit_forwarding_update(arg)
}

/// Schedules execution of the given task.
/// Only a controller can call this endpoint.
pub fn run_task(task: TaskType) -> String {
    if is_endpoint_paused(Endpoint::RunTask) {
        return "This endpoint is paused.".to_string();
    }

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    if is_task_paused(task) {
        return "Task is paused. Resume it first.".to_string();
    }
    task::schedule_now(task, "run_task endpoint".into());
    "Ok".to_string()
}

/// Schedules execution of all tasks.
/// Only a controller or a relayer can call this endpoint.
pub fn schedule_all_tasks() -> String {
    if is_endpoint_paused(Endpoint::ScheduleAllTasks) {
        return "This endpoint is paused.".to_string();
    }

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    task::schedule_all_tasks("schedule_all_tasks");
    "Ok".to_string()
}

/// Pause all tasks.
/// Only a controller or a relayer can call this endpoint.
pub fn pause_all_tasks() -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }

    task::pause_all_tasks();
    "Ok".to_string()
}

/// Pauses the given task, such that it skips execution when scheduled.
/// Only a controller or a relayer can call this endpoint.
pub fn pause_task(task: TaskType) -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    task::pause_task(task);
    "Ok".to_string()
}

/// Undoes `pause_task()` for the given task.
/// Only a controller or a relayer can call this endpoint.
pub fn resume_task(task: TaskType) -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    task::resume_task(task);
    "Ok".to_string()
}

/// Undoes `pause_task()` for all paused tasks.
/// Only a controller or a relayer can call this endpoint.
pub fn resume_all_paused_tasks() -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    task::resume_all_paused_tasks();
    "Ok".to_string()
}

/// Pauses execution of the given endpoint such that it becomes a no-op when
/// called.
/// Only a controller or a relayer can call this endpoint.
pub fn pause_endpoint(endpoint: Endpoint) -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    mutate_state(|s| s.paused_endpoints.insert(endpoint));
    "Ok".to_string()
}

/// Undoes `pause_endpoint()` for the given endpoint.
/// Only a controller or a relayer can call this endpoint.
pub fn resume_endpoint(endpoint: Endpoint) -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    mutate_state(|s| s.paused_endpoints.remove(&endpoint));
    "Ok".to_string()
}

/// Undoes `pause_endpoint()` for all paused endpoints.
/// Only a controller or a relayer can call this endpoint.
pub fn resume_all_paused_endpoints() -> String {
    // Endpoints related to pausing are not paused.

    if !is_controller(&caller()) && !is_allowed_relayer(&caller()) {
        return "Only a controller or a relayer can call this endpoint".to_string();
    }
    for endpoint in get_paused_endpoints() {
        resume_endpoint(endpoint);
    }
    "Ok".to_string()
}
