// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bigquery_client::{extract_from_api_transactions, BigQueryClient},
    database::PgDbPool,
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
};

use aptos_api_types::Transaction as APITransaction;
use async_trait::async_trait;
use futures_util::stream;
use std::fmt::Debug;
use tonic;

pub const NAME: &str = "data_ingestion_processor";

pub struct DataIngestionProcessor {
    connection_pool: PgDbPool,
    bigquery_client: BigQueryClient,
    bigquery_stream_id: String,
    bigquery_project_id: String,
    bigquery_dataset_name: String,
    bigquery_table_name: String,
    cloud_resource_prefix: String,
}

impl DataIngestionProcessor {
    pub fn new(
        connection_pool: PgDbPool,
        bigquery_client_and_stream: (BigQueryClient, String),
        bigquery_project_id: Option<String>,
        bigquery_dataset_name: Option<String>,
        bigquery_table_name: Option<String>,
    ) -> Self {
        let project_id = if let Some(project_id) = bigquery_project_id {
            project_id
        } else {
            panic!("BigQuery project id is not set for DataIngestionProcessor!");
        };

        let dataset_name = if let Some(name) = bigquery_dataset_name {
            name
        } else {
            panic!("BigQuery dataset name is not set for DataIngestionProcessor!");
        };

        let table_name = if let Some(name) = bigquery_table_name {
            name
        } else {
            panic!("BigQuery table name is not set for DataIngestionProcessor!");
        };
        let cloud_resource_prefix =
            format!("projects/{project_id}/datasets/{dataset_name}/tables/{table_name}");
        aptos_logger::info!(
            project_id = project_id,
            dataset_name = dataset_name,
            table_name = table_name,
            cloud_resource_prefix = cloud_resource_prefix,
            "init DataIngestionProcessor"
        );

        Self {
            connection_pool,
            bigquery_client: bigquery_client_and_stream.0,
            bigquery_stream_id: bigquery_client_and_stream.1,
            bigquery_project_id: project_id,
            bigquery_dataset_name: dataset_name,
            bigquery_table_name: table_name,
            cloud_resource_prefix,
        }
    }
}

impl Debug for DataIngestionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DataIngestionProcessor {{ project id: {:?}  dataset name: {:?} table name: {:?}}}",
            self.bigquery_project_id, self.bigquery_dataset_name, self.bigquery_table_name
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
            .get()
            .append_rows(tonic::Request::new(stream::iter(vec![append_row_request])))
            .await
        {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::BigQueryTransactionCommitError(
                (
                    anyhow::Error::from(err),
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
