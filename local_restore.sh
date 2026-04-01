#!/usr/bin/env bash

set -e

if [ ! -f backup/local.pem ]; then
  echo "Please export the local (unsecure) identity to backup/local.pem using 'dfx identity export default > backup/local.pem'"
fi

if [ ! -f backup/events.data ]; then
  echo "Please backup mainnet events first by running 'cd backup; cargo run'"
fi

dfx canister create one_sec_staging
cargo build --target wasm32-unknown-unknown --release -p one_sec --locked --features=dev
dfx canister install one_sec_staging --mode=reinstall --wasm ./target/wasm32-unknown-unknown/release/one_sec.wasm --argument-file ./one_sec/prod_init_arg.did

cd backup
cargo run -- --canister-id zvjow-lyaaa-aaaar-qap7q-cai --icp-url http://127.0.0.1:8080 --restore --identity ./local.pem
cd ..

dfx deploy one_sec_staging --argument-file one_sec/prod_upgrade_arg.did
dfx canister call one_sec_staging pause_all_tasks '()'

# Try another upgrade to see that there are no pre-upgrade failures.
dfx deploy one_sec_staging --argument-file one_sec/prod_upgrade_arg.did
dfx canister call one_sec_staging pause_all_tasks '()'

