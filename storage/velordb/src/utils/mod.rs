// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod iterators;
pub(crate) mod truncation_helper;

use crate::schema::db_metadata::{DbMetadataKey, DbMetadataSchema};
use velor_schemadb::{batch::NativeBatch, DB};
use velor_storage_interface::Result;
use velor_types::{state_store::NUM_STATE_SHARDS, transaction::Version};

pub(crate) type ShardedStateKvSchemaBatch<'db> = [NativeBatch<'db>; NUM_STATE_SHARDS];

pub(crate) fn get_progress(db: &DB, progress_key: &DbMetadataKey) -> Result<Option<Version>> {
    Ok(db
        .get::<DbMetadataSchema>(progress_key)?
        .map(|v| v.expect_version()))
}
