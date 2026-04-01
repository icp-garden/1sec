use lazy_static::lazy_static;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Metadata error: {0}")]
    Metadata(#[from] cargo_metadata::Error),
    #[error("Hash mismatch")]
    HashMismatch,
    #[error("Unknown canister")]
    UnknownCanister,
    #[error("Build failed: {0}")]
    BuildFailed(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Hash, Eq, PartialOrd, Ord, PartialEq)]
pub enum CanisterName {
    Local(String),
}

lazy_static! {
    static ref WORKSPACE_ROOT: PathBuf = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .expect("Failed to get workspace root")
        .workspace_root
        .into();
}

pub async fn get_wasm_path(name: CanisterName) -> Result<PathBuf> {
    match name {
        CanisterName::Local(name) => build_local_wasm(&name),
    }
}

pub fn get_wasm_path_sync(name: CanisterName) -> Result<PathBuf> {
    match name {
        CanisterName::Local(name) => build_local_wasm(&name),
    }
}

fn build_local_wasm(name: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(WORKSPACE_ROOT.join("artifacts"))?;

    let home_dir = std::env::var("HOME")
        .map_err(|e| Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, e)))?;
    let cargo_dir = PathBuf::from(home_dir).join(".cargo");

    let rustflags = format!(
        "RUSTFLAGS=\"--remap-path-prefix={}= --remap-path-prefix={}=\"",
        WORKSPACE_ROOT.display(),
        cargo_dir.display()
    );

    let file_name = name.to_string();

    let build_steps = [
        format!(
            "{0} cargo canister -p {1} --release --bin {1} --locked",
            rustflags,
            name,
        ),
        format!("ic-wasm target/wasm32-unknown-unknown/release/{0}.wasm -o artifacts/{0}.wasm metadata candid:service -f {0}/{0}.did -v public", name),
        format!("ic-wasm artifacts/{0}.wasm -o artifacts/{1}.wasm metadata git_commit_id -d $(git rev-parse HEAD) -v public", name, file_name),
        format!("ic-wasm artifacts/{0}.wasm shrink", file_name),
        format!("gzip -cnf9 artifacts/{0}.wasm > artifacts/{0}.wasm.gz", file_name),
        format!("rm artifacts/{0}.wasm", file_name),
    ];

    for cmd in &build_steps {
        if !std::process::Command::new("sh")
            .current_dir(&*WORKSPACE_ROOT)
            .args(["-c", cmd])
            .status()?
            .success()
        {
            return Err(Error::BuildFailed(cmd.to_string()));
        }
    }

    Ok(WORKSPACE_ROOT.join(format!("artifacts/{}.wasm.gz", file_name)))
}
