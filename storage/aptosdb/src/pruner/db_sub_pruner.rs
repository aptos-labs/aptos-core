// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_schemadb::SchemaBatch;

/// Defines the trait for sub-pruner of a parent DB pruner
pub trait DBSubPruner {
    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()>;
}
