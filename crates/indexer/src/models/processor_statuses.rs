// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]
use crate::{indexer::errors::TransactionProcessingError, schema::processor_statuses};
use field_count::FieldCount;

#[derive(AsChangeset, Debug, FieldCount, Insertable, Queryable)]
#[diesel(treat_none_as_null = true)]
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
            version: version as i64,
            success,
            details,
            last_updated: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_transaction_processing_err(tpe: &TransactionProcessingError) -> Vec<Self> {
        let (error, start_version, end_version, name) = tpe.inner();
        Self::from_versions(
            name,
            *start_version,
            *end_version,
            false,
            Some(error.to_string()),
        )
    }

    pub fn from_versions(
        name: &'static str,
        start_version: u64,
        end_version: u64,
        success: bool,
        details: Option<String>,
    ) -> Vec<Self> {
        let mut status: Vec<Self> = vec![];
        for version in start_version..(end_version + 1) {
            status.push(Self::new(name, version as i64, success, details.clone()));
        }
        status
    }
}

// Prevent conflicts with other things named `ProcessorStatus`
pub type ProcessorStatusModel = ProcessorStatus;
