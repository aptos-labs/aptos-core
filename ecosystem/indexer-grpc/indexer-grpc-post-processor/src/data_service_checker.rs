// Copyright © Aptos Foundation

use crate::metrics::{DATA_SERVICE_CHECKER_TRANSACTION_COUNT, DATA_SERVICE_CHECKER_TRANSACTION_TPS};
use anyhow::Result;
use aptos_indexer_grpc_utils::constants::GRPC_AUTH_TOKEN_HEADER;
use aptos_protos::indexer::v1::{raw_data_client::RawDataClient, GetTransactionsRequest};
use futures::StreamExt;
use rand::Rng;
use aptos_moving_average::MovingAverage;
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
        let mut ma = MovingAverage::new(100_000);
        while let Some(transaction) = stream.next().await {
            let transaction = transaction?;
            let num_res = transaction.transactions.len();
            DATA_SERVICE_CHECKER_TRANSACTION_COUNT.inc_by(num_res as u64);
            ma.tick_now(num_res as u64);
            DATA_SERVICE_CHECKER_TRANSACTION_TPS.set(ma.avg() * 1000.0);
        }
        Ok(())
    }
}
