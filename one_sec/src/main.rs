//! The Wasm binary of the canister.

use ic_cdk::{init, post_upgrade, query, update};
use ic_http_types::{HttpRequest, HttpResponse};
use one_sec::{
    api::{
        queries,
        types::{
            CanisterCalls, EvmBlockStats, EvmChain, ForwardEvmToIcpArg, ForwardingAccount,
            ForwardingResponse, ForwardingUpdate, GetTransfersArg, IcpAccount, IcpTokenMetadata,
            InitOrUpgradeArg, Metadata, RelayProof, RelayTask, SignedForwardingTx, Transfer,
            TransferArg, TransferEvmToIcpArg, TransferFee, TransferIcpToEvmArg, TransferId,
            TransferResponse, TransferStats,
        },
        updates, Endpoint,
    },
    task::TaskType,
};

#[init]
fn init(arg: InitOrUpgradeArg) {
    updates::init(arg)
}

#[post_upgrade]
pub fn post_upgrade(arg: InitOrUpgradeArg) {
    updates::post_upgrade(arg)
}

#[export_name = "canister_global_timer"]
fn timer() {
    updates::timer()
}

#[update]
async fn transfer_icp_to_evm(arg: TransferIcpToEvmArg) -> TransferResponse {
    updates::transfer_icp_to_evm(arg).await
}

#[update]
fn transfer_evm_to_icp(arg: TransferEvmToIcpArg) -> TransferResponse {
    updates::transfer_evm_to_icp(arg)
}

#[update]
async fn transfer(arg: TransferArg) -> TransferResponse {
    updates::transfer(arg).await
}

#[update]
fn submit_relay_proof(chain: EvmChain, proofs: Vec<RelayProof>) -> Result<(), String> {
    updates::submit_relay_proof(chain, proofs)
}

#[update]
fn forward_evm_to_icp(account: ForwardEvmToIcpArg) -> Result<ForwardingResponse, String> {
    updates::forward_evm_to_icp(account)
}

#[update]
fn submit_forwarding_update(arg: ForwardingUpdate) -> Result<(), String> {
    updates::submit_forwarding_update(arg)
}

#[update]
fn run_task(task: TaskType) -> String {
    updates::run_task(task)
}

#[update]
fn schedule_all_tasks() -> String {
    updates::schedule_all_tasks()
}

#[update]
fn pause_task(task: TaskType) -> String {
    updates::pause_task(task)
}

#[update]
fn pause_all_tasks() -> String {
    updates::pause_all_tasks()
}

#[update]
fn resume_task(task: TaskType) -> String {
    updates::resume_task(task)
}

#[update]
fn resume_all_paused_tasks() -> String {
    updates::resume_all_paused_tasks()
}

#[update]
fn pause_endpoint(endpoint: Endpoint) -> String {
    updates::pause_endpoint(endpoint)
}

#[update]
fn resume_endpoint(endpoint: Endpoint) -> String {
    updates::resume_endpoint(endpoint)
}

#[update]
fn resume_all_paused_endpoints() -> String {
    updates::resume_all_paused_endpoints()
}

#[cfg(feature = "dev")]
#[update(hidden = true)]
fn upload_events(events: Vec<Vec<u8>>) -> Result<(), String> {
    one_sec::api::dev::upload_events(events)
}

#[cfg(feature = "dev")]
#[update(hidden = true)]
fn replace_events() -> Result<(), String> {
    one_sec::api::dev::replace_events()
}

#[query]
fn get_evm_address() -> Option<String> {
    queries::get_evm_address()
}

#[query]
fn get_evm_encoding(account: Option<IcpAccount>) -> String {
    queries::get_evm_encoding(account)
}

#[query]
fn get_transfer(arg: TransferId) -> Result<Transfer, String> {
    queries::get_transfer(arg)
}

#[query]
fn get_transfers(arg: GetTransfersArg) -> Result<Vec<Transfer>, String> {
    queries::get_transfers(arg)
}

#[query]
fn get_relay_tasks(chain: EvmChain) -> Vec<RelayTask> {
    queries::get_relay_tasks(chain)
}

#[query]
fn validate_forwarding_address(receiver: IcpAccount, address: String) -> Result<(), String> {
    queries::validate_forwarding_address(receiver, address)
}

#[query]
fn get_forwarding_address(receiver: IcpAccount) -> Result<String, String> {
    queries::get_forwarding_address(receiver)
}

#[query]
fn get_forwarding_accounts(chain: EvmChain, skip: u64, count: u64) -> Vec<ForwardingAccount> {
    queries::get_forwarding_accounts(chain, skip, count)
}

#[query]
fn get_forwarding_transactions(chain: EvmChain) -> Vec<SignedForwardingTx> {
    queries::get_forwarding_transactions(chain)
}

#[query]
fn get_forwarding_status(arg: ForwardEvmToIcpArg) -> Result<ForwardingResponse, String> {
    queries::get_forwarding_status(arg)
}

#[query]
fn get_events_bin(count: u64, skip: u64) -> Result<Vec<Vec<u8>>, String> {
    queries::get_events_bin(count, skip)
}

#[query]
fn get_events(count: u64, skip: u64) -> Result<Vec<String>, String> {
    queries::get_events(count, skip)
}

#[query]
fn get_evm_block_stats(chain: EvmChain) -> EvmBlockStats {
    queries::get_evm_block_stats(chain)
}

#[query]
fn get_canister_calls() -> Vec<CanisterCalls> {
    queries::get_canister_calls()
}

#[query]
fn get_wei_per_icp_rate() -> f64 {
    queries::get_wei_per_icp_rate()
}

#[query]
fn get_metadata() -> Result<Metadata, String> {
    queries::get_metadata()
}

#[query]
fn get_icp_token_metadata() -> Vec<IcpTokenMetadata> {
    queries::get_icp_token_metadata()
}

#[query]
fn get_transfer_fees() -> Vec<TransferFee> {
    queries::get_transfer_fees()
}

#[query]
fn get_transfer_stats(count: u64) -> Vec<TransferStats> {
    queries::get_transfer_stats(count)
}

#[query(hidden = true)]
fn http_request(req: HttpRequest) -> HttpResponse {
    queries::http_request(req)
}

#[query]
fn get_paused_tasks() -> Vec<TaskType> {
    queries::get_paused_tasks()
}

#[query]
fn get_paused_endpoints() -> Vec<Endpoint> {
    queries::get_paused_endpoints()
}

fn main() {}

/// Checks the real candid interface against the one declared in the did file.
#[test]
fn check_candid_interface_compatibility() {
    fn source_to_str(source: &candid_parser::utils::CandidSource) -> String {
        match source {
            candid_parser::utils::CandidSource::File(f) => {
                std::fs::read_to_string(f).unwrap_or_else(|_| "".to_string())
            }
            candid_parser::utils::CandidSource::Text(t) => t.to_string(),
        }
    }

    fn check_service_equal(
        new_name: &str,
        new: candid_parser::utils::CandidSource,
        old_name: &str,
        old: candid_parser::utils::CandidSource,
    ) {
        let new_str = source_to_str(&new);
        let old_str = source_to_str(&old);
        match candid_parser::utils::service_equal(new, old) {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "{} is not compatible with {}!\n\n\
            {}:\n\
            {}\n\n\
            {}:\n\
            {}\n",
                    new_name, old_name, new_name, new_str, old_name, old_str
                );
                panic!("{:?}", e);
            }
        }
    }

    candid::export_service!();

    let new_interface = __export_service();

    let old_interface =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("one_sec.did");

    check_service_equal(
        "Actual one_sec candid interface:",
        candid_parser::utils::CandidSource::Text(&new_interface),
        "Declared candid interface in one_sec.did file:",
        candid_parser::utils::CandidSource::File(old_interface.as_path()),
    );
}
