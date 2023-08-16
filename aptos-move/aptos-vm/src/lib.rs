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

mod access_path_cache;
#[macro_use]
pub mod counters;
pub mod data_cache;

mod adapter_common;
pub mod aptos_vm;
mod aptos_vm_impl;
pub mod block_executor;
mod errors;
pub mod move_vm_ext;
pub mod natives;
pub mod sharded_block_executor;
pub mod system_module_names;
pub mod transaction_metadata;
mod transaction_validation;
mod verifier;

pub use crate::aptos_vm::AptosVM;
use crate::sharded_block_executor::{executor_client::ExecutorClient, ShardedBlockExecutor};
use aptos_state_view::StateView;
use aptos_types::{
    block_executor::partitioner::PartitionedTransactions,
    transaction::{SignedTransaction, Transaction, TransactionOutput, VMValidatorResult},
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
        transactions: Vec<Transaction>,
        state_view: &(impl StateView + Sync),
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus>;

    /// Executes a block of transactions using a sharded block executor and returns the results.
    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        sharded_block_executor: &ShardedBlockExecutor<S, E>,
        transactions: PartitionedTransactions,
        state_view: Arc<S>,
        maybe_block_gas_limit: Option<u64>,
    ) -> Result<Vec<TransactionOutput>, VMStatus>;
}

/*
/// Get the AccessPath to a resource stored under `address` with type name `tag`
/// DNS
fn create_access_path(address: AccountAddress, tag: StructTag) -> anyhow::Result<AccessPath> {
    let resource_tag = ResourceKey::new(address, tag);
    AccessPath::resource_access_path(resource_tag)
}
 */
