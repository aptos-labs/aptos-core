// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::indexer_api::confirm_metadata_applied;
use anyhow::{anyhow, Context, Result};
use aptos_indexer_processor_sdk::{
    aptos_indexer_transaction_stream::{transaction_stream::get_chain_id, TransactionStreamConfig},
    postgres::processor_metadata_schema::processor_metadata::processor_status,
};
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use diesel_async::{pg::AsyncPgConnection, AsyncConnection, RunQueryDsl};
use reqwest::Url;
use serde::Serialize;
use std::time::Duration;
use tokio::time::Instant;
use tracing::info;

const MAX_WAIT_S: u64 = 120;
const WAIT_INTERVAL_MS: u64 = 200;

/// This provides a single place to define a variety of different healthchecks. In
/// cases where the name of the service being checked isn't obvious, the enum will take
/// a string arg that names it.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum HealthChecker {
    /// Check that a HTTP API is up. The second param is the name of the HTTP service.
    Http(Url, String),
    /// Check that the node API is up. This is just a specific case of Http for extra
    /// guarantees around liveliness.
    NodeApi(Url),
    /// Check that a data service GRPC stream is up.
    DataServiceGrpc(Url),
    /// Check that a postgres instance is up.
    Postgres(String),
    /// Check that a processor is successfully processing txns. The first value is the
    /// postgres connection string. The second is the name of the processor. We check
    /// the that last_success_version in the processor_status table is present and > 0.
    Processor(String, String),
    /// Check that the indexer API is up and the metadata has been applied. We only use
    /// this one in the ready server.
    IndexerApiMetadata(Url),
}

impl HealthChecker {
    pub async fn check(&self) -> Result<()> {
        match self {
            HealthChecker::Http(url, _) => {
                reqwest::get(Url::clone(url))
                    .await
                    .with_context(|| format!("Failed to GET {}", url))?;
                Ok(())
            },
            HealthChecker::NodeApi(url) => {
                aptos_rest_client::Client::new(Url::clone(url))
                    .get_index()
                    .await?;
                Ok(())
            },
            HealthChecker::DataServiceGrpc(url) => {
                let mut backoff = backoff::ExponentialBackoff::default();
                backoff.max_elapsed_time = Some(Duration::from_secs(5));
                backoff::future::retry(backoff, || async {
                    let transaction_stream_config = TransactionStreamConfig {
                        indexer_grpc_data_service_address: url.clone(),
                        auth_token: "notused".to_string(),
                        starting_version: Some(0),
                        request_ending_version: None,
                        request_name_header: "notused".to_string(),
                        additional_headers: Default::default(),
                        indexer_grpc_http2_ping_interval_secs: Default::default(),
                        indexer_grpc_http2_ping_timeout_secs: 60,
                        indexer_grpc_response_item_timeout_secs: 60,
                        indexer_grpc_reconnection_timeout_secs: 60,
                        indexer_grpc_reconnection_max_retries: Default::default(),
                        transaction_filter: None,
                    };
                    get_chain_id(transaction_stream_config)
                        .await
                        .context("Failed to get chain id")?;
                    Ok(())
                })
                .await
                .context("Failed to get a response from gRPC")?;
                Ok(())
            },
            HealthChecker::Postgres(connection_string) => {
                AsyncPgConnection::establish(connection_string)
                    .await
                    .context("Failed to connect to postgres to check DB liveness")?;
                Ok(())
            },
            HealthChecker::Processor(connection_string, processor_name) => {
                let mut connection = AsyncPgConnection::establish(connection_string)
                    .await
                    .context("Failed to connect to postgres to check processor status")?;
                let result = processor_status::table
                    .select((processor_status::last_success_version,))
                    .filter(processor_status::processor.eq(processor_name))
                    .first::<(i64,)>(&mut connection)
                    .await
                    .optional()
                    .context("Failed to look up processor status")?;
                match result {
                    Some(result) => {
                        // This is last_success_version.
                        if result.0 > 0 {
                            info!(
                                "Processor {} started processing successfully (currently at version {})",
                                processor_name, result.0
                            );
                            Ok(())
                        } else {
                            Err(anyhow!(
                                "Processor {} found in DB but last_success_version is zero",
                                processor_name
                            ))
                        }
                    },
                    None => Err(anyhow!(
                        "Processor {} has not processed any transactions",
                        processor_name
                    )),
                }
            },
            HealthChecker::IndexerApiMetadata(url) => {
                confirm_metadata_applied(url.clone()).await?;
                Ok(())
            },
        }
    }

    /// Wait up to MAX_WAIT_S seconds for a service to start up.
    pub async fn wait(
        &self,
        // The service, if any, waiting for this service to start up.
        waiting_service: Option<&str>,
    ) -> Result<()> {
        let prefix = self.to_string();
        wait_for_startup(|| self.check(), match waiting_service {
            Some(waiting_service) => {
                format!(
                    "{} at {} did not start up before {}",
                    prefix,
                    self.address_str(),
                    waiting_service,
                )
            },
            None => format!("{} at {} did not start up", prefix, self.address_str()),
        })
        .await
    }

    /// This is only ever used for display purposes. If possible, this should be the
    /// endpoint of the service that this HealthChecker is checking.
    pub fn address_str(&self) -> &str {
        match self {
            HealthChecker::Http(url, _) => url.as_str(),
            HealthChecker::NodeApi(url) => url.as_str(),
            HealthChecker::DataServiceGrpc(url) => url.as_str(),
            HealthChecker::Postgres(url) => url.as_str(),
            HealthChecker::Processor(_, processor_name) => processor_name.as_str(),
            HealthChecker::IndexerApiMetadata(url) => url.as_str(),
        }
    }

    /// Given a port, make an instance of HealthChecker::Http targeting 127.0.0.1.
    pub fn http_checker_from_port(port: u16, name: String) -> Self {
        Self::Http(
            Url::parse(&format!("http://127.0.0.1:{}", port,)).unwrap(),
            name,
        )
    }
}

impl std::fmt::Display for HealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthChecker::Http(_, name) => write!(f, "{}", name),
            HealthChecker::NodeApi(_) => write!(f, "Node API"),
            HealthChecker::DataServiceGrpc(_) => write!(f, "Transaction stream"),
            HealthChecker::Postgres(_) => write!(f, "Postgres"),
            HealthChecker::Processor(_, processor_name) => write!(f, "{}", processor_name),
            HealthChecker::IndexerApiMetadata(_) => write!(f, "Indexer API with metadata applied"),
        }
    }
}

async fn wait_for_startup<F, Fut>(check_fn: F, error_message: String) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: futures::Future<Output = Result<()>>,
{
    let max_wait = Duration::from_secs(MAX_WAIT_S);
    let wait_interval = Duration::from_millis(WAIT_INTERVAL_MS);

    let start = Instant::now();
    let mut started_successfully = false;

    let mut last_error_message = None;
    while start.elapsed() < max_wait {
        match check_fn().await {
            Ok(_) => {
                started_successfully = true;
                break;
            },
            Err(err) => {
                last_error_message = Some(format!("{:#}", err));
            },
        }
        tokio::time::sleep(wait_interval).await
    }

    if !started_successfully {
        let error_message = match last_error_message {
            Some(last_error_message) => format!("{}: {}", error_message, last_error_message),
            None => error_message,
        };
        return Err(anyhow!(error_message));
    }

    Ok(())
}
