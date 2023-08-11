// Copyright © Aptos Foundation

use crate::metrics::DATA_SERVICE_CHECKER_TRANSACTION_COUNT;
use anyhow::Result;
use aptos_indexer_grpc_utils::constants::GRPC_AUTH_TOKEN_HEADER;
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use futures::StreamExt;
use rand::Rng;

pub struct DataServiceChecker {
    pub indexer_grpc_address: String,
    pub indexer_grpc_auth_token: String,
    pub ledger_version: u64,
}

impl DataServiceChecker {
    pub fn new(
        indexer_grpc_address: String,
        indexer_grpc_auth_token: String,
        ledger_version: u64,
    ) -> Result<Self> {
        Ok(Self {
            indexer_grpc_address,
            indexer_grpc_auth_token,
            ledger_version,
        })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let mut client = RawDataClient::connect(self.indexer_grpc_address.clone()).await?;
        let starting_version = rand::thread_rng().gen_range(0, self.ledger_version);
        let mut request = tonic::Request::new(GetTransactionsRequest {
            starting_version: Some(starting_version),
            ..GetTransactionsRequest::default()
        });
        request.metadata_mut().insert(
            GRPC_AUTH_TOKEN_HEADER,
            tonic::metadata::MetadataValue::try_from(&self.indexer_grpc_auth_token)?,
        );

        let mut stream = client.get_transactions(request).await?.into_inner();
        while let Some(transaction) = stream.next().await {
            let transaction = transaction?;
            DATA_SERVICE_CHECKER_TRANSACTION_COUNT.inc_by(transaction.transactions.len() as u64);
        }
        Ok(())
    }
}
