#!/usr/bin/env bash

set -e

dfx build --ic one_sec_staging
dfx canister stop one_sec_staging --ic
dfx canister install one_sec_staging --ic --argument-file one_sec/test_upgrade_arg.did --mode upgrade
dfx canister start one_sec_staging --ic