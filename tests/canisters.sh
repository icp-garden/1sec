#!/bin/bash

cd ..
dfx build one_sec

cd contracts/evm
forge build
cd ../..

if [ -d "$HOME/.cache/dfinity/pulled/7hfb6-caaaa-aaaar-qadga-cai" ] &&
   [ -d "$HOME/.cache/dfinity/pulled/ryjl3-tyaaa-aaaaa-aaaba-cai" ] &&
   [ -d "$HOME/.cache/dfinity/pulled/uf6dk-hyaaa-aaaaq-qaaaq-cai" ]
then
  echo "All remote canisters exist: skipping dfx pull."
else
  dfx deps pull
fi

ls -l .dfx/local/canisters/one_sec/* | grep "\(wasm\|service.did\$\)"
ls -l $HOME/.cache/dfinity/pulled/*

