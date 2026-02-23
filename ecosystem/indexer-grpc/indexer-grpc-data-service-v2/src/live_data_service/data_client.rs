// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::connection_manager::ConnectionManager;
use aptos_protos::{indexer::v1::GetTransactionsRequest, transaction::v1::Transaction};
use std::sync::Arc;
use tracing::{info_span, trace, Instrument};

pub(super) struct DataClient {
    connection_manager: Arc<ConnectionManager>,
}

impl DataClient {
    pub(super) fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    pub(super) async fn fetch_transactions(&self, starting_version: u64) -> Vec<Transaction> {
        let span = info_span!(
            "data_client.fetch_transactions",
            otel.kind = "client",
            starting_version = starting_version,
        );

        async {
            trace!("Fetching transactions from GrpcManager, start_version: {starting_version}.");

            let request = GetTransactionsRequest {
                starting_version: Some(starting_version),
                transactions_count: None,
                batch_size: None,
                transaction_filter: None,
            };
            loop {
                let mut client = self
                    .connection_manager
                    .get_grpc_manager_client_for_request();
                let response = client.get_transactions(request.clone()).await;
                if let Ok(response) = response {
                    let transactions = response.into_inner().transactions;
                    if transactions.is_empty() {
                        return vec![];
                    }
                    if transactions.first().unwrap().version == starting_version {
                        return transactions;
                    }
                }
                // TODO(grao): Error handling.
            }
        }
        .instrument(span)
        .await
    }
}
