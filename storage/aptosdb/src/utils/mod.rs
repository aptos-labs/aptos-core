// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod iterators;
pub(crate) mod truncation_helper;

use crate::schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue};
use anyhow::Result;
use aptos_schemadb::DB;
use aptos_types::transaction::Version;

pub(crate) fn get_progress(db: &DB, progress_key: &DbMetadataKey) -> Result<Option<Version>> {
    Ok(
        if let Some(DbMetadataValue::Version(progress)) =
            db.get::<DbMetadataSchema>(progress_key)?
        {
            Some(progress)
        } else {
            None
        },
    )
}
