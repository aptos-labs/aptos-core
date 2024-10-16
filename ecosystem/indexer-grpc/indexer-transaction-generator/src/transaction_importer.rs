// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::TransactionImporterPerNetworkConfig;
use anyhow::Context;
use aptos_indexer_grpc_utils::create_data_service_grpc_client;
use aptos_protos::indexer::v1::GetTransactionsRequest;
use std::{path::Path, time::Duration};

/// GRPC request metadata key for the token ID.
const GRPC_API_GATEWAY_API_KEY_HEADER: &str = "authorization";
const GRPC_REQUEST_NAME_HEADER: &str = "x-request-name";
const GRPC_REQUEST_NAME_VALUE: &str = "testing-framework";
const TRANSACTION_STREAM_TIMEOUT_IN_SECS: u64 = 60;

impl TransactionImporterPerNetworkConfig {
    pub async fn run(&self, output_path: &Path) -> anyhow::Result<()> {
        let mut client = create_data_service_grpc_client(
            self.transaction_stream_endpoint.clone(),
            Some(Duration::from_secs(TRANSACTION_STREAM_TIMEOUT_IN_SECS)),
        )
        .await?;

        for (version, output_file) in &self.versions_to_import {
            let mut request = tonic::Request::new(GetTransactionsRequest {
                starting_version: Some(*version),
                transactions_count: Some(1),
                ..GetTransactionsRequest::default()
            });
            request.metadata_mut().insert(
                GRPC_REQUEST_NAME_HEADER,
                GRPC_REQUEST_NAME_VALUE.parse().unwrap(),
            );
            if let Some(api_key) = &self.api_key {
                request.metadata_mut().insert(
                    GRPC_API_GATEWAY_API_KEY_HEADER,
                    format!("Bearer {}", api_key.clone()).parse().unwrap(),
                );
            }
            let mut stream = client.get_transactions(request).await?.into_inner();
            while let Some(resp) = stream.message().await.context(format!(
                "[Transaction Importer] Stream ended unexpected for endpoint {:?}",
                self.transaction_stream_endpoint
            ))? {
                let transaction = resp.transactions.first().context(format!(
                    "[Transaction Importer] Transaction at version {} is not in response.",
                    version
                ))?;
                let json_string = serde_json::to_string_pretty(transaction).context(
                    format!("[Transaction Importer] Transaction at version {} failed to serialized to json string.", version))?;
                let output_path = output_path.join(output_file).with_extension("json");
                // TODO: add a diffing process here.
                tokio::fs::write(output_path, json_string)
                    .await
                    .context(format!(
                        "[Transaction Importer] Failed to write transaction at version {} to file.",
                        version
                    ))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::config::TransactionImporterPerNetworkConfig;
    use aptos_protos::{
        indexer::v1::{
            raw_data_server::{RawData, RawDataServer},
            GetTransactionsRequest, TransactionsResponse,
        },
        transaction::v1::Transaction,
    };
    use futures::Stream;
    use std::pin::Pin;
    use tonic::{Request, Response, Status};

    type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

    #[derive(Debug, Default)]
    pub struct DummyServer {
        pub transactions: Vec<TransactionsResponse>,
    }

    #[tonic::async_trait]
    impl RawData for DummyServer {
        type GetTransactionsStream = ResponseStream;

        async fn get_transactions(
            &self,
            req: Request<GetTransactionsRequest>,
        ) -> Result<Response<Self::GetTransactionsStream>, Status> {
            let version = req.into_inner().starting_version.unwrap();
            let transaction = self
                .transactions
                .iter()
                .find(|t| t.transactions.first().unwrap().version == version)
                .unwrap();
            let stream = futures::stream::iter(vec![Ok(transaction.clone())]);
            Ok(Response::new(Box::pin(stream)))
        }
    }

    #[tokio::test]
    async fn test_run() {
        // Create a dummy transaction server.
        let transaction = Transaction {
            version: 1,
            ..Transaction::default()
        };
        let transactions = vec![TransactionsResponse {
            transactions: vec![transaction],
            ..TransactionsResponse::default()
        }];
        let server = DummyServer { transactions };
        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(RawDataServer::new(server))
                .serve("127.0.0.1:51254".parse().unwrap())
                .await
                .unwrap();
        });
        // Note: do not sleep here; client connection will be retried.

        // create temp dir
        let temp_dir = tempfile::tempdir().unwrap();

        let config_json = r#"
            transaction_stream_endpoint: "http://localhost:51254"
            versions_to_import:
                1: "testing_transaction"
        "#;

        let config =
            serde_yaml::from_str::<TransactionImporterPerNetworkConfig>(config_json).unwrap();
        config.run(temp_dir.path()).await.unwrap();

        // Validate the output.
        let output_path = temp_dir.path().join("testing_transaction.json");
        let output = tokio::fs::read_to_string(output_path).await.unwrap();
        let transaction = serde_json::from_str::<Transaction>(&output).unwrap();
        assert_eq!(transaction.version, 1);
    }
}
