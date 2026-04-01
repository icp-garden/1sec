use alloy::{
    eips::BlockNumberOrTag,
    hex::FromHex,
    primitives::{Address, FixedBytes, B256},
    providers::{Provider, ProviderBuilder},
    rpc::types::Filter,
    signers::{k256::ecdsa::SigningKey, local::PrivateKeySigner},
};
use alloy_consensus::TxEnvelope;
use alloy_rlp::Decodable;
use async_trait::async_trait;
use candid::{decode_one, Encode, Principal};
use eyre::{bail, eyre, OptionExt};
use futures::{stream::FuturesUnordered, StreamExt};
use ic_agent::{agent::CallResponse, export::reqwest::Url, Agent, Identity, RequestId};
use merkle::check_proof;
use one_sec::{
    api::types::{EvmChain, Metadata, RelayProof, RelayTask},
    evm::TxHash,
};
use openssl::ec::EcKey;
use openssl::pkey::Private;
use std::{
    collections::{hash_map, BTreeMap, BTreeSet, HashMap, VecDeque},
    time::{Duration, Instant},
};
use std::{fs, sync::Arc};

mod arbitrum;
mod ethereum;
mod forwarder;
mod merkle;
mod optimism;

type TxIndex = usize;

struct TaskMetadata {
    id: u64,
}

#[async_trait]
trait Worker: Send {
    async fn send_tx(&self, raw_tx: &[u8]) -> Result<(), eyre::Error>;

    async fn check_status(
        &self,
        tx_hash: TxHash,
    ) -> Result<Option<(BlockNumberOrTag, TxIndex)>, eyre::Error>;

    async fn build_tx_proof(
        &self,
        block_number: BlockNumberOrTag,
        tx_index: usize,
        task_metadata: &TaskMetadata,
    ) -> Result<Vec<RelayProof>, eyre::Error>;

    async fn build_block_proof(
        &self,
        block_number: BlockNumberOrTag,
    ) -> Result<(u64, RelayProof), eyre::Error>;

    async fn self_check(&self) -> Result<(), eyre::Error>;
}

#[tokio::main]
async fn main() -> Result<(), eyre::Error> {
    std::panic::set_hook(Box::new(|info| {
        eprintln!("Thread panicked: {}", info);
        std::process::exit(1);
    }));

    let matches = clap::Command::new("relayer")
        .version("0.1")
        .about("Relay transactions from ICP to EVM and submit proofs back to ICP")
        .arg(
            clap::Arg::new("canister_id")
                .long("canister-id")
                .help("The id of the backend canister")
                .default_value("5okwm-giaaa-aaaar-qbn6a-cai"),
        )
        .arg(
            clap::Arg::new("icp_url")
                .long("icp-url")
                .help("The URL of an ICP gateway.")
                .default_value("https://ic0.app"),
        )
        .arg(
            clap::Arg::new("base_url")
                .long("base-url")
                .help("The URL of an Base RPC node.")
                .default_value("https://base-rpc.publicnode.com"),
        )
        .arg(
            clap::Arg::new("arbitrum_url")
                .long("arbitrum-url")
                .help("The URL of an Arbitrum RPC node.")
                .default_value("https://arbitrum-one-rpc.publicnode.com"),
        )
        .arg(
            clap::Arg::new("ethereum_url")
                .long("ethereum-url")
                .help("The URL of an Ethereum RPC node.")
                .default_value("https://ethereum-rpc.publicnode.com"),
        )
        .arg(
            clap::Arg::new("identity")
                .long("identity")
                .required(true)
                .help(
                    r"Path to PEM file with encrypted EC key.
You can generate a new key using openssl:
`openssl ecparam -name secp256k1 -genkey | openssl ec -aes256 -out encrypted_ec_key.pem`
                ",
                ),
        )
        .arg(
            clap::Arg::new("forward-chains")
                .long("forward-chains")
                .help("A comma or space separated list of chains where to run a forwarder: all, arbitrum, base, ethereum, none")
                .default_value("none"),
        )
        .get_matches();

    let canister_id = matches
        .get_one::<String>("canister_id")
        .ok_or_eyre("cannot parse canister-id")?
        .clone();
    let canister_id = Principal::from_text(canister_id)?;

    let icp_url = matches
        .get_one::<String>("icp_url")
        .ok_or_eyre("cannot parse icp-url")?
        .clone();

    let base_url = matches
        .get_one::<String>("base_url")
        .ok_or_eyre("cannot parse base-url")?
        .clone();
    let base_url: Url = base_url.parse()?;

    let arbitrum_url = matches
        .get_one::<String>("arbitrum_url")
        .ok_or_eyre("cannot parse arbitrum-url")?
        .clone();
    let arbitrum_url: Url = arbitrum_url.parse()?;

    let ethereum_url = matches
        .get_one::<String>("ethereum_url")
        .ok_or_eyre("cannot parse ethereum-url")?
        .clone();
    let ethereum_url: Url = ethereum_url.parse()?;

    let forward_chains = matches
        .get_one::<String>("forward-chains")
        .ok_or_eyre("cannot parse forward-chains")?
        .clone();

    let forward_chains: Vec<String> = forward_chains
        .replace(',', " ")
        .split_whitespace()
        .map(|x| x.to_lowercase())
        .collect();

    for x in forward_chains.iter() {
        let x = x.to_lowercase();
        if x == "all" {
            if forward_chains.len() != 1 {
                bail!("--forward-chains=all cannot be combined with other chains");
            }
        } else if x == "none" {
            if forward_chains.len() != 1 {
                bail!("--forward-chains=none cannot be combined with other chains");
            }
        } else if x != "arbitrum" && x != "base" && x != "ethereum" {
            bail!("invalid value of --forward-chains: {}", x);
        }
    }

    let pem_file = matches
        .get_one::<String>("identity")
        .ok_or_eyre("cannot parse identity")?
        .clone();

    let pem_data = fs::read(pem_file.clone())
        .map_err(|err| eyre!("couldn't load the PEM file {}: {}", pem_file, err))?;

    let mut pwd: Result<String, eyre::Error> =
        std::env::var("RELAYER_SECRET").map_err(|err| err.into());

    if pwd.is_err() {
        println!("couldn't find $RELAYER_SECRET, please enter it manually:");
        pwd = rpassword::read_password().map_err(|err| err.into());
    }

    let ec_key: EcKey<Private> = EcKey::private_key_from_pem_passphrase(&pem_data, pwd?.as_bytes())
        .map_err(|_| eyre!("failed to decrypt PEM file with the given secret"))?;

    let ec_pem = ec_key
        .private_key_to_pem()
        .map_err(|_| eyre!("failed to serialize EC key"))?;

    let raw_key = ec_key.private_key().to_vec();

    let signer = PrivateKeySigner::from_signing_key(SigningKey::from_slice(&raw_key)?);

    let identity: Arc<dyn Identity> = Arc::new(
        ic_agent::identity::Secp256k1Identity::from_pem(std::io::Cursor::new(ec_pem))
            .map_err(|_| eyre!("failed to create an identity from EC pem file"))?,
    );

    println!(
        "Relayer ICP principal: {}",
        identity.as_ref().sender().unwrap().to_text()
    );

    println!("Relayer EVM address: {}", signer.address());

    let mut join = vec![
        tokio::spawn(run(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Base,
            base_url.clone(),
        )),
        tokio::spawn(run(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Ethereum,
            ethereum_url.clone(),
        )),
        tokio::spawn(run(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Arbitrum,
            arbitrum_url.clone(),
        )),
        tokio::spawn(fetch_tx_logs(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Base,
            base_url.clone(),
        )),
        tokio::spawn(fetch_tx_logs(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Ethereum,
            ethereum_url.clone(),
        )),
        tokio::spawn(fetch_tx_logs(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Arbitrum,
            arbitrum_url.clone(),
        )),
        tokio::spawn(fetch_fee(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Base,
            base_url.clone(),
        )),
        tokio::spawn(fetch_fee(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Ethereum,
            ethereum_url.clone(),
        )),
        tokio::spawn(fetch_fee(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Arbitrum,
            arbitrum_url.clone(),
        )),
    ];

    if forward_chains.iter().any(|x| x == "all" || x == "base") {
        println!("Forwarding on Base");
        join.push(tokio::spawn(forwarder::forward(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Base,
            base_url,
            signer.clone(),
        )));
    }

    if forward_chains.iter().any(|x| x == "all" || x == "arbitrum") {
        println!("Forwarding on Arbitrum");
        join.push(tokio::spawn(forwarder::forward(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Arbitrum,
            arbitrum_url,
            signer.clone(),
        )));
    }

    if forward_chains.iter().any(|x| x == "all" || x == "ethereum") {
        println!("Forwarding on Ethereum");
        join.push(tokio::spawn(forwarder::forward(
            Arc::clone(&identity),
            canister_id,
            icp_url.clone(),
            EvmChain::Ethereum,
            ethereum_url,
            signer.clone(),
        )));
    }

    let mut join = FuturesUnordered::from_iter(join);
    while let Some(result) = join.next().await {
        result??;
    }

    Ok(())
}

async fn fetch_tx_logs(
    identity: Arc<dyn ic_agent::Identity>,
    canister_id: Principal,
    icp_url: String,
    chain: EvmChain,
    rpc_url: Url,
) -> Result<(), eyre::Error> {
    let agent = Agent::builder()
        .with_arc_identity(identity)
        .with_url(&icp_url)
        .build()?;

    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let provider = ProviderBuilder::new()
        .disable_recommended_fillers()
        .network::<alloy::network::Ethereum>()
        .on_http(rpc_url);

    let tokens: Vec<_> = loop {
        match agent
            .query(&canister_id, "get_metadata")
            .with_arg(Encode!(&())?)
            .await
        {
            Ok(result) => {
                let metadata: Result<Metadata, String> = decode_one(&result).unwrap();
                break metadata
                    .unwrap()
                    .tokens
                    .into_iter()
                    .filter(|c| c.chain == Some(chain.into()))
                    .collect();
            }
            Err(err) => {
                println!("Error in get_relay_tasks: {}", err);
                tokio::time::sleep(Duration::from_millis(5000)).await;
                continue;
            }
        };
    };

    let contracts: Vec<_> = tokens
        .iter()
        .cloned()
        .map(|t| t.locker.unwrap_or(t.contract))
        .map(|a| Address::from_hex(a).unwrap())
        .collect();
    let topics: BTreeSet<Vec<_>> = tokens.into_iter().flat_map(|t| t.topics).collect();

    let topics: Vec<B256> = topics.into_iter().map(|v| B256::from_slice(&v)).collect();

    let mut previous = None;
    loop {
        let latest = match provider.get_block_number().await {
            Ok(block) => block,
            Err(err) => {
                println!("Error in get_block_number: {}", err);
                tokio::time::sleep(Duration::from_millis(5000)).await;
                continue;
            }
        };
        let filter = Filter::new()
            .address(contracts.clone())
            .event_signature(topics.clone())
            .from_block(BlockNumberOrTag::Number(
                previous.map(|x| x + 1).unwrap_or(latest),
            ))
            .to_block(BlockNumberOrTag::Number(latest));

        match provider.get_logs(&filter).await {
            Ok(logs) => {
                let blocks: BTreeSet<_> = logs.into_iter().filter_map(|x| x.block_number).collect();
                let proofs: Vec<_> = blocks
                    .iter()
                    .map(|b| RelayProof::EvmBlockWithTxLogs { block_number: *b })
                    .collect();
                if proofs.is_empty() {
                    previous = Some(latest);
                } else {
                    match submit_proof(&agent, canister_id, chain, proofs).await {
                        Ok(call) => match await_call(&agent, call).await {
                            Ok(_) => {
                                previous = Some(latest);
                                for b in blocks {
                                    println!("{:?}: submitted block with tx logs: {}", chain, b);
                                }
                            }
                            Err(err) => {
                                println!("Error when submitting blocks with log events: {}", err);
                                tokio::time::sleep(Duration::from_millis(5000)).await;
                            }
                        },
                        Err(err) => {
                            println!("Error when submitting blocks with log events: {}", err);
                            tokio::time::sleep(Duration::from_millis(5000)).await;
                        }
                    }
                }
            }
            Err(err) => {
                println!("Error in get_logs: {}", err);
                tokio::time::sleep(Duration::from_millis(5000)).await;
                continue;
            }
        }
        tokio::time::sleep(Duration::from_millis(30000)).await;
    }
}

async fn fetch_fee(
    identity: Arc<dyn ic_agent::Identity>,
    canister_id: Principal,
    icp_url: String,
    chain: EvmChain,
    rpc_url: Url,
) -> Result<(), eyre::Error> {
    let agent = Agent::builder()
        .with_arc_identity(identity)
        .with_url(&icp_url)
        .build()?;

    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let worker = match chain {
        EvmChain::Base => Box::new(optimism::OpWorker::new(rpc_url)) as Box<dyn Worker>,
        EvmChain::Arbitrum => Box::new(arbitrum::ArbitrumWorker::new(rpc_url)) as Box<dyn Worker>,
        EvmChain::Ethereum => Box::new(ethereum::EthWorker::new(rpc_url)) as Box<dyn Worker>,
    };

    let mut fee_per_gas = 0;
    let mut priority_fee_per_gas = 0;
    let mut last_submit_time = Instant::now();

    loop {
        if let Ok((block_number, proof)) = worker.build_block_proof(BlockNumberOrTag::Latest).await
        {
            let mut new_fee_per_gas = fee_per_gas;
            let mut new_priority_fee_per_gas = priority_fee_per_gas;
            match &proof {
                RelayProof::EvmBlockHeader {
                    hint_fee_per_gas,
                    hint_priority_fee_per_gas,
                    ..
                } => {
                    if let Some(x) = hint_fee_per_gas {
                        new_fee_per_gas = *x;
                    }
                    if let Some(x) = hint_priority_fee_per_gas {
                        new_priority_fee_per_gas = *x;
                    }
                }
                RelayProof::EvmBlockWithTxLogs { .. }
                | RelayProof::EvmTransactionReceipt { .. } => {
                    unreachable!("expected a block proof");
                }
            }

            if last_submit_time.elapsed() > Duration::from_secs(60)
                || new_fee_per_gas > fee_per_gas + fee_per_gas / 20
                || new_priority_fee_per_gas > priority_fee_per_gas + priority_fee_per_gas / 20
            {
                match submit_proof(&agent, canister_id, chain, vec![proof]).await {
                    Ok(call) => match await_call(&agent, call).await {
                        Ok(_) => {
                            last_submit_time = Instant::now();
                            fee_per_gas = new_fee_per_gas;
                            priority_fee_per_gas = new_priority_fee_per_gas;
                            println!(
                                "{} {:?}: block={}, fee_per_gas={}, priority_fee_per_gas={}",
                                chrono::Local::now().format("%m-%d %H:%M:%S"),
                                chain,
                                block_number,
                                fee_per_gas,
                                priority_fee_per_gas
                            );
                        }
                        Err(err) => {
                            println!("Error when submitting fee: {}", err);
                            tokio::time::sleep(Duration::from_millis(5000)).await;
                        }
                    },
                    Err(err) => {
                        println!("Error when submitting fee: {}", err);
                        tokio::time::sleep(Duration::from_millis(5000)).await;
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(30000)).await;
    }
}

async fn run(
    identity: Arc<dyn ic_agent::Identity>,
    canister_id: Principal,
    icp_url: String,
    chain: EvmChain,
    rpc_url: Url,
) -> Result<(), eyre::Error> {
    let agent = Agent::builder()
        .with_arc_identity(identity)
        .with_url(&icp_url)
        .build()?;

    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let worker = match chain {
        EvmChain::Base => Box::new(optimism::OpWorker::new(rpc_url)) as Box<dyn Worker>,
        EvmChain::Arbitrum => Box::new(arbitrum::ArbitrumWorker::new(rpc_url)) as Box<dyn Worker>,
        EvmChain::Ethereum => Box::new(ethereum::EthWorker::new(rpc_url)) as Box<dyn Worker>,
    };

    if !icp_url.contains("localhost") && !icp_url.contains("127.0.0.1") {
        worker.self_check().await?;
    }

    let mut sent: HashMap<TxHash, Instant> = HashMap::new();
    let mut proven: HashMap<TxHash, Instant> = HashMap::new();
    let mut retry_after: HashMap<TxHash, Instant> = HashMap::new();
    let mut block_proofs: BTreeMap<u64, RelayProof> = BTreeMap::new();
    let mut done_blocks: BTreeSet<u64> = BTreeSet::new();

    let mut pending_calls: VecDeque<_> = Default::default();

    let mut latest = 0;

    loop {
        let tasks = match agent
            .query(&canister_id, "get_relay_tasks")
            .with_arg(Encode!(&(chain))?)
            .await
        {
            Ok(tasks) => tasks,
            Err(err) => {
                println!("Error in get_relay_tasks: {}", err);
                tokio::time::sleep(Duration::from_millis(5000)).await;
                continue;
            }
        };

        let tasks: Vec<RelayTask> = decode_one(&tasks)?;

        if !tasks.is_empty() {
            println!(
                "{} {:?} tasks: {}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                tasks.len()
            );
        }

        let mut proofs = vec![];
        let mut pending = vec![];

        let blocks: Vec<_> = tasks
            .iter()
            .filter_map(|task| match task {
                RelayTask::SendEvmTransaction { .. } => None,
                RelayTask::FetchEvmBlock { block_number } => Some(*block_number),
            })
            .filter(|b| !done_blocks.contains(b))
            .collect();

        let any_tasks = !tasks.is_empty();

        for task in tasks {
            match task {
                RelayTask::FetchEvmBlock { .. } => {}
                RelayTask::SendEvmTransaction { id, tx } => {
                    let task_metadata = TaskMetadata { id };

                    let tx_hash = TxHash(decode_tx_hash(&tx.bytes)?.0);

                    if proven.contains_key(&tx_hash) {
                        continue;
                    }

                    if let Some(deadline) = retry_after.get(&tx_hash) {
                        if deadline < &std::time::Instant::now() {
                            continue;
                        }
                    }

                    if let hash_map::Entry::Vacant(e) = sent.entry(tx_hash) {
                        match worker.send_tx(&tx.bytes).await {
                            Ok(()) => {
                                println!("sent tx: {}", tx_hash);
                                e.insert(Instant::now());
                            }
                            Err(err) => {
                                let msg = err.to_string();
                                println!("failed to send {}: {}", tx_hash, msg);
                                if msg.contains("max fee per gas less than block base fee") {
                                    retry_after
                                        .insert(tx_hash, Instant::now() + Duration::from_secs(10));
                                } else if msg.contains("nonce too low")
                                    || msg.contains("already known")
                                    || msg.contains("replacement transaction underpriced")
                                {
                                    e.insert(Instant::now());
                                }
                                // Fall through without `continue` in order to check the status.
                            }
                        }
                    }

                    let (block, tx_index) = match worker.check_status(tx_hash).await {
                        Ok(Some((block, tx_index))) => (block, tx_index),
                        Ok(None) => {
                            continue;
                        }
                        Err(err) => {
                            println!("receipt error for {}: {}", tx_hash, err);
                            continue;
                        }
                    };

                    let proof = match worker.build_tx_proof(block, tx_index, &task_metadata).await {
                        Ok(proof) => {
                            println!(
                                "{} {:?} proved: {}",
                                chrono::Local::now().format("%m-%d %H:%M:%S"),
                                chain,
                                tx_hash,
                            );
                            proven.insert(tx_hash, Instant::now());
                            proof
                        }
                        Err(err) => {
                            println!(
                                "{} {:?} failed to prove: {}: {}",
                                chrono::Local::now().format("%m-%d %H:%M:%S"),
                                chain,
                                tx_hash,
                                err,
                            );
                            continue;
                        }
                    };

                    for p in proof.iter().cloned() {
                        assert_eq!(Ok(()), check_proof(p, true))
                    }

                    proofs.extend(proof);
                }
            }
        }

        if any_tasks {
            if let Ok((block_number, proof)) =
                worker.build_block_proof(BlockNumberOrTag::Latest).await
            {
                latest = block_number;
                if let std::collections::btree_map::Entry::Vacant(e) =
                    block_proofs.entry(block_number)
                {
                    println!(
                        "{} {:?} fetched block: {}",
                        chrono::Local::now().format("%m-%d %H:%M:%S"),
                        chain,
                        block_number
                    );
                    e.insert(proof.clone());
                    proofs.push(proof);
                }
            }
        }

        for block in blocks.iter() {
            if !block_proofs.contains_key(block) && *block <= latest {
                match worker
                    .build_block_proof(BlockNumberOrTag::Number(*block))
                    .await
                {
                    Ok((_block_number, proof)) => {
                        block_proofs.insert(*block, proof);
                    }
                    Err(err) => {
                        let msg = err.to_string();
                        if !msg.contains("Block not found") {
                            println!("failed to fetch block {}: {}", block, msg);
                        }
                        break;
                    }
                }
            }
            if let Some(proof) = block_proofs.get(block) {
                proofs.push(proof.clone());
                pending.push(*block);
            }
        }

        if !proofs.is_empty() {
            let n = proofs.len();

            println!(
                "{} {:?} submitting proofs: {}",
                chrono::Local::now().format("%m-%d %H:%M:%S"),
                chain,
                n,
            );
            match submit_proof(&agent, canister_id, chain, proofs.clone()).await {
                Ok(call) => {
                    for block in pending.iter() {
                        done_blocks.insert(*block);
                    }
                    pending_calls.push_back((call, pending));
                }
                Err(err) => {
                    println!(
                        "{} {:?} failed to submit proofs: {}: {}",
                        chrono::Local::now().format("%m-%d %H:%M:%S"),
                        chain,
                        n,
                        err,
                    );
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(5000)).await;
        let too_old = Instant::now() - Duration::from_secs(3600);
        sent.retain(|_h, t| *t >= too_old);
        proven.retain(|_h, t| *t >= too_old);
        retry_after.retain(|_h, t| *t >= too_old);
        while block_proofs.len() > 1_000 {
            block_proofs.pop_first();
        }
        while done_blocks.len() > 1_000 {
            done_blocks.pop_first();
        }
        if pending_calls.len() > 10 {
            if let Some((call, pending)) = pending_calls.pop_front() {
                let start = Instant::now();
                match await_call(&agent, call).await {
                    Ok(_) => {
                        println!(
                            "{} {:?} awaited call: {}s",
                            chrono::Local::now().format("%m-%d %H:%M:%S"),
                            chain,
                            start.elapsed().as_secs(),
                        );
                    }
                    Err(err) => {
                        for block in pending {
                            done_blocks.remove(&block);
                        }
                        println!(
                            "{} {:?} failed to await call: {}s:  {}",
                            chrono::Local::now().format("%m-%d %H:%M:%S"),
                            chain,
                            start.elapsed().as_secs(),
                            err,
                        );
                    }
                }
            }
        }
    }
}

enum PendingCall {
    Ready(Result<(), eyre::Error>),
    Pending(RequestId, Principal),
}

async fn submit_proof(
    agent: &Agent,
    canister_id: Principal,
    chain: EvmChain,
    proof: Vec<RelayProof>,
) -> Result<PendingCall, eyre::Error> {
    let result = agent
        .update(&canister_id, "submit_relay_proof")
        .with_arg(Encode!(&chain, &proof)?)
        .call()
        .await?;
    match result {
        CallResponse::Response(response) => {
            let result: std::result::Result<(), String> = decode_one(&response.0)?;
            Ok(PendingCall::Ready(result.map_err(|e| eyre!(e))))
        }
        CallResponse::Poll(request_id) => Ok(PendingCall::Pending(request_id, canister_id)),
    }
}

async fn await_call(agent: &Agent, call: PendingCall) -> Result<(), eyre::Error> {
    match call {
        PendingCall::Ready(result) => result,
        PendingCall::Pending(request_id, principal) => {
            let result = agent.wait(&request_id, principal).await?;
            let result: std::result::Result<(), String> = decode_one(&result.0)?;
            result.map_err(|e| eyre!(e))
        }
    }
}

fn decode_tx_hash(mut buf: &[u8]) -> Result<FixedBytes<32>, eyre::Error> {
    let decode_tx = TxEnvelope::decode(&mut buf)?;
    Ok(*decode_tx.tx_hash())
}
