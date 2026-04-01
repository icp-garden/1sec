#!/usr/bin/env bash

set -e

dfx build --ic one_sec
dfx canister stop one_sec --ic --wallet 54mbv-kyaaa-aaaar-qbn5a-cai
dfx canister install one_sec --ic --wallet 54mbv-kyaaa-aaaar-qbn5a-cai --argument-file one_sec/prod_upgrade_arg.did --mode upgrade
dfx canister start one_sec --ic --wallet 54mbv-kyaaa-aaaar-qbn5a-cai