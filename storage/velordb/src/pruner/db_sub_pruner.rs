// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_storage_interface::Result;
use velor_types::transaction::Version;
/// Defines the trait for sub-pruner of a parent DB pruner
pub trait DBSubPruner {
    /// Returns the name of the sub pruner.
    fn name(&self) -> &str;

    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()>;
}
