// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

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

#[cfg(test)]
mod tests;
#[macro_use]
pub mod counters;
pub mod data_cache;

pub mod aptos_vm;
pub mod block_executor;
mod errors;
mod gas;
pub mod move_vm_ext;
pub mod natives;
pub mod sharded_block_executor;
pub mod system_module_names;
pub mod testing;
pub mod transaction_metadata;
mod transaction_validation;
pub mod validator_txns;
pub mod verifier;
mod zkid_validation;

pub use crate::aptos_vm::{AptosSimulationVM, AptosVM};
use crate::sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor};
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
use std::{marker::Sync, sync::Arc};
pub use verifier::view_function::determine_is_view;

/// This trait describes the VM's validation interfaces.
pub trait VMValidator {
    /// Executes the prologue of the Aptos Account and verifies that the transaction is valid.
    fn validate_transaction(
        &self,
        transaction: SignedTransaction,
        state_view: &impl StateView,
    ) -> VMValidatorResult;
}

/// This trait describes the VM's execution interface.
pub trait VMExecutor: Send + Sync {
    // NOTE: At the moment there are no persistent caches that live past the end of a block (that's
    // why execute_block doesn't take &self.)
    // There are some cache invalidation issues around transactions publishing code that need to be
    // sorted out before that's possible.

    /// Executes a block of transactions and returns output for each one of them.
    fn execute_block(
        transactions: &[SignatureVerifiedTransaction],
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus>;

    /// Executes a block of transactions and returns output for each one of them,
    /// Without applying any block limit
    fn execute_block_no_limit(
        transactions: &[SignatureVerifiedTransaction],
        state_view: &(impl StateView + Sync),
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        Self::execute_block(
            transactions,
            state_view,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
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
