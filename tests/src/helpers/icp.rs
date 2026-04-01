use std::path::PathBuf;

use candid::{decode_one, encode_one, CandidType, Principal};
use one_sec::numeric::{Amount, Wei};
use pocket_ic::{management_canister::CanisterId, nonblocking::PocketIc, WasmResult};
use serde::Deserialize;

use crate::WORKSPACE_ROOT;

pub const CANISTERS: [(Canister, Location); 5] = [
    (
        Canister::OneSec,
        Location::Local {
            path: ".dfx/local/canisters/one_sec/one_sec.wasm.gz",
            id: "5okwm-giaaa-aaaar-qbn6a-cai",
        },
    ),
    (
        Canister::Ledger,
        Location::Local {
            path: "canisters/ledger.wasm.gz",
            id: "53nhb-haaaa-aaaar-qbn5q-cai",
        },
    ),
    (
        Canister::IcpLedger,
        Location::Pulled("ryjl3-tyaaa-aaaaa-aaaba-cai"),
    ),
    (
        Canister::EvmRpc,
        Location::Pulled("7hfb6-caaaa-aaaar-qadga-cai"),
    ),
    (
        Canister::ExchangeRate,
        Location::Pulled("uf6dk-hyaaa-aaaaq-qaaaq-cai"),
    ),
];

pub const ICP_LEDGER_FEE: Amount = Amount::new(10_000);
pub const USDC_LEDGER_FEE: Wei = Wei::new(0);

#[derive(CandidType)]
pub struct EmptyRecord {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Canister {
    OneSec,
    Ledger,
    IcpLedger,
    EvmRpc,
    ExchangeRate,
}

impl Canister {
    pub fn location(&self) -> Location {
        CANISTERS.iter().find(|(c, _)| c == self).unwrap().1
    }

    pub fn id(&self) -> CanisterId {
        match self.location() {
            Location::Local { id, .. } | Location::Pulled(id) => CanisterId::from_text(id).unwrap(),
        }
    }

    pub fn wasm(&self) -> Vec<u8> {
        let location = self.location();
        let path = match location {
            Location::Local { path, .. } => {
                let mut path_buf = WORKSPACE_ROOT.clone();
                path_buf.push(path);
                path_buf
            }
            Location::Pulled(id) => {
                let mut path = PathBuf::new();
                path.push(std::env::var("HOME").unwrap());
                path.push(".cache");
                path.push("dfinity");
                path.push("pulled");
                path.push(id);
                path.push("canister.wasm.gz");
                path
            }
        };
        std::fs::read(path.as_path())
            .unwrap_or_else(|_| panic!("wasm binary not found: {:?}", path))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Location {
    Local {
        path: &'static str,
        id: &'static str,
    },
    Pulled(&'static str),
}

pub async fn update<T>(
    pic: &PocketIc,
    canister: CanisterId,
    caller: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, String>
where
    T: for<'a> Deserialize<'a> + CandidType,
{
    let result = pic
        .update_call(canister, caller, method, encode_one(arg).unwrap())
        .await
        .unwrap();
    match result {
        WasmResult::Reply(reply) => Ok(decode_one(&reply).unwrap()),
        WasmResult::Reject(error) => Err(error),
    }
}

pub async fn query<T>(
    pic: &PocketIc,
    canister: CanisterId,
    caller: Principal,
    method: &str,
    arg: impl CandidType,
) -> Result<T, String>
where
    T: for<'a> Deserialize<'a> + CandidType,
{
    let result = pic
        .query_call(canister, caller, method, encode_one(arg).unwrap())
        .await
        .unwrap();
    match result {
        WasmResult::Reply(reply) => Ok(decode_one(&reply).unwrap()),
        WasmResult::Reject(error) => Err(error),
    }
}

pub mod ledger {
    use candid::{CandidType, Int, Nat, Principal};
    use icrc_ledger_types::icrc1::account::Account;
    use serde::Deserialize;

    #[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
    pub enum MetadataValue {
        Nat(Nat),
        Int(Int),
        Text(String),
        Blob(Vec<u8>),
    }

    #[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
    pub struct ArchiveOptions {
        pub num_blocks_to_archive: u64,
        pub trigger_threshold: u64,
        pub controller_id: Principal,
    }

    #[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
    pub struct InitArgs {
        pub minting_account: Account,
        pub transfer_fee: Nat,
        pub decimals: Option<u8>,
        pub token_symbol: String,
        pub token_name: String,
        pub metadata: Vec<(String, MetadataValue)>,
        pub initial_balances: Vec<(Account, Nat)>,
        pub archive_options: ArchiveOptions,
    }

    #[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
    pub struct UpgradeArgs {}

    #[derive(Deserialize, CandidType, PartialEq, Eq, Ord, PartialOrd, Clone, Debug)]
    pub enum LedgerArg {
        Init(InitArgs),
        Upgrade(Option<UpgradeArgs>),
    }
}
