// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod iterators;
pub(crate) mod truncation_helper;

use crate::schema::db_metadata::{DbMetadataKey, DbMetadataSchema};
use anyhow::Result;
use aptos_schemadb::DB;
use aptos_types::transaction::Version;

pub(crate) fn get_progress(db: &DB, progress_key: &DbMetadataKey) -> Result<Option<Version>> {
    Ok(db
        .get::<DbMetadataSchema>(progress_key)?
        .map(|v| v.expect_version()))
}
