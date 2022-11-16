// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    bigquery_client::{get_txn_request, BigQueryClient, TypedAppendRowsRequest},
    database::PgDbPool,
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::transactions::TransactionModel,
    util::bigdecimal_to_u64,
};
use aptos_api_types::Transaction as APITransaction;
use aptos_protos::indexer::transaction::v1::Transaction as TransactionProto;
use async_trait::async_trait;
use prost::Message;
use std::fmt::Debug;
use std::sync::Arc;

pub const NAME: &str = "default_processor_bq";

pub struct DefaultTransactionProcessorBq {
    connection_pool: PgDbPool,
    bigquery_client: Arc<BigQueryClient>,
    bigquery_project_id: String,
}

impl DefaultTransactionProcessorBq {
    pub async fn new(connection_pool: PgDbPool, bigquery_project_id: String) -> Self {
        let bigquery_client = Arc::new(BigQueryClient::new(bigquery_project_id.clone()).await);
        Self {
            connection_pool,
            bigquery_client,
            bigquery_project_id,
        }
    }
}

impl Debug for DefaultTransactionProcessorBq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "DefaultTransactionProcessor {{ connections: {:?}  idle_connections: {:?} project id: {:?} }}",
            state.connections, state.idle_connections, self.bigquery_project_id
        )
    }
}

fn get_transaction_proto(transaction: &TransactionModel) -> TransactionProto {
    TransactionProto {
        type_str: transaction.type_.clone(),
        payload: transaction
            .payload
            .as_ref()
            .map(|payload| payload.to_string()),
        version: transaction.version,
        block_height: transaction.block_height,
        hash: transaction.hash.clone(),
        state_change_hash: transaction.state_change_hash.clone(),
        event_root_hash: transaction.event_root_hash.clone(),
        state_checkpoint_hash: transaction.state_checkpoint_hash.clone(),
        gas_used: bigdecimal_to_u64(&transaction.gas_used),
        success: transaction.success,
        vm_status: transaction.vm_status.clone(),
        accumulator_root_hash: transaction.accumulator_root_hash.clone(),
        num_events: transaction.num_events,
        num_write_set_changes: transaction.num_write_set_changes,
        epoch: transaction.epoch,
        inserted_at: chrono::Utc::now().timestamp(),
    }
}

#[async_trait]
impl TransactionProcessor for DefaultTransactionProcessorBq {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<APITransaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let (txns, _, _, _, _) = TransactionModel::from_transactions(&transactions);
        let txn_protos: Vec<Vec<u8>> = txns
            .iter()
            .map(|txn| {
                let mut buf1 = Vec::new();
                let txn_proto = get_transaction_proto(txn);
                txn_proto.encode(&mut buf1).unwrap();
                buf1
            })
            .collect();

        let append_row_request = get_txn_request(txn_protos);

        match self
            .bigquery_client
            .send_data(TypedAppendRowsRequest::Transactions(append_row_request))
            .await
        {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::BigQueryTransactionCommitError(
                (
                    anyhow::Error::msg(format!("{:?}", err)),
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
