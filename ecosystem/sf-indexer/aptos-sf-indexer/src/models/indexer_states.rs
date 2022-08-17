// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]

use crate::{
    indexer::{errors::BlockProcessingError, processing_result::ProcessingResult},
    schema::indexer_states,
};

#[derive(AsChangeset, Debug, Insertable, Queryable)]
#[diesel(table_name = "indexer_states")]
#[changeset_options(treat_none_as_null = "true")]
pub struct IndexerState {
    pub substream_module: String,
    pub block_height: i64,
    pub success: bool,
    pub details: Option<String>,
    pub last_updated: chrono::NaiveDateTime,
}

impl IndexerState {
    pub fn new(
        substream_module: String,
        block_height: i64,
        success: bool,
        details: Option<String>,
    ) -> Self {
        Self {
            substream_module,
            block_height,
            success,
            details,
            last_updated: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_processing_result_ok(processing_result: &ProcessingResult) -> Self {
        Self::new(
            processing_result.substream_module.to_string(),
            processing_result.block_height as i64,
            true,
            None,
        )
    }

    pub fn from_block_processing_err(bpe: &BlockProcessingError) -> Self {
        let (error, block_height, substream_module) = bpe.inner();

        Self::new(
            substream_module.to_string(),
            *block_height as i64,
            false,
            Some(error.to_string()),
        )
    }

    pub fn for_mark_started(substream_module: String, block_height: i64) -> Self {
        Self::new(substream_module, block_height, false, None)
    }
}
