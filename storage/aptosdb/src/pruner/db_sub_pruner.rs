// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use schemadb::SchemaBatch;
use std::fmt::Debug;

/// Defines the trait for sub-pruner of a parent DB pruner
pub trait DBSubPruner: Debug {
    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()>;
}
