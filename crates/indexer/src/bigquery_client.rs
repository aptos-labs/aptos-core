// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! BigQuery-related functions
use crate::indexer::errors::TransactionProcessingError;
use crate::util::u64_to_bigdecimal_str;
use aptos_api_types::{Transaction as APITransaction, TransactionInfo};
use futures_util::io::Write;
use futures_util::stream;
use gcloud_sdk::google::cloud::bigquery::storage::v1::append_rows_request::{ProtoData, Rows};
use gcloud_sdk::google::cloud::bigquery::storage::v1::{
    write_stream, AppendRowsRequest, CreateWriteStreamRequest, ProtoRows, ProtoSchema, WriteStream,
};
use gcloud_sdk::{
    google::cloud::bigquery::storage::v1::big_query_write_client::BigQueryWriteClient, GoogleApi,
    GoogleAuthMiddleware,
};
use once_cell::sync::Lazy;
use prost::Message;
use prost_types::{DescriptorProto, FileDescriptorSet};
use std::collections::HashMap;
use std::sync::Mutex;
use streaming_proto::Transaction as TransactionProto;

pub const BIGQUERY_DATASET_NAME: &str = "aptos_indexer_bigquery_data";

pub mod streaming_proto {
    include!(concat!("./pb", "/aptos.indexer.proto.v1.rs"));
}

pub struct BigQueryClient {
    /// BigQuery client  that handles stream management and data flow.
    client: GoogleApi<BigQueryWriteClient<GoogleAuthMiddleware>>,
    /// Maps from table name to the corresponding stream id. If not present, create one.
    pub stream_map: HashMap<String, String>,
    /// BigQuery path to the data set.
    ///   Example: "projects/YOUR_PROJECT_ID/datasets/YOUR_DATASET/tables/"
    pub data_set_path: String,
}

impl BigQueryClient {
    /// Creates BigQuery client based on service account credential file(json).
    /// GOOGLE_APPLICATION_CREDENTIALS saves the absolute path pointing to the file.
    pub async fn new(project_id: String) -> Self {
        let client = GoogleApi::from_function(
            BigQueryWriteClient::new,
            "https://bigquerystorage.googleapis.com",
            None,
        )
        .await
        .expect("Create raw Bigquery Client successfully");
        let data_set_path = format!(
            "projects/{}/datasets/{}/tables/",
            project_id, BIGQUERY_DATASET_NAME
        );
        Self {
            client,
            stream_map: HashMap::new(),
            data_set_path,
        }
    }
    /// Returns the default stream for the data; if not present, create one.
    pub async fn get_stream(&self, table: String) -> String {
        match self.stream_map.get(&table) {
            Some(stream_id) => stream_id.to_owned(),
            None => {
                let write_stream_resp = self
                    .client
                    .get()
                    .create_write_stream(tonic::Request::new(CreateWriteStreamRequest {
                        parent: format!("{}{}", self.data_set_path, table),
                        write_stream: Some(WriteStream {
                            r#type: i32::from(write_stream::Type::Committed),
                            ..WriteStream::default()
                        }),
                    }))
                    .await
                    .expect("Create stream successfully.");
                write_stream_resp.into_inner().name
            }
        }
    }

    /// Sends the data.
    pub async fn send_data(
        &self,
        mut append_row_req: AppendRowsRequest,
    ) -> Result<(), TransactionProcessingError> {
        // TODO(laliu): fix this.
        append_row_req.write_stream = self.get_stream("transactions".to_string()).await;

        match self
            .client
            .get()
            .append_rows(tonic::Request::new(stream::iter(vec![append_row_req])))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => Err(TransactionProcessingError::BigQueryTransactionCommitError(
                (anyhow::Error::msg("123"), 0, 1, "testing"),
            )),
        }
    }
}

/// The Protobuf descriptor for Transaction.
static TRANSACTION_DESCRIPTOR: Lazy<DescriptorProto> = Lazy::new(|| {
    let node_set = FileDescriptorSet::decode(&streaming_proto::FILE_DESCRIPTOR_SET[..])
        .expect("Decode Proto successfully.");
    node_set
        .file
        // First one in file set.
        .get(0)
        .unwrap()
        .message_type
        // First type defined in this file.
        .get(0)
        .unwrap()
        .clone()
});

// Deprecated; separate this out from bigquery client module.
fn from_transaction_info(
    info: &TransactionInfo,
    // Serialized Json string.
    payload: Option<String>,
    type_: String,
    num_events: i64,
    block_height: i64,
    epoch: i64,
) -> TransactionProto {
    TransactionProto {
        type_str: type_,
        payload,
        version: info.version.0 as i64,
        block_height,
        hash: info.hash.to_string().as_bytes().to_vec(),
        state_change_hash: info.state_change_hash.to_string().as_bytes().to_vec(),
        event_root_hash: info.event_root_hash.to_string().as_bytes().to_vec(),
        state_checkpoint_hash: match info.state_checkpoint_hash.map(|h| h.to_string()) {
            Some(hash) => Some(hash.as_bytes().to_vec()),
            None => None,
        },
        gas_used: u64_to_bigdecimal_str(info.gas_used.0),
        success: info.success,
        vm_status: info.vm_status.clone(),
        accumulator_root_hash: info.accumulator_root_hash.to_string().as_bytes().to_vec(),
        num_events,
        num_write_set_changes: info.changes.len() as i64,
        epoch,
    }
}

pub fn extract_from_api_transactions(transactions: &[APITransaction]) -> AppendRowsRequest {
    let a = transactions
        .iter()
        .map(|transaction| {
            let block_height = transaction
                .transaction_info()
                .unwrap()
                .block_height
                .unwrap()
                .0 as i64;
            let epoch = transaction.transaction_info().unwrap().epoch.unwrap().0 as i64;
            let transaction_proto = match transaction {
                APITransaction::UserTransaction(user_txn) => from_transaction_info(
                    &user_txn.info,
                    Some(
                        serde_json::to_string(&user_txn.request.payload)
                            .expect("Unable to deserialize transaction payload"),
                    ),
                    transaction.type_str().to_string(),
                    user_txn.events.len() as i64,
                    block_height,
                    epoch,
                ),
                APITransaction::GenesisTransaction(genesis_txn) => from_transaction_info(
                    &genesis_txn.info,
                    Some(
                        serde_json::to_string(&genesis_txn.payload)
                            .expect("Unable to deserialize Genesis transaction"),
                    ),
                    transaction.type_str().to_string(),
                    0,
                    block_height,
                    epoch,
                ),
                APITransaction::BlockMetadataTransaction(block_metadata_txn) => {
                    from_transaction_info(
                        &block_metadata_txn.info,
                        None,
                        transaction.type_str().to_string(),
                        0,
                        block_height,
                        epoch,
                    )
                }
                APITransaction::StateCheckpointTransaction(state_checkpoint_txn) => {
                    from_transaction_info(
                        &state_checkpoint_txn.info,
                        None,
                        transaction.type_str().to_string(),
                        0,
                        block_height,
                        epoch,
                    )
                }
                APITransaction::PendingTransaction(..) => {
                    unreachable!()
                }
            };
            let mut buf1 = Vec::new();
            transaction_proto.encode(&mut buf1);
            buf1
        })
        .collect();
    AppendRowsRequest {
        offset: None,
        trace_id: String::new(),
        rows: Some(Rows::ProtoRows(ProtoData {
            rows: Some(ProtoRows { serialized_rows: a }),
            writer_schema: Some(ProtoSchema {
                proto_descriptor: Some((*TRANSACTION_DESCRIPTOR).clone()),
            }),
        })),
        ..AppendRowsRequest::default()
    }
}
