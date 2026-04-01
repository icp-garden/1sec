#![cfg(test)]

use alloy::{hex::FromHex, primitives::Address};
use candid::{CandidType, Encode, Nat, Principal};
use evm_rpc_types::RpcServices;
use helpers::{
    evm::EvmEnv,
    http_outcalls::handle_http_outcalls,
    icp::{
        ledger::{self, InitArgs},
        query, update, Canister, ICP_LEDGER_FEE, USDC_LEDGER_FEE,
    },
};
use ic_cdk::api::management_canister::main::CanisterId;
use icp_ledger::{protobuf::BlockIndex, AccountIdentifier, LedgerCanisterInitPayload, Tokens};
use icrc_ledger_types::{
    icrc1::{
        account::Account,
        transfer::{TransferArg as IcrcTransferArg, TransferError},
    },
    icrc2::approve::{ApproveArgs, ApproveError},
};
use one_sec::{
    api::{
        types::{
            CanisterCalls, Deployment, EvmChain, ForwardEvmToIcpArg, ForwardingResponse,
            ForwardingUpdate, GetTransfersArg, IcpAccount, InitArg, InitEvmArg, InitEvmTokenArg,
            InitOrUpgradeArg, Metadata, SignedForwardingTx, Token, Transfer, TransferArg,
            TransferId, TransferResponse, UpgradeArg,
        },
        Endpoint,
    },
    config,
    numeric::{Amount, Wei, E8S},
    task::TaskType,
};
use lazy_static::lazy_static;
use pocket_ic::{nonblocking::PocketIc, PocketIcBuilder};
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{sync::Mutex, task};

mod helpers;

mod evm_tests;
mod icp_tests;

lazy_static! {
    static ref WORKSPACE_ROOT: PathBuf = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get workspace root")
        .workspace_root
        .into();
}

struct TestEnv {
    pic: Arc<Mutex<PocketIc>>,
    controller: Principal,
    user: Principal,
    users: Vec<Principal>,
    icp_ledger: CanisterId,
    usdc_ledger: CanisterId,
    one_sec: CanisterId,
    evm_rpc: CanisterId,
    xrc: CanisterId,
    evm: EvmEnv,
}

impl TestEnv {
    async fn new() -> Self {
        std::env::set_var("RUST_LOG", "error");

        let evm = EvmEnv::new().await;

        let pic = PocketIcBuilder::new()
            .with_nns_subnet()
            .with_ii_subnet()
            .build_async()
            .await;

        pic.set_time(
            SystemTime::UNIX_EPOCH
                .checked_add(Duration::from_secs(1738933200))
                .unwrap(),
        )
        .await;

        // Use an arbitrary valid principal as the controller of all canister.
        let controller =
            Principal::from_text("hpikg-6exdt-jn33w-ndty3-fc7jc-tl2lr-buih3-cs3y7-tftkp-sfp62-gqe")
                .unwrap();

        // Use an arbitrary valid principal as the user.
        let user =
            Principal::from_text("7e5g4-xptqs-cr6lw-37w4t-ykbwh-nsnd3-ntxta-euv3c-64gid-vjgww-7qe")
                .unwrap();

        // Some arbitrary valid principals for other users.
        let principals = vec![
            "ytjjg-zvl7k-kasj6-vrci3-npnmw-txo74-pgo3n-logga-ef57e-r2qfi-oqe",
            "zrkee-qug43-qelyv-v2xol-f4oog-yyjko-tz6y4-72it6-nfec5-dseqp-7ae",
            "zqh6l-42cdp-la3mg-hiko5-67rln-iengl-lvrhj-en62q-nrp36-wxeri-iae",
            "pjzfu-4t2rz-ujvms-pjlpx-2ztv2-ary25-3lqeg-c3mlv-toag7-pgqpy-4qe",
            "yj45n-oudfy-qlnhk-dlpi4-7gwek-22zv7-4kfbd-gaegn-fbphd-g2ydi-2ae",
            "mqrhn-q2xzv-xz5jh-ev4gh-xr6vl-coejm-wgrts-l2his-krfjm-ki4zv-nae",
            "6azll-6vqot-c3egz-obpg7-vmp6u-6uccx-rykj6-6ptg5-xlnsw-xnem6-gqe",
            "hl4q6-v7ksa-6bmke-qp4xl-iufta-iezck-3tamd-zik4y-dm5mw-zdmjp-cqe",
            "i4caz-ycw3l-3xgl2-pcnrt-oa7ql-in3qv-v7gau-graus-gmuej-t4aa4-wqe",
            "nwoed-ynucf-xi5ro-kh44u-c5cdg-4s7o6-wpaok-cqfza-opkih-4hjwm-uae",
            "su2bz-ndrav-amrbt-a2cfp-2u3gi-46oxr-y5lo7-7xc3s-az3lf-bmdgh-dae",
            "vtxyh-3i4p6-fdi6s-momhz-adqcs-mkz6c-42upr-maxev-imd5u-kzfxu-tqe",
            "46ufl-t6duy-45oni-moiwp-dl7qg-v6ehy-iwjj7-4kouf-rl5ch-n7edk-pae",
            "p2cwk-7uby3-vclxq-35zwo-dnzrk-lesnv-5t2yp-k3su5-2fo6l-xpps3-7ae",
            "z5e42-bcxgn-pxjmm-ejr4p-bi7i3-qhqkr-r4rre-dxzbt-p4rxi-kanx7-dqe",
            "3krjf-xhbli-mcsyh-dnxhx-2giht-5befp-k7uki-e3qd2-4pdqy-o4cyj-yae",
            "ybcs3-h5oeq-ocbau-swgud-htz3y-oi5ho-dwvya-vnhmj-r7qql-abjja-lqe",
            "jpnyo-vzs5e-7l3il-2cnaq-rgox5-cnaqh-4fu4b-stmme-iezgi-shla4-hae",
            "zbhfg-4mw2f-7nf5d-tp6o4-xyexr-w4vns-nlutd-tsvzc-atyty-6c2gi-4ae",
            "hrlet-rcmbs-qkqds-5vcuy-du255-3qcxb-t7kgl-l2vwf-y6abx-3j7dz-sqe",
        ];

        let users: Vec<_> = principals
            .iter()
            .map(|p| Principal::from_text(p).unwrap())
            .collect();

        let icp_ledger = pic
            .create_canister_with_id(Some(controller), None, Canister::IcpLedger.id())
            .await
            .unwrap();
        pic.add_cycles(icp_ledger, u64::MAX.into()).await;
        let mut initial_balances = HashMap::new();
        initial_balances.insert(
            AccountIdentifier::new(user.into(), None),
            Tokens::from_e8s(100_000 * E8S as u64),
        );
        for user in users.clone() {
            initial_balances.insert(
                AccountIdentifier::new(user.into(), None),
                Tokens::from_e8s(100_000 * E8S as u64),
            );
        }
        pic.install_canister(
            icp_ledger,
            Canister::IcpLedger.wasm(),
            Encode!(&LedgerCanisterInitPayload::builder()
                .initial_values(initial_balances.clone())
                .transfer_fee(Tokens::from_e8s(ICP_LEDGER_FEE.into_inner() as u64))
                .minting_account(controller.into())
                .token_symbol_and_name("ICP", "Internet Computer")
                .feature_flags(icp_ledger::FeatureFlags { icrc2: true })
                .build()
                .unwrap())
            .unwrap(),
            Some(controller),
        )
        .await;

        let evm_rpc = pic
            .create_canister_with_id(Some(controller), None, Canister::EvmRpc.id())
            .await
            .unwrap();
        pic.add_cycles(evm_rpc, u64::MAX.into()).await;
        pic.install_canister(
            evm_rpc,
            Canister::EvmRpc.wasm(),
            Encode!(&evm_rpc_types::InstallArgs {
                log_filter: Some(evm_rpc_types::LogFilter::HideAll),
                demo: None,
                manage_api_keys: None,
                override_provider: None,
                nodes_in_subnet: None
            })
            .unwrap(),
            Some(controller),
        )
        .await;

        let xrc = pic
            .create_canister_with_id(Some(controller), None, Canister::ExchangeRate.id())
            .await
            .unwrap();
        pic.add_cycles(xrc, u64::MAX.into()).await;
        pic.install_canister(xrc, Canister::ExchangeRate.wasm(), vec![], Some(controller))
            .await;

        let one_sec = pic
            .create_canister_with_id(Some(controller), None, Canister::OneSec.id())
            .await
            .unwrap();

        let usdc_ledger = pic
            .create_canister_with_id(Some(controller), None, Canister::Ledger.id())
            .await
            .unwrap();
        pic.add_cycles(usdc_ledger, u64::MAX.into()).await;
        let args = ledger::LedgerArg::Init(InitArgs {
            minting_account: Account {
                owner: one_sec,
                subaccount: None,
            },
            transfer_fee: Nat::from(USDC_LEDGER_FEE),
            decimals: Some(6),
            token_symbol: "USDC".to_string(),
            token_name: "USDC".to_string(),
            metadata: vec![],
            initial_balances: vec![],
            archive_options: ledger::ArchiveOptions {
                num_blocks_to_archive: 1000,
                trigger_threshold: 2000,
                controller_id: one_sec,
            },
        });
        pic.install_canister(
            usdc_ledger,
            Canister::Ledger.wasm(),
            Encode!(&args).unwrap(),
            Some(controller),
        )
        .await;

        pic.add_cycles(one_sec, u64::MAX.into()).await;
        let evm_arg = vec![InitEvmArg {
            chain: EvmChain::Base,
            initial_nonce: None,
            initial_block: None,
            ledger: vec![
                InitEvmTokenArg {
                    token: Token::ICP,
                    erc20_address: Some(evm.icp.to_string()),
                    logger_address: Some(evm.icp.to_string()),
                    initial_balance: None,
                },
                InitEvmTokenArg {
                    token: Token::USDC,
                    erc20_address: Some(evm.usdc.to_string()),
                    logger_address: Some(evm.usdc_locker.to_string()),
                    initial_balance: None,
                },
            ],
        }];
        pic.install_canister(
            one_sec,
            Canister::OneSec.wasm(),
            Encode!(&InitOrUpgradeArg::Init(InitArg {
                deployment: Deployment::Test,
                icp: None,
                evm: evm_arg,
            }))
            .unwrap(),
            Some(controller),
        )
        .await;

        let test = TestEnv {
            pic: Arc::new(Mutex::new(pic)),
            controller,
            user,
            users,
            icp_ledger,
            usdc_ledger,
            one_sec,
            evm_rpc,
            xrc,
            evm,
        };

        while test.get_evm_address().await.is_none() {
            test.tick().await;
        }

        let canister_evm_address =
            Address::from_hex(test.get_evm_address().await.unwrap()).unwrap();

        test.evm.token_update_owner(canister_evm_address).await;

        test.evm.locker_update_owner(canister_evm_address).await;

        test.evm
            .usdc_mint(test.evm.user, Wei::new(1_000_000_000))
            .await;

        test.evm.transfer_eth(canister_evm_address, "1").await;

        let pic = Arc::downgrade(&test.pic);
        let config = config::Config::local();
        let rpc_nodes = config
            .evm
            .iter()
            .flat_map(|x| get_nodes(&x.evm_rpc.rpc_services).1)
            .collect();
        task::spawn(handle_http_outcalls(
            pic,
            test.evm.anvil_url.clone(),
            rpc_nodes,
        ));
        test
    }

    pub async fn check_reproducibility(&self) {
        self.advance_time(Duration::from_secs(3600)).await;
        for _ in 0..20 {
            self.tick().await;
        }

        let old_metadata = self.get_metadata(self.controller).await;
        let old_transfers = self.get_transfers(self.controller, 1000).await;

        {
            let pic = self.pic.lock().await;
            pic.upgrade_canister(
                self.one_sec,
                Canister::OneSec.wasm(),
                Encode!(&InitOrUpgradeArg::Upgrade(UpgradeArg {
                    deployment: Deployment::Test,
                }))
                .unwrap(),
                Some(self.controller),
            )
            .await
            .unwrap();
        }

        let new_metadata = self.get_metadata(self.controller).await;
        let new_transfers = self.get_transfers(self.controller, 1000).await;

        assert_eq!(old_metadata.event_count + 1, new_metadata.event_count);

        assert_eq!(old_metadata.tokens.len(), new_metadata.tokens.len());
        for i in 0..old_metadata.tokens.len() {
            assert_eq!(
                old_metadata.tokens[i].balance,
                new_metadata.tokens[i].balance
            );
        }

        assert_eq!(old_metadata.evm_chains.len(), new_metadata.evm_chains.len());
        for i in 0..old_metadata.evm_chains.len() {
            assert_eq!(
                old_metadata.evm_chains[i].nonce,
                new_metadata.evm_chains[i].nonce
            );
        }

        assert_eq!(old_transfers.len(), new_transfers.len());
        for i in 0..old_transfers.len() {
            assert_eq!(old_transfers[i].status, new_transfers[i].status);
            assert_eq!(old_transfers[i].source, new_transfers[i].source);
            assert_eq!(old_transfers[i].destination, new_transfers[i].destination);
            assert_eq!(old_transfers[i].start, new_transfers[i].start);
            assert_eq!(old_transfers[i].end, new_transfers[i].end);
        }
    }

    pub async fn icp_approve(&self, user: Principal, spender: Principal, amount: Amount) {
        self.update::<Result<Nat, ApproveError>>(
            self.icp_ledger,
            user,
            "icrc2_approve",
            ApproveArgs {
                from_subaccount: None,
                spender: Account {
                    owner: spender,
                    subaccount: None,
                },
                amount: amount.into(),
                expected_allowance: None,
                expires_at: None,
                fee: None,
                memo: None,
                created_at_time: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    }

    pub async fn usdc_approve(&self, user: Principal, spender: Principal, amount: Wei) {
        self.update::<Result<Nat, ApproveError>>(
            self.usdc_ledger,
            user,
            "icrc2_approve",
            ApproveArgs {
                from_subaccount: None,
                spender: Account {
                    owner: spender,
                    subaccount: None,
                },
                amount: amount.into(),
                expected_allowance: None,
                expires_at: None,
                fee: None,
                memo: None,
                created_at_time: None,
            },
        )
        .await
        .unwrap()
        .unwrap();
    }

    pub async fn _icp_transfer(
        &self,
        from: Principal,
        to: Principal,
        amount: Amount,
    ) -> BlockIndex {
        let result = self
            .update::<Result<Nat, TransferError>>(
                self.icp_ledger,
                from,
                "icrc1_transfer",
                IcrcTransferArg {
                    from_subaccount: None,
                    to: Account {
                        owner: to,
                        subaccount: None,
                    },
                    fee: None,
                    created_at_time: None,
                    memo: None,
                    amount: Nat::from(amount),
                },
            )
            .await
            .unwrap()
            .unwrap();
        let height: u64 = result.0.try_into().unwrap();
        BlockIndex { height }
    }

    pub async fn icp_balance_of(&self, user: Principal) -> Amount {
        let result = self
            .query::<Nat>(
                self.icp_ledger,
                user,
                "icrc1_balance_of",
                Account {
                    owner: user,
                    subaccount: None,
                },
            )
            .await
            .unwrap();
        result.try_into().unwrap()
    }

    pub async fn usdc_balance_of(&self, user: Principal) -> Wei {
        let result = self
            .query::<Nat>(
                self.usdc_ledger,
                user,
                "icrc1_balance_of",
                Account {
                    owner: user,
                    subaccount: None,
                },
            )
            .await
            .unwrap();
        result.try_into().unwrap()
    }

    pub async fn usdc_balance_with_subaccount(&self, user: Principal, subaccount: [u8; 32]) -> Wei {
        let result = self
            .query::<Nat>(
                self.usdc_ledger,
                user,
                "icrc1_balance_of",
                Account {
                    owner: user,
                    subaccount: Some(subaccount),
                },
            )
            .await
            .unwrap();
        result.try_into().unwrap()
    }

    pub async fn transfer(&self, user: Principal, arg: TransferArg) -> TransferResponse {
        self.update(self.one_sec, user, "transfer", arg)
            .await
            .unwrap()
    }

    pub async fn get_transfer(&self, user: Principal, id: TransferId) -> Transfer {
        self.query::<Result<Transfer, String>>(self.one_sec, user, "get_transfer", id)
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn get_transfers(&self, user: Principal, count: u64) -> Vec<Transfer> {
        self.query::<Result<Vec<Transfer>, String>>(
            self.one_sec,
            user,
            "get_transfers",
            GetTransfersArg {
                accounts: vec![],
                skip: 0,
                count,
            },
        )
        .await
        .unwrap()
        .unwrap()
    }

    pub async fn get_forwarding_address(&self, user: Principal, account: IcpAccount) -> String {
        self.query::<Result<String, String>>(
            self.one_sec,
            user,
            "get_forwarding_address",
            account,
        )
        .await
        .unwrap()
        .unwrap()
    }

    pub async fn try_get_forwarding_address(
        &self,
        user: Principal,
        account: IcpAccount,
    ) -> Result<String, String> {
        self.query::<Result<String, String>>(
            self.one_sec,
            user,
            "get_forwarding_address",
            account,
        )
        .await
        .unwrap()
    }

    pub async fn get_forwarding_transactions(
        &self,
        user: Principal,
        chain: EvmChain,
    ) -> Vec<SignedForwardingTx> {
        self.query::<Vec<SignedForwardingTx>>(
            self.one_sec,
            user,
            "get_forwarding_transactions",
            chain,
        )
        .await
        .unwrap()
    }

    pub async fn forward_evm_to_icp(
        &self,
        user: Principal,
        arg: ForwardEvmToIcpArg,
    ) -> ForwardingResponse {
        self.update::<Result<ForwardingResponse, String>>(
            self.one_sec,
            user,
            "forward_evm_to_icp",
            arg,
        )
        .await
        .unwrap()
        .unwrap()
    }

    pub async fn submit_forwarding_update(&self, user: Principal, arg: ForwardingUpdate) -> () {
        self.update::<Result<(), String>>(self.one_sec, user, "submit_forwarding_update", arg)
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn _get_events(&self, user: Principal) -> Vec<String> {
        self.query::<Vec<String>>(self.one_sec, user, "get_events", ())
            .await
            .unwrap()
    }

    pub async fn get_wei_per_icp_rate(&self) -> f64 {
        self.query(self.one_sec, self.user, "get_wei_per_icp_rate", ())
            .await
            .unwrap()
    }

    pub async fn get_evm_address(&self) -> Option<String> {
        self.query::<Option<String>>(self.one_sec, self.user, "get_evm_address", ())
            .await
            .unwrap()
    }

    pub async fn get_canister_calls(&self, user: Principal) -> Vec<CanisterCalls> {
        self.query(self.one_sec, user, "get_canister_calls", ())
            .await
            .unwrap()
    }

    pub async fn get_metadata(&self, user: Principal) -> Metadata {
        self.query::<Result<Metadata, String>>(self.one_sec, user, "get_metadata", ())
            .await
            .unwrap()
            .unwrap()
    }

    pub async fn pause_task(&self, user: Principal, task: TaskType) -> String {
        self.update(self.one_sec, user, "pause_task", task)
            .await
            .unwrap()
    }

    pub async fn resume_task(&self, user: Principal, task: TaskType) -> String {
        self.update(self.one_sec, user, "resume_task", task)
            .await
            .unwrap()
    }

    pub async fn run_task(&self, user: Principal, task: TaskType) -> String {
        self.update(self.one_sec, user, "run_task", task)
            .await
            .unwrap()
    }

    pub async fn resume_all_paused_tasks(&self, user: Principal) -> String {
        self.update(self.one_sec, user, "resume_all_paused_tasks", ())
            .await
            .unwrap()
    }

    pub async fn pause_endpoint(&self, user: Principal, endpoint: Endpoint) -> String {
        self.update(self.one_sec, user, "pause_endpoint", endpoint)
            .await
            .unwrap()
    }

    pub async fn resume_endpoint(&self, user: Principal, endpoint: Endpoint) -> String {
        self.update(self.one_sec, user, "resume_endpoint", endpoint)
            .await
            .unwrap()
    }

    pub async fn resume_all_paused_endpoints(&self, user: Principal) -> String {
        self.update(self.one_sec, user, "resume_all_paused_endpoints", ())
            .await
            .unwrap()
    }

    pub async fn tick(&self) {
        let pic = self.pic.lock().await;
        pic.advance_time(Duration::from_secs(1)).await;
        pic.tick().await;
    }

    pub async fn advance_time(&self, duration: Duration) {
        let pic = self.pic.lock().await;
        pic.advance_time(duration).await;
    }

    async fn update<T>(
        &self,
        canister: CanisterId,
        caller: Principal,
        method: &str,
        arg: impl CandidType,
    ) -> Result<T, String>
    where
        T: for<'a> Deserialize<'a> + CandidType,
    {
        let pic = self.pic.lock().await;
        update(&pic, canister, caller, method, arg).await
    }

    async fn query<T>(
        &self,
        canister: CanisterId,
        caller: Principal,
        method: &str,
        arg: impl CandidType,
    ) -> Result<T, String>
    where
        T: for<'a> Deserialize<'a> + CandidType,
    {
        let pic = self.pic.lock().await;
        query(&pic, canister, caller, method, arg).await
    }
}

fn get_nodes(providers: &RpcServices) -> (usize, Vec<String>) {
    match &providers {
        RpcServices::Custom {
            chain_id: _,
            services,
        } => (
            services.len(),
            services.iter().map(|x| x.url.clone()).collect(),
        ),
        RpcServices::EthMainnet(eth_mainnet_services) => (
            eth_mainnet_services
                .as_ref()
                .map(|x| x.len())
                .unwrap_or_default(),
            vec![],
        ),
        RpcServices::EthSepolia(eth_sepolia_services) => (
            eth_sepolia_services
                .as_ref()
                .map(|x| x.len())
                .unwrap_or_default(),
            vec![],
        ),
        RpcServices::ArbitrumOne(l2_mainnet_services) => (
            l2_mainnet_services
                .as_ref()
                .map(|x| x.len())
                .unwrap_or_default(),
            vec![],
        ),
        RpcServices::BaseMainnet(l2_mainnet_services) => (
            l2_mainnet_services
                .as_ref()
                .map(|x| x.len())
                .unwrap_or_default(),
            vec![],
        ),
        RpcServices::OptimismMainnet(l2_mainnet_services) => (
            l2_mainnet_services
                .as_ref()
                .map(|x| x.len())
                .unwrap_or_default(),
            vec![],
        ),
    }
}
