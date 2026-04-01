use candid::{decode_one, Encode, Principal};
use chrono::{DateTime, TimeZone, Utc};
use eyre::{bail, eyre, OptionExt};
use ic_agent::Agent;
use one_sec::event::RootEvent;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{self, BufReader};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

#[tokio::main]
async fn main() -> Result<(), eyre::Error> {
    let matches = clap::Command::new("backup")
        .version("0.1")
        .about("Backup events and logs")
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
            clap::Arg::new("event_file")
                .long("event-file")
                .help("Path to the output event file")
                .default_value("events.data"),
        )
        .arg(
            clap::Arg::new("identity")
                .long("identity")
                .help("Path to the PEM file of the identity that controls the canister")
                .default_value(""),
        )
        .arg(
            clap::Arg::new("restore")
                .long("restore")
                .action(clap::ArgAction::SetTrue),
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

    let mut event_file = PathBuf::new();
    event_file.push(
        matches
            .get_one::<String>("event_file")
            .ok_or_eyre("cannot parse event-file")?,
    );

    let restore = matches.get_flag("restore");

    if restore {
        let mut identity_file = PathBuf::new();
        identity_file.push(
            matches
                .get_one::<String>("identity")
                .ok_or_eyre("cannot parse identity")?
                .clone(),
        );

        println!("Reading events from: {:?}", event_file);
        println!("Using identity: {:?}", identity_file);

        if icp_url == "https://ic0.app" {
            return Err(eyre!(
                "Refusing to restore the mainnet canister for safety."
            ));
        }
        restore_events(canister_id, icp_url.clone(), event_file, identity_file).await?;
    } else {
        println!("Writing events to: {:?}", event_file);
        fetch_events(canister_id, icp_url.clone(), event_file).await?;
    }

    Ok(())
}

async fn restore_events(
    canister_id: Principal,
    icp_url: String,
    event_file: PathBuf,
    identity_file: PathBuf,
) -> Result<(), eyre::Error> {
    if !event_file.exists() {
        return Err(eyre!("{} is missing", event_file.to_string_lossy()));
    }

    let agent = Agent::builder()
        .with_identity(ic_agent::identity::Secp256k1Identity::from_pem_file(
            identity_file,
        )?)
        .with_url(&icp_url)
        .build()?;
    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let file = tokio::fs::File::open(event_file).await?;
    let mut reader = BufReader::new(file);
    let mut events = vec![];
    loop {
        let len = match reader.read_u32().await {
            Ok(len) => len as usize,
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break, // end of file
            Err(e) => return Err(e.into()),
        };
        let mut event = vec![0u8; len];
        reader.read_exact(&mut event).await?;
        events.push(event);
    }
    println!("Read {} events. Uploading..", events.len());
    let result = agent
        .update(&canister_id, "pause_all_tasks")
        .with_arg(Encode!()?)
        .await?;

    let result: String = decode_one(&result)?;
    if result != *"Ok" {
        bail!("pause_all_tasks failed: {}", result);
    }

    for chunk in events.chunks(10_000) {
        let result = agent
            .update(&canister_id, "upload_events")
            .with_arg(Encode!(&chunk.to_vec())?)
            .await?;
        let result: Result<(), String> = decode_one(&result)?;
        if let Err(err) = result {
            bail!("upload_events failed: {}", err);
        }
    }

    let result = agent
        .update(&canister_id, "replace_events")
        .with_arg(Encode!()?)
        .await?;

    let result: Result<(), String> = decode_one(&result)?;
    if let Err(err) = result {
        bail!("replace_events failed: {}", err);
    }
    println!("Uploaded events.");
    println!("Please upgrade the canister yourself.");
    Ok(())
}

async fn count_events(file: &Path) -> Result<u64, eyre::Error> {
    if !file.exists() {
        return Ok(0);
    }
    let file = tokio::fs::File::open(file).await?;
    let mut reader = BufReader::new(file);
    let mut count = 0;
    loop {
        let len = match reader.read_u32().await {
            Ok(len) => len as usize,
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break, // end of file
            Err(e) => return Err(e.into()),
        };
        count += 1;
        let mut event = vec![0u8; len];
        reader.read_exact(&mut event).await?;
        print_event(&event);
    }
    println!("read {} events", count);
    Ok(count)
}

async fn write_events(
    writer: &mut BufWriter<tokio::fs::File>,
    events: Vec<Vec<u8>>,
) -> Result<(), eyre::Error> {
    for event in events {
        writer.write_u32(event.len() as u32).await?;
        writer.write_all(&event).await?;
    }
    writer.flush().await?;
    Ok(())
}

fn print_event(event: &[u8]) {
    let event: RootEvent = minicbor::decode(event).unwrap();
    let datetime: DateTime<Utc> = Utc
        .timestamp_millis_opt(event.timestamp.into_inner() as i64)
        .single()
        .expect("Invalid timestamp");
    println!("{}: {:?}", datetime, event.event)
}

fn print_events(events: &[Vec<u8>]) {
    for event in events {
        print_event(event);
    }
}

async fn fetch_events(
    canister_id: Principal,
    icp_url: String,
    event_file: PathBuf,
) -> Result<(), eyre::Error> {
    let agent = Agent::builder().with_url(&icp_url).build()?;

    if icp_url.contains("localhost") || icp_url.contains("127.0.0.1") {
        agent.fetch_root_key().await?;
    }

    let mut count = count_events(event_file.as_path()).await?;

    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(event_file.as_path())
        .await?;
    let mut writer = BufWriter::new(file);

    loop {
        match agent
            .query(&canister_id, "get_events_bin")
            .with_arg(Encode!(&1000_u64, &count)?)
            .await
        {
            Ok(result) => {
                let result: Result<Vec<Vec<u8>>, String> = decode_one(&result)?;
                match result {
                    Ok(events) => {
                        print_events(&events);
                        count += events.len() as u64;
                        write_events(&mut writer, events).await?;
                    }
                    Err(err) => {
                        return Err(eyre!("{}", err));
                    }
                }
            }
            Err(err) => {
                println!("Error in get_events_bin: {}", err);
                tokio::time::sleep(Duration::from_millis(10000)).await;
                continue;
            }
        };
        tokio::time::sleep(Duration::from_millis(5000)).await;
    }
}
