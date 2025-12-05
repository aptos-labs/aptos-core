// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod iterators;
pub(crate) mod truncation_helper;

use crate::schema::db_metadata::{DbMetadataKey, DbMetadataSchema};
use aptos_schemadb::{batch::NativeBatch, DB};
use aptos_storage_interface::Result;
use aptos_types::{state_store::NUM_STATE_SHARDS, transaction::Version};

pub(crate) type ShardedStateKvSchemaBatch<'db> = [NativeBatch<'db>; NUM_STATE_SHARDS];

pub(crate) fn get_progress(db: &DB, progress_key: &DbMetadataKey) -> Result<Option<Version>> {
    Ok(db
        .get::<DbMetadataSchema>(progress_key)?
        .map(|v| v.expect_version()))
}
