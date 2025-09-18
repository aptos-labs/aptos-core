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
use aptos_block_executor::txn_provider::default::DefaultTxnProvider;
use aptos_types::{
    block_executor::{
        config::BlockExecutorConfigFromOnchain, partitioner::PartitionedTransactions,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    state_store::StateView,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, AuxiliaryInfo, BlockOutput,
        SignedTransaction, TransactionOutput, VMValidatorResult,
    },
    vm_status::VMStatus,
};
use move_vm_runtime::ModuleStorage;
use std::{marker::Sync, sync::Arc};
pub use verifier::view_function::determine_is_view;

/// This trait describes the VM's validation interfaces.
pub trait VMValidator {
    /// Executes the prologue of the Aptos Account and verifies that the transaction is valid.
    fn validate_transaction(
        &self,
        transaction: SignedTransaction,
        state_view: &impl StateView,
        module_storage: &impl ModuleStorage,
    ) -> VMValidatorResult;
}

/// This trait describes the block executor interface which is responsible for executing a block of
/// transactions. In general, block execution returns a vector of transaction outputs. This vector
/// has the same length as the input vector of transactions. In case transactions are skipped or
/// discarded, they are still included - but their output is empty. The outputs are not applied to
/// the state directly. It is the responsibility of the caller to update the state accordingly.
pub trait VMBlockExecutor: Send + Sync {
    /// Be careful if any state (such as caches) is kept in [VMBlockExecutor]. It is the
    /// responsibility of the implementation to ensure the state is valid across multiple
    /// executions. For example, the same executor may be used to run on a new state, and then on
    /// an old one.
    fn new() -> Self;

    /// Executes a block of transactions and returns output for each one of them.
    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<SignatureVerifiedTransaction, TransactionOutput>, VMStatus>;

    /// Executes a block of transactions and returns output for each one of them, without applying
    /// any block limit.
    fn execute_block_no_limit(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction, AuxiliaryInfo>,
        state_view: &(impl StateView + Sync),
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        self.execute_block(
            txn_provider,
            state_view,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
            // For all use cases, we run on an unknown state.
            TransactionSliceMetadata::unknown(),
        )
        .map(BlockOutput::into_transaction_outputs_forced)
    }

    /// Executes a block of transactions using a sharded block executor and returns the results.
    fn execute_block_sharded<S: StateView + Sync + Send + 'static, E: ExecutorClient<S>>(
        _sharded_block_executor: &ShardedBlockExecutor<S, E>,
        _transactions: PartitionedTransactions,
        _state_view: Arc<S>,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<Vec<TransactionOutput>, VMStatus> {
        unimplemented!("sharded not supported")
    }
}
