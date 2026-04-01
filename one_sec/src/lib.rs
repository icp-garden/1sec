//! This crate implements a bridge between EVM chains, like Arbitrum and Base,
//! and ICP using a _lock-and-mint_ protocol.
//!
//! ## Design goals
//! - Multiple EVM chains in the same canister. In the future: other chains like
//!   BTC and Solana.
//! - Multiple tokens in the same canister.
//! - Bridging in both directions: EVM to ICP and ICP to EVM.
//!   For example, a user should be able to convert:
//!   - native `USDC` on EVM to wrapped `USDC` on ICP.
//!   - wrapped `USDC` on ICP to native `USDC` on EVM.
//!   - native `ICP` on ICP to wrapped `ICP` on EVM.
//!   - wrapped `ICP` on EVM to native `ICP` on ICP.
//! - Correctness and readability: the code should be easy to follow and
//!   to understand why it works.
//! - High transaction throughput: 10-100 TPS.
//!
//! ## Key features
//!
//! The high-level approach is similar to that of `ckUSDC` generalized to
//! multiple chains and both bridging directions.
//! - Threshold ECDSA gives the canister a secure EOA (Externally Owned
//!   Account) that holds locked tokens and signs EVM transactions.
//! - HTTP outcalls enable the canister to fetch blocks, event logs, and
//!   transaction receipts from EVM using multiple RPC providers.
//! - HTTP outcalls enable the canister to send signed transactions to EVM.
//! - The canister does not use the HTTP outcalls directly. Instead, it relies on
//!   the EVM RPC canister to performs the outcalls.
//! - Optional off-chain relayers help the canister to speed up sending
//!   transactions and receiving their receipts.
//!
//! The canister does not trust the relayers and verifies Merkle-Patricia trie
//! paths before accepting transaction receipts.
//!
//! ## User flow 1: a native token on EVM to a wrapped token on ICP
//! Let's say the user has `USDC` on Arbitrum.
//! The user can convert that into wrapped token `USDC` on ICP as follows:
//! 1. The user calls the `approve()` method of the `USDC` ERC20 contract on Arbitrum
//!    specifying the helper `Locker` contract address as the spender.
//! 2. The user calls the `lock()` method of the `Locker` contract on
//!    Arbitrum specifying the amount to lock and the ICP account that will
//!    receive the corresponding minted `USDC` tokens.
//! 3. `Locker` transfers `USDC` to the address of the canister and emits a log
//!    event to notify the canister.
//! 4. The canister eventually fetches the log event and calls the `USDC`
//!    ledger to mint the corresponding about of tokens to the recipient.
//!
//! See [flow] for the flow details in the implementation.
//!
//! ## User flow 2: a wrapped token on ICP to a native token on EVM
//! Converting `USDC` on ICP to `USDC` on Arbitrum works as follows:
//! 1. The user calls the `icrc2_approve()` endpoint of the `USDC` ledger specifying
//!    the canister as the spender.
//! 2. The user calls the `transfer()` endpoint of the canister to initiate
//!    burning `USDC` and unlocking of `USDC` to the specified recipient.
//! 3. The canister calls the `transferFrom()` endpoint of the `USDC` ledger to
//!    burn the tokens.
//! 4. The canister reserves the next transaction nonce for this flow.
//! 5. The canister creates an unlock transaction using the reserved nonce.
//! 6. The canister signs the unlock transaction using threshold ECDSA.
//! 7. The canister sends the transaction to EVM using the EVM RPC canister.
//! 8. The canister fetches the transaction receipt using the EVM RPC canister.
//! 9. If there is no receipt after some timeout, then the canister increases
//!    the transaction fee and repeats steps 5-9.
//! 10. Eventually, one of the unlock transactions is mined and `USDC` is sent
//!     to the recipient.
//!
//! Steps 6-8 can be accelerated using off-chain relayers:
//! 1. A relayer fetches a list of pending signed transactions that need to be
//!    sent to EVM by calling the `get_relay_tasks()` query endpoint of the canister.
//! 2. The relayer sends the transactions using its EVM RPC provider.
//! 3. For each mined transaction, the relayer fetches its receipt and builds
//!    a Merkle-Patricia trie proof.
//! 4. The relayer submits the proof along with the block header hash to the
//!    canister by calling the `submit_relay_proof()` endpoint of the canister.
//! 5. The canister verifies that the Merkle proof matches with the root hash
//!    specified in the block header and saves the proof as unconfirmed.
//! 6. An unconfirmed proof becomes confirmed when the canister observes a path
//!    for the block of the proof to a block that it fetched itself using the EVM
//!    RPC canister.
//! 7. Once the proof is confirmed, the canister accepts the receipt as if it
//!    fetched it itself in step 8 of the previous flow.
//!
//! See [flow] for the flow details in the implementation.
//!
//! ## User flow 3: a native token on ICP to a wrapped token on EVM
//! Let's say the user has `ICP` and wants to convert it into wrapped `ICP` on Arbitrum.
//! This requires locking `ICP` and minting `ICP`.
//! The flow steps are similar to those of "User flow 2" with the following changes:
//! - replace the burn of `USDC` with a lock of `ICP` on ICP.
//!   Both steps involve `approve()` and `transferFrom()` calls on the corresponding ledger.
//! - replace the unlock of `USDC` with a mint of `ICP` on EVM.
//!   Both steps require signing and sending a transaction to EVM.
//!
//! ## User flow 4: a wrapped token on EVM to a native token on ICP
//! In order to convert `ICP` on Arbitrum to `ICP` on ICP, the user needs to
//! perform steps that are similar to the steps of "User flow 1" with
//! the lock-and-mint operations replaced with burn-and-unlock:
//! - replace the lock of `USDC` with a burn of `ICP`.
//!   The ERC-20 contract of `ICP` has a special `burn()` method that emits
//!   `Burned(amount,icp_account)` log events, so there is no need to a separate
//!   locker-like helper contract.
//! - replace the mint of `USDC` with an unlock of `ICP`.
//!
//! ## Contracts and canisters
//! - The key-token canister defined in this crate that owns EVM contracts,
//!   holds locked tokens, mints wrapped tokens, and performs bridging.
//!
//! - EVM to ICP bridge: e.g. `USDC` <=> `USDC`:
//!   1. A third-party ERC-20 token contract of the native token: `USDC`.
//!   2. A locker contract defined in `contracts/evm/Locker.sol` that locks the
//!      ERC-20 token by transferring them to the canister's address and emits
//!      `Locked` log events. This contract is owned by the canister.
//!   3. An ICRC-2 ledger canister for the wrapped token: `USDC`.
//!      The key-token canister is set up as the minter for this ledger.
//!      The controller of the ledger canister is the same as the
//!      controller of the key-token canister.
//!
//! - ICP to EVM bridge: e.g. `ICP` <=> `ICP`:
//!   1. A third-party ICRC-2 ledger canister of the native token: `ICP`.
//!   2. An ERC-20 contract for the wrapped token: `ICP`.
//!      The contract is defined in `contracts/evm/Token.sol` and is owned
//!      by the canister. It allows the owner to mint tokens and has a special
//!      burn method that emits `Burned` events.
//!
//! ## Parties and trust assumptions
//! 1. **Trusted**: the ICP node providers and the NNS governance that control the subnet on
//!    which the canisters run. The standard trust assumptions of the ICP protocol
//!    apply here.
//! 2. **Trusted**: the controller of the key-token canister. The code does not
//!    attempt to protect against a malicous controller. The assumption is that the
//!    controller will be decentralized via SNS or other means in the future.
//! 3. **Trusted**: the EVM RPC canister. The code assumes that the EVM RPC
//!    canisters works as intended and correctly makes HTTP outcalls to the EVM
//!    RPC providers.
//! 4. **Untrusted**: EVM RPC providers. For critical information fetched
//!    from EVM such as safe block headers and event logs, the canister queries
//!    three independent providers and accepts the information only if at least two
//!    of them agree.
//! 5. **Untrusted**: off-chain transaction relayers. The relayers speed up
//!    sending EVM transactions and fetching receipts. Each relayer has to
//!    submit a Merkle-proof of an executed transaction and its receipt.
//!    Since the canister checks the proof against block headers it itself fetched
//!    from the EVM RPC providers, a malicious relayer cannot forge a fake
//!    transaction or receipt. **Note**: in the initial version, relayers are
//!    whitelisted to protect against denial-of-service attacks because a malicious
//!    relayer can force the canister to perform computationally expensive proof
//!    checks. In the future this restriction will be replaced with a staking
//!    mechanism that discourages relayers from making excessive number of calls.
//! 6. **Untrusted**: users. The code protects against malicious users both for
//!    correctness and availability (using fees).
//!
//! ## Implementation
//!
//! The canister is implemented as a state machine following the [event
//! sourcing](https://mmapped.blog/posts/19-eventlog#solution) pattern.
//! *Events* trigger *state transitions* and are stored in the stable log
//! data-structure in the stable memory. [Do not confuse this event with an EVM
//! log event. Even though they share the same name, they represent different
//! concepts.]
//!
//! The advantage of the event sourcing pattern is that it is possible to
//! deterministically replay all events to arrive at the current state of the
//! state machine. This makes canister upgrades trivial and removes the
//! requirement to serialize the state in the stable memory.
//!
//! In order to manage complexity and improve readability, the large state
//! machine is divided up into hierarchy of smaller state machines with
//! well-defined responsibilities. This enables local reasoning where one can
//! focus on a single state machine at a time.
//!
//! The best way to explore the state machines is to start with the `[flow]`
//! state machine and go through the bridging flows described in its
//! documentation.
//!
//! Here is a summary overview of the state machines:
//! - [state::State]: the global state machine
//!    - [flow]: keeps track of bridging requests (transfer) from users.
//!    - [icp]: ICP-related state
//!      - [icp::ledger]: one per ICP token
//!         - maintains the token balance,
//!         - supports: lock, unlock, mint, burn operations.
//!         - communicates with the ICRC-2 ledger canister,
//!      - ECDSA public key,
//!      - exchange rates for tokens.
//!    - [evm]: one per EVM chain (e.g. Arbitrum, Base)
//!         - [evm::ledger]: one per EVM token
//!           - maintains the token balance,
//!           - supports: lock, unlock, mint, burn operations.
//!         - [evm::prover]:
//!             - fetches the latest block header using EVM RPC.
//!             - fetches the safe block header using EVM RPC.
//!             - accepts Merkle proofs from relayers.
//!             - verifies the proofs.
//!         - [evm::reader]:
//!             - fetches EVM event logs using EVM RPC.
//!         - [evm::writer]:
//!             - maintains the transaction nonce,
//!             - signs transactions,
//!             - sends transactions,
//!             - fetches transaction receipts.
//!

pub mod api;
pub mod config;
pub mod event;
pub mod evm;
pub mod flow;
pub mod icp;
pub mod numeric;
pub mod state;
pub mod task;

mod cbor;
mod dashboard;
mod guards;
mod logs;
mod management;
mod metrics;
mod storage;

#[macro_use]
extern crate assert_matches;
