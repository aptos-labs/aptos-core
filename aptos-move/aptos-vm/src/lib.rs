// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(deprecated)]

//! # The VM runtime
//!
//! ## Transaction flow
//!
//! This is the path taken to process a single transaction.
//!
//! ```text
//!                   SignedTransaction
//!                            +
//!                            |
//! +--------------------------|-------------------+
//! | Validate  +--------------+--------------+    |
//! |           |                             |    |
//! |           |       check signature       |    |
//! |           |                             |    |
//! |           +--------------+--------------+    |
//! |                          |                   |
//! |                          |                   |
//! |                          v                   |
//! |           +--------------+--------------+    |
//! |           |                             |    |
//! |           |      check size and gas     |    |
//! |           |                             |    +---------------------------------+
//! |           +--------------+--------------+    |         validation error        |
//! |                          |                   |                                 |
//! |                          |                   |                                 |
//! |                          v                   |                                 |
//! |           +--------------+--------------+    |                                 |
//! |           |                             |    |                                 |
//! |           |         run prologue        |    |                                 |
//! |           |                             |    |                                 |
//! |           +--------------+--------------+    |                                 |
//! |                          |                   |                                 |
//! +--------------------------|-------------------+                                 |
//!                            |                                                     |
//! +--------------------------|-------------------+                                 |
//! |                          v                   |                                 |
//! |  Verify   +--------------+--------------+    |                                 |
//! |           |                             |    |                                 |
//! |           |     deserialize script,     |    |                                 |
//! |           |     verify arguments        |    |                                 |
//! |           |                             |    |                                 |
//! |           +--------------+--------------+    |                                 |
//! |                          |                   |                                 |
//! |                          |                   |                                 v
//! |                          v                   |                    +----------------+------+
//! |           +--------------+--------------+    |                    |                       |
//! |           |                             |    +------------------->+ discard, no write set |
//! |           |     deserialize modules     |    | verification error |                       |
//! |           |                             |    |                    +----------------+------+
//! |           +--------------+--------------+    |                                 ^
//! |                          |                   |                                 |
//! |                          |                   |                                 |
//! |                          v                   |                                 |
//! |           +--------------+--------------+    |                                 |
//! |           |                             |    |                                 |
//! |           | verify scripts and modules  |    |                                 |
//! |           |                             |    |                                 |
//! |           +--------------+--------------+    |                                 |
//! |                          |                   |                                 |
//! +--------------------------|-------------------+                                 |
//!                            |                                                     |
//! +--------------------------|-------------------+                                 |
//! |                          v                   |                                 |
//! | Execute   +--------------+--------------+    |                                 |
//! |           |                             |    |                                 |
//! |           |        execute main         |    |                                 |
//! |           |                             |    |                                 |
//! |           +--------------+--------------+    |                                 |
//! |                          |                   |                                 |
//! |      success or failure  |                   |                                 |
//! |                          v                   |                                 |
//! |           +--------------+--------------+    |                                 |
//! |           |                             |    +---------------------------------+
//! |           |        run epilogue         |    | invariant violation (internal panic)
//! |           |                             |    |
//! |           +--------------+--------------+    |
//! |                          |                   |
//! |                          |                   |
//! |                          v                   |
//! |           +--------------+--------------+    |                    +-----------------------+
//! |           |                             |    | execution failure  |                       |
//! |           |       make write set        +------------------------>+ keep, only charge gas |
//! |           |                             |    |                    |                       |
//! |           +--------------+--------------+    |                    +-----------------------+
//! |                          |                   |
//! +--------------------------|-------------------+
//!                            |
//!                            v
//!             +--------------+--------------+
//!             |                             |
//!             |  keep, transaction executed |
//!             |        + gas charged        |
//!             |                             |
//!             +-----------------------------+
//! ```

#[macro_use]
pub mod counters;
pub mod data_cache;

pub mod aptos_vm;
pub mod block_executor;
mod errors;
pub mod gas;
#[cfg(not(feature = "testing"))]
mod keyless_validation;
#[cfg(feature = "testing")]
pub mod keyless_validation;
pub mod move_vm_ext;
pub mod natives;
pub mod sharded_block_executor;
pub mod system_module_names;
pub mod testing;
pub mod transaction_metadata;
mod transaction_validation;
pub mod validator_txns;
pub mod verifier;

pub use crate::aptos_vm::{AptosSimulationVM, AptosVM};
use crate::sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor};
use aptos_crypto::HashValue;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
    },
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput,
        SignedTransaction, TransactionOutput, VMValidatorResult,
    },
    vm_status::VMStatus,
};
use aptos_vm_types::module_and_script_storage::code_storage::AptosCodeStorage;
use std::{marker::Sync, sync::Arc};
pub use verifier::view_function::determine_is_view;

/// This trait describes the VM's validation interfaces.
pub trait VMValidator {
    /// Executes the prologue of the Aptos Account and verifies that the transaction is valid.
    fn validate_transaction(
        &self,
        transaction: SignedTransaction,
        state_view: &impl StateView,
        module_storage: &impl AptosCodeStorage,
    ) -> VMValidatorResult;
}

/// This trait describes the block executor interface.
pub trait VMBlockExecutor: Send + Sync {
    /// Be careful if any state (such as caches) is kept in [VMBlockExecutor]. It is the
    /// responsibility of the implementation to ensure the state is valid across multiple
    /// executions. For example, the same executor may be used to run on a new state, and then on
    /// an old one.
    fn new() -> Self;

    /// Executes a block of transactions and returns output for each one of them.
    fn execute_block(
        &self,
        transactions: &[SignatureVerifiedTransaction],
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
        parent_block: Option<&HashValue>,
        current_block: Option<HashValue>,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus>;

    /// Executes a block of transactions and returns output for each one of them, without applying
    /// any block limit.
    fn execute_block_no_limit(
        &self,
        transactions: &[SignatureVerifiedTransaction],
        state_view: &(impl StateView + Sync),
        parent_block: Option<&HashValue>,
        current_block: Option<HashValue>,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        self.execute_block(
            transactions,
            state_view,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
            parent_block,
            current_block,
        )
        .map(BlockOutput::into_transaction_outputs_forced)
    }

    /// Executes a block of transactions using a sharded block executor and returns the results.
    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        sharded_block_executor: &ShardedBlockExecutor<S, E>,
        transactions: PartitionedTransactions,
        state_view: Arc<S>,
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>, VMStatus>;
}
