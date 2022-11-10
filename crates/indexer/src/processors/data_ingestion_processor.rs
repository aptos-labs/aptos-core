// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bigquery_client::{extract_from_api_transactions, BigQueryClient, TypedAppendRowsRequest},
    database::PgDbPool,
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
};
use aptos_api_types::Transaction as APITransaction;
use async_trait::async_trait;
use std::fmt::Debug;
use std::sync::Arc;

pub const NAME: &str = "bigquery_default_processor";

pub struct DataIngestionProcessor {
    connection_pool: PgDbPool,
    bigquery_client: Arc<BigQueryClient>,
    bigquery_project_id: String,
}

impl DataIngestionProcessor {
    pub async fn new(connection_pool: PgDbPool, bigquery_project_id: String) -> Self {
        let bigquery_client = Arc::new(BigQueryClient::new(bigquery_project_id.clone()).await);
        Self {
            connection_pool,
            bigquery_client,
            bigquery_project_id,
        }
    }
}

impl Debug for DataIngestionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataIngestionProcessor {{ project id: {:?} }}",
            self.bigquery_project_id,
        )
    }
}

#[async_trait]
impl TransactionProcessor for DataIngestionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }
    async fn process_transactions(
        &self,
        transactions: Vec<APITransaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let append_row_request = extract_from_api_transactions(&transactions);
        match self
            .bigquery_client
            .send_data(
                TypedAppendRowsRequest::Transactions(append_row_request),
                start_version,
                end_version,
            )
            .await
        {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::BigQueryTransactionCommitError(
                (
                    anyhow::Error::msg("123"),
                    start_version,
                    end_version,
                    self.name(),
                ),
            )),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
