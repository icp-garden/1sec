use std::process::{exit, Command};

fn main() {
    println!("cargo::rerun-if-changed=../one_sec");
    println!("cargo::rerun-if-changed=canisters.sh");
    println!("cargo::rerun-if-changed=build.rs");
    let script = std::env::current_dir().unwrap().join("canisters.sh");
    let result = Command::new("bash").args([script]).status().unwrap();
    if !result.success() {
        eprintln!("Failed to build canisters: {}", result);
        exit(result.code().unwrap_or(0));
    }
}
