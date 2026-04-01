# Reproduce Build

```shell
git clone https://github.com/dfinity/ic
cd ic
git checkout ebebe0c1ffe8c1cb30396c2e34e447a6f48e40a8
./ci/container/build-ic.sh -c
sha256sum artifacts/canisters/ic-icrc1-ledger.wasm.gz
sha256sum artifacts/canisters/ic-icrc1-index-ng.wasm.gz
```