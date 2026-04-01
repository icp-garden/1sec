# OneSec Bridge

A cross-chain bridge between ICP and EVM chains (Base, Arbitrum, Ethereum) using a lock-and-mint protocol.

## Supported tokens

| Token | ICP mode | EVM mode | Chains |
|-------|----------|----------|--------|
| ICP | Locker | Minter | Base, Arbitrum, Ethereum |
| GLDT | Locker | Minter | Base, Arbitrum, Ethereum |
| ckBTC | Locker | Minter | Base, Arbitrum, Ethereum |
| CHAT | Locker | Minter | Base, Arbitrum, Ethereum |
| BOB | Locker | Minter | Base, Arbitrum, Ethereum |
| USDC | Minter | Locker | Base, Arbitrum, Ethereum |
| USDT | Minter | Locker | Ethereum |
| cbBTC | Minter | Locker | Base, Arbitrum, Ethereum |

## Architecture

The canister uses event sourcing: all state changes are recorded as events in a stable memory log. On upgrade, the full state is reconstructed by replaying events.

Key components:
- **flow** -- orchestrates two-step bridging transfers (ICP-to-EVM and EVM-to-ICP)
- **evm** -- reader (fetches logs), writer (signs/sends txs), prover (verifies block headers and Merkle proofs), forwarder (derived-address forwarding)
- **icp** -- ICP ledger interactions (ICRC-1/ICRC-2 transfers, mint/burn)
- **relayer** -- off-chain helper that submits relay proofs and forwarding updates (untrusted, all proofs are verified on-chain)

## Project structure

```
one_sec/       -- main canister (Rust, compiles to Wasm)
relayer/       -- off-chain relayer binary
tests/         -- integration tests (PocketIC)
ic_wasm_utils/ -- Wasm build utilities
backup/        -- event backup/restore tool
frontend/      -- web frontend (SvelteKit)
```

## Documentation

The detailed design documentation is in the crate-level docs:

```
cargo doc -p one_sec --no-deps
open target/doc/one_sec/index.html
```

Or browse [the pre-generated docs](https://luxbj-aqaaa-aaaah-qqbdq-cai.icp0.io/doc/one_sec/index.html).

## Development

Build the canister:

```
./build.sh
```

Run tests:

```
cd tests && cargo test
```

See [HACKING.md](HACKING.md) for local setup, staging deployment, and backup/restore instructions.
