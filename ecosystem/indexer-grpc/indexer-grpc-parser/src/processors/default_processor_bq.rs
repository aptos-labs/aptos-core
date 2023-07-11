// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    default_processor::DefaultTransactionProcessor,
    processor_trait::{ProcessingResult, ProcessorTrait},
};
use crate::{
    models::default_models::transactions::TransactionModel,
    schema,
    utils::{
        bigquery_client::{get_txn_request, BigQueryClient, TypedAppendRowsRequest},
        database::{
            clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
        },
        util::bigdecimal_to_u64,
    },
};
use anyhow::bail;
use aptos_protos::{
    bigquery_schema::transaction::v1::Transaction as TransactionBQ, transaction::v1::Transaction,
};
use async_trait::async_trait;
use prost::Message;
use std::{
    fmt::Debug,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::error;

pub const NAME: &str = "default_processor_bq";
pub struct DefaultTransactionProcessorBq {
    connection_pool: PgDbPool,
    bigquery_client: Arc<BigQueryClient>,
    bigquery_project_id: String,
    bigquery_dataset_name: String,
}

impl DefaultTransactionProcessorBq {
    pub async fn new(
        connection_pool: PgDbPool,
        bigquery_project_id: String,
        bigquery_dataset_name: String,
    ) -> Self {
        let bigquery_client = Arc::new(
            BigQueryClient::new(bigquery_project_id.clone(), bigquery_dataset_name.clone()).await,
        );
        Self {
            connection_pool,
            bigquery_client,
            bigquery_project_id,
            bigquery_dataset_name,
        }
    }
}

impl Debug for DefaultTransactionProcessorBq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "DefaultTransactionProcessor {{ connections: {:?}  idle_connections: {:?} project id: {:?} dataset prefix: {:?} }}",
            state.connections, state.idle_connections, self.bigquery_project_id, self.bigquery_dataset_name
        )
    }
}

impl DefaultTransactionProcessorBq {
    fn get_transaction_bq(transaction: &TransactionModel) -> TransactionBQ {
        TransactionBQ {
            version: transaction.version,
            block_height: transaction.block_height,
            hash: transaction.hash.clone(),
            r#type: transaction.type_.clone(),
            payload: transaction
                .payload
                .map(|p| serde_json::to_string(&p).unwrap()),
            state_change_hash: transaction.state_change_hash.clone(),
            event_root_hash: transaction.event_root_hash.clone(),
            state_checkpoint_hash: transaction.state_checkpoint_hash.clone(),
            gas_used: bigdecimal_to_u64(&transaction.gas_used),
            success: transaction.success,
            vm_status: transaction.vm_status,
            accumulator_root_hash: transaction.accumulator_root_hash,
            num_events: transaction.num_events,
            num_write_set_changes: transaction.num_write_set_changes,
            epoch: transaction.epoch,
            inserted_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Current time is before UNIX_EPOCH")
                .as_micros() as i64,
        }
    }
}

#[async_trait]
impl ProcessorTrait for DefaultTransactionProcessorBq {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<ProcessingResult> {
        let (txns, txn_details, events, write_set_changes, wsc_details) =
            DefaultTransactionProcessor::get_models(transactions, start_version, end_version);
        let txn_models: Vec<Vec<u8>> = txns
            .iter()
            .map(|t| {
                let mut buf = Vec::new();
                let txn_bq = Self::get_transaction_bq(t);
                txn_bq.encode(&mut buf).unwrap();
                buf
            })
            .collect();
        let append_row_request = get_txn_request(txn_models);

        match self
            .bigquery_client
            .send_data(TypedAppendRowsRequest::Transactions(append_row_request))
            .await
        {
            Ok(_) => Ok((start_version, end_version)),
            Err(e) => {
                error!(
                    start_version = start_version,
                    end_version = end_version,
                    processor_name = self.name(),
                    error = ?e,
                    "[Parser] Error inserting transactions to db",
                );
                bail!(e)
            },
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
