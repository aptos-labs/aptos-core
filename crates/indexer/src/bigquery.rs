// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! BigQuery-related functions
use crate::util::u64_to_bigdecimal_str;
use aptos_api_types::{Transaction as APITransaction, TransactionInfo};
use gcloud_sdk::google::cloud::bigquery::storage::v1::append_rows_request::{ProtoData, Rows};
use gcloud_sdk::google::cloud::bigquery::storage::v1::{
    AppendRowsRequest, CreateWriteStreamRequest, ProtoRows, ProtoSchema,
};
use gcloud_sdk::{
    google::cloud::bigquery::storage::v1::big_query_write_client::BigQueryWriteClient, GoogleApi,
    GoogleAuthMiddleware,
};
use once_cell::sync::Lazy;
use prost::Message;
use prost_types::{DescriptorProto, FileDescriptorSet};
use streaming_proto::Transaction as TransactionProto;

pub mod streaming_proto {
    include!(concat!("./pb", "/aptos.indexer.streaming.proto.v1.rs"));
}
pub type BigQueryClient = GoogleApi<BigQueryWriteClient<GoogleAuthMiddleware>>;

/// The Protobuf descriptor for Transaction.
static TRANSACTION_DESCRIPTOR: Lazy<DescriptorProto> = Lazy::new(|| {
    let node_set = FileDescriptorSet::decode(&streaming_proto::FILE_DESCRIPTOR_SET[..])
        .expect("Decode Proto successfully.");
    let msg_type = node_set
        .file
        // First one in file set.
        .get(0)
        .unwrap()
        .message_type
        // First type defined in this file.
        .get(0)
        .unwrap()
        .clone();
    msg_type
});

/// Creates BigQuery client based on service account credential(json).
/// GOOGLE_APPLICATION_CREDENTIALS saves the absolute path pointing to the file.
pub async fn create_bigquery_client(
    cloud_resource_prefix: String,
) -> (GoogleApi<BigQueryWriteClient<GoogleAuthMiddleware>>, String) {
    let client = GoogleApi::from_function(
        BigQueryWriteClient::new,
        "https://bigquerystorage.googleapis.com",
        Some(cloud_resource_prefix),
    )
    .await
    .expect("Create Bigquery Client successfully");

    let write_stream_resp = client
        .get()
        .create_write_stream(tonic::Request::new(CreateWriteStreamRequest {
            ..CreateWriteStreamRequest::default()
        }))
        .await
        .expect("Create stream successfully.");
    (client, write_stream_resp.into_inner().name)
}

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
