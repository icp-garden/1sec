//! This state machine fetches block headers and validates proofs submitted by
//! off-chain relayers.
//!
//! Note that it is not a persistent state machine because it doesn't use event
//! sourcing like other state machines. Persistence is not needed here because
//! the state can be easily restored after a canister upgrade:
//! - the new block headers can be refetched using EVM RPC.
//! - the off-chain relayers can resubmit their proofs.
//!
//! ## Fetching block headers
//!
//! Other state machines, like [evm::reader] and [evm::writer], rely on
//! [evm::prover] as the source of truth for the *safe block number* -- the
//! block number that is unlikely to get reorged and gives an acceptable user
//! latency.
//!
//! For simplicity, we define the safe block as a block that is `N` blocks behind
//! the latest block, where `N` is a configurable parameter
//! [head::Config::safety_margin]. Each EVM chain has its own `N` that can be chosen
//! based on the block rate, reorg probability, and the required user latency.
//!
//! Thus, the problem of fetching the safe block number reduces to fetching the
//! latest block number. The most straightforward way of passing
//! `BlockTag::Latest` to `eth_get_block_by_number()` does not work well for EVM
//! chains with high block rates such as Arbitrum (~4 block per second). Since
//! the response of the HTTP outcall changes rapidly, the ICP nodes often fail
//! to reach consensus.
//!
//! Instead of block tags, [evm::prover] uses block numbers when calling
//! `eth_get_block_by_number()` to ensure deterministic responses.
//!
//! **The algorithm of fetching the latest block header**:
//!
//! - State:
//!   - [State::head] stores all the state related to fetching block headers.
//!   - [head::State::latest] stores information about the previously fetched
//!     latest block: block number, block hash, and the time of fetching. This
//!     field is optional and is empty after a canister startup (init or upgrade).
//!   - [head::State::block_time_ms] stores the block time estimate.
//! - [Task::FetchLatestBlock] periodically updates [head::State::latest].
//! - Initialization:
//!    - after a canister startup, [Task::FetchLatestBlock] calls
//!      `eth_get_block_by_number()` with `BlockTag::Latest`.
//!    - If it gets an error (e.g. due to failed consensus), then it retries
//!      until a successful call.
//!    - Note that the empirical success rate is more than 10% even for Arbitrum,
//!      so the initialization will eventually succeed.
//!  - [Task::FetchLatestBlock] estimates the next block number based on
//!    - the block time estimate.
//!    - the current time.
//!    - the time of the previous fetch.
//!    - the previosly fetched block number.
//!  - [Task::FetchLatestBlock] calls `eth_get_block_by_number()` passing the
//!    estimated next block number.
//!  - If the call is successful, then this means that the estimated next block
//!    number exists:
//!    - the task updates [head::State::latest] with the fetched data.
//!    - the task decreases [head::State::block_time_ms] by a few percent (config parameter)
//!      to avoid falling behing the real latest block number.
//!  - If the call fails due to an unknown block number, then increases
//!    [head::State::block_time_ms] by a few percent (config parameter).
//!
//! Note that the feedback based correction of the estimated block time ensures
//! that [head::State::latest] fluctuates near the real latest block number.
//!
//! Once the new latest block number is successfully fetched,
//! [Task::FetchSafeBlock] fetches the new safe block number that is `N` blocks
//! behind the latest block number.
//!
//! ## Validating proofs from off-chain relayers.
//!
//! As performance optimization, the canister allows relayers to send EVM
//! transactions and upload Merkle proofs of the transaction receipts.
//!
//! A relayer can get a list of signed transactions to be sent by calling the
//! [get_relay_tasks()] query endpoint.
//!
//! Once the transaction is sent and executed, the relayer can submit a proof
//! of the transaction receipt by calling the [submit_relay_proof()] endpoint.
//!
//! There are two kinds of proofs:
//! - [RelayProof::EvmTransactionReceipt]:
//!   - a path in [Merkle-Patricia Tries](https://ethereum.org/en/developers/docs/data-structures-and-encoding/patricia-merkle-trie/)
//!     from an RLP encoded transaction to the root of the trie.
//!   - a path in [Merkle-Patricia Tries](https://ethereum.org/en/developers/docs/data-structures-and-encoding/patricia-merkle-trie/)
//!     from an RLP encoded receipt to the root of the trie.
//!   - additionally, it also contains the hash of the block that includes the transaction.
//! - [RelayProof::EvmBlockHeader]: a proof that a block header hashes to
//!   the given block hash.
//!
//! In order to validate a proof of the transaction and the receipt, the canister needs to
//! check the following properties:
//! 1. The Merkle paths are indeed valid and hash to the claimed root hashes.
//! 2. The corresponding block has the matching root hashes in its
//!    `transactionsRoot`  and `receiptsRoot` fields.
//! 3. There is a chain of blocks from some fetched safe block to the proof
//!    block following the parent hash field.
//!
//! Thus, [RelayProof::EvmTransactionReceipt] by itself can only help with property (1).
//! The relayer needs to submit one or more [RelayProof::EvmBlockHeader] to
//! convince the canister that properties (2) and (3) hold.
//!
//! When a relayer submits a proof, it might be the case that the [evm::prover]
//! hasn't yet fetched a safe block that could confirm the proof.
//! In order to reduce the ingress traffic, [evm::prover] doesn't reject such
//! proofs right away, but instead keeps them in the forest of blocks.
//!
//! The forrest is stored in [forest::State] and has the following structure:
//! - Nodes of the forest are blocks whose hashes have been checked against
//!   their headers.
//! - Edges of the forest are parent-child links (based on the parent hash field
//!   in the block header).
//! - Each block has a confirmation status: [Confirmation] that shows whether
//!   there is a path from some fetched safe block to that block in the forest.
//! - Each block has a list of submitted transaction/receipt proofs attached to it.
//! - Proofs that are attached to a confirmed block are considered confirmed and
//!   are sent to [evm::writer].
//!
//! ## Flow of validating a block proof.
//!
//! 1. The relayer calls [submit_relay_proof()] passing
//!    [RelayProof::EvmBlockHeader] with a block hash `H` and block header `B`
//! 2. [submit_relay_proof()] calls [validate_block_proof()] to check if the
//!    hash of the block header `B` is equal to the given hash `H`.
//! 3. If there is a mismatch, then the proof is rejected.
//! 4. Otherwise, the block is added to the forest (if it is not already there).
//! 5. If there is an existing block `C` with a parent hash `H` and that block
//!    is confirmed, then the new block `B` is also marked as confirmed and
//!    the confirmation is recursively propagated to the parent of `B` (if it is
//!    in the forest).
//! 6. All proofs of the newly confirmed blocks are moved to
//!    [forest::State::confirmed_proofs] to be sent to [evm::writer].
//!
//! ## Flow of validating a transaction/receipt proof.
//! 1. The relayer calls [submit_relay_proof()] passing
//!    [RelayProof::EvmTransactionReceipt].
//! 2. [submit_relay_proof()] calls [validate_tx_receipt_proof()].
//! 3. [validate_tx_receipt_proof()] looks up the block with the given hash in the forest.
//!    If the block doesn't exist, then the proof is rejected.
//!    This means that the relayer must submit the block proof with or before
//!    submitting a transaction/receipt proof.
//! 4. [validate_tx_receipt_proof()] checks that the roots of the  given
//!    Merkle-Patricia Trie paths  match with the `transactionsRoot` and
//!    `receiptsRoot` of the block.
//! 5. [validate_tx_receipt_proof()] validates hashes along the trie paths and
//!    checks that the computed roots match with the given roots.
//! 6. If any of the validation checks fail, then [submit_relay_proof()] rejects the proof.
//! 7. Otherwise:
//!    - If the block is confirmed, then the proof is also marked as confirmed and
//!      added to [forest::State::confirmed_proofs] to be sent to [evm::writer].
//!    - If the block is not confirmed yet, then the proof is added to the list
//!      of attached proofs of the block.
//!
//!
#[cfg(doc)]
use crate::{
    api::{
        queries::get_relay_tasks,
        types::{RelayProof, Token},
        updates::submit_relay_proof,
    },
    config::OperatingMode,
    evm, flow,
    flow::event::Operation,
};
#[cfg(doc)]
use evm_rpc_types::BlockTag;
#[cfg(doc)]
pub use forest::State as ForestState;
#[cfg(doc)]
pub use proof::{validate_tx_receipt_proof, Confirmation};

pub use config::Config;
pub use proof::ValidatedProof;
pub use proof::{decode_receipt, decode_tx, validate_block_proof, validate_trie_proof};
pub use state::State;
pub use task::Task;

pub mod endpoint;
pub mod head;

mod config;
mod forest;
mod proof;
mod state;
mod task;
