// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! BigQuery-related functions
use anyhow::bail;
use aptos_protos::indexer::transaction::v1::FILE_DESCRIPTOR_SET;
use futures_util::stream;
use gcloud_sdk::google::cloud::bigquery::storage::v1::{
    append_rows_request::{ProtoData, Rows},
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
use tokio::sync::Mutex;

const BIGQUERY_DATASET_NAME: &str = "aptos_indexer_bigquery_data";

/// Typed AppendRowsRequest, which facilitates the table name(data destination) resolution.
pub enum TypedAppendRowsRequest {
    Transactions(AppendRowsRequest),
}

/// Thread-safe BigQuery client, which manages single stream for each table.
pub struct BigQueryClient {
    /// BigQuery client  that handles stream management and data flow.
    client: GoogleApi<BigQueryWriteClient<GoogleAuthMiddleware>>,
    /// Maps from table/resource name to the corresponding stream id. If not present, create one.
    pub stream_map: Mutex<HashMap<String, String>>,
    /// BigQuery path to the data set.
    ///   Example: "projects/YOUR_PROJECT_ID/datasets/YOUR_DATASET/tables/"
    pub data_set_path: String,
    /// TODO(laliu): move this to an event-driven approach. This is to avoid concurrent operations
    /// for the stream.
    pub sender_counter: Mutex<u64>,
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
            stream_map: Mutex::new(HashMap::new()),
            data_set_path,
            sender_counter: Mutex::new(0),
        }
    }
    /// Returns the default stream for the data; if not present, create one.
    async fn get_stream(&self, table: String) -> String {
        match self.stream_map.lock().await.get(&table) {
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

    /// Sends the data to bigquery streaming api.
    pub async fn send_data(&self, append_row_req: TypedAppendRowsRequest) -> anyhow::Result<()> {
        let req = match append_row_req {
            TypedAppendRowsRequest::Transactions(mut data) => {
                data.write_stream = self.get_stream("transactions".to_string()).await;
                data
            }
        };
        // let mut current = self.sender_counter.lock().await;
        match self
            .client
            .get()
            .append_rows(tonic::Request::new(stream::iter(vec![req])))
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => bail!("Failed to send data to BigQuery."),
        }
    }
}

/// The Protobuf descriptor for Transaction.
static TRANSACTION_DESCRIPTOR: Lazy<DescriptorProto> = Lazy::new(|| {
    let node_set =
        FileDescriptorSet::decode(FILE_DESCRIPTOR_SET).expect("Decode Proto successfully.");
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

pub fn get_request(protos: Vec<Vec<u8>>) -> AppendRowsRequest {
    AppendRowsRequest {
        offset: None,
        trace_id: String::new(),
        rows: Some(Rows::ProtoRows(ProtoData {
            rows: Some(ProtoRows {
                serialized_rows: protos,
            }),
            writer_schema: Some(ProtoSchema {
                proto_descriptor: Some((*TRANSACTION_DESCRIPTOR).clone()),
            }),
        })),
        ..AppendRowsRequest::default()
    }
}
