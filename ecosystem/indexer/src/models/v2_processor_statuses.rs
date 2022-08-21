// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{
    indexer::{errors::TransactionProcessingError, processing_result::ProcessingResult},
    schema::v2_processor_statuses as processor_statuss,
};

#[derive(AsChangeset, Debug, Insertable, Queryable)]
#[diesel(table_name = processor_statuses)]
pub struct ProcessorStatus {
    pub name: &'static str,
    pub block_height: i64,
    pub success: bool,
    pub details: Option<String>,
    pub last_updated: chrono::NaiveDateTime,
}

impl ProcessorStatus {
    pub fn new(
        name: &'static str,
        block_height: i64,
        success: bool,
        details: Option<String>,
    ) -> Self {
        Self {
            name,
            block_height,
            success,
            details,
            last_updated: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_processing_result_ok(processing_result: &ProcessingResult) -> Self {
        Self::new(
            processing_result.name,
            processing_result.block_height as i64,
            true,
            None,
        )
    }

    pub fn from_transaction_processing_err(tpe: &TransactionProcessingError) -> Self {
        let (error, block_height, name) = tpe.inner();

        Self::new(name, *block_height as i64, false, Some(error.to_string()))
    }

    pub fn for_mark_started(name: &'static str, block_height: u64) -> Self {
        Self::new(name, block_height as i64, false, None)
    }
}

// Prevent conflicts with other things named `ProcessorStatus`
pub type ProcessorStatusModel = ProcessorStatus;
