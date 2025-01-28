// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::connection_manager::ConnectionManager;
use aptos_protos::{indexer::v1::GetTransactionsRequest, transaction::v1::Transaction};
use std::sync::Arc;
use tracing::trace;

pub(super) struct DataClient {
    connection_manager: Arc<ConnectionManager>,
}

impl DataClient {
    pub(super) fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    pub(super) async fn fetch_transactions(&self, starting_version: u64) -> Vec<Transaction> {
        trace!("Fetching transactions from GrpcManager, start_version: {starting_version}.");

        let request = GetTransactionsRequest {
            starting_version: Some(starting_version),
            transactions_count: None,
            batch_size: None,
        };
        loop {
            let mut client = self
                .connection_manager
                .get_grpc_manager_client_for_request();
            let response = client.get_transactions(request).await;
            if let Ok(response) = response {
                return response.into_inner().transactions;
            }
            // TODO(grao): Error handling.
        }
    }
}
