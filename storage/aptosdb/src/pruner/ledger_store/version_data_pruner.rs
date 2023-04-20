// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{pruner::db_sub_pruner::DBSubPruner, schema::version_data::VersionDataSchema};
use aptos_schemadb::SchemaBatch;

#[derive(Debug)]
pub struct VersionDataPruner {}

impl DBSubPruner for VersionDataPruner {
    fn prune(
        &self,
        db_batch: &mut SchemaBatch,
        min_readable_version: u64,
        target_version: u64,
    ) -> anyhow::Result<()> {
        for version in min_readable_version..target_version {
            db_batch.delete::<VersionDataSchema>(&version)?;
        }
        Ok(())
    }
}

impl VersionDataPruner {
    pub(in crate::pruner) fn new() -> Self {
        VersionDataPruner {}
    }
}
