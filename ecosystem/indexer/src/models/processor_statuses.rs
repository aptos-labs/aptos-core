// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    indexer::{errors::TransactionProcessingError, processing_result::ProcessingResult},
    schema::processor_statuses as processor_statuss,
};

#[derive(AsChangeset, Debug, Insertable, Queryable)]
#[diesel(table_name = processor_statuses)]
pub struct ProcessorStatus {
    pub name: &'static str,
    pub version: i64,
    pub success: bool,
    pub details: Option<String>,
    pub last_updated: chrono::NaiveDateTime,
}

impl ProcessorStatus {
    pub fn new(name: &'static str, version: i64, success: bool, details: Option<String>) -> Self {
        Self {
            name,
            version,
            success,
            details,
            last_updated: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_processing_result_ok(processing_result: &ProcessingResult) -> Self {
        Self::new(
            processing_result.name,
            processing_result.version as i64,
            true,
            None,
        )
    }

    pub fn from_transaction_processing_err(tpe: &TransactionProcessingError) -> Self {
        let (error, version, name) = tpe.inner();

        Self::new(name, *version as i64, false, Some(error.to_string()))
    }

    pub fn for_mark_started(name: &'static str, version: i64) -> Self {
        Self::new(name, version, false, None)
    }
}

// Prevent conflicts with other things named `ProcessorStatus`
pub type ProcessorStatusModel = ProcessorStatus;
