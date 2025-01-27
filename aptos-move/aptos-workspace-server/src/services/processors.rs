// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{node::get_data_service_url, postgres::get_postgres_connection_string};
use crate::{
    common::{make_shared, ArcError},
    no_panic_println,
};
use anyhow::{anyhow, Context, Result};
use aptos_localnet::{health_checker::HealthChecker, processors::get_processor_config};
use diesel::Connection;
use diesel_async::{async_connection_wrapper::AsyncConnectionWrapper, pg::AsyncPgConnection};
use futures::{future::try_join_all, stream::FuturesUnordered, StreamExt, TryFutureExt};
use processor::{
    gap_detectors::DEFAULT_GAP_DETECTION_BATCH_SIZE, processors::ProcessorName,
    utils::database::run_pending_migrations, IndexerGrpcProcessorConfig,
};
use server_framework::RunnableConfig;
use std::future::Future;
use tokio::try_join;

/// Names of the processors to enable in the local network.
const PROCESSOR_NAMES: &[ProcessorName] = {
    use ProcessorName::*;

    &[
        AccountTransactionsProcessor,
        DefaultProcessor,
        EventsProcessor,
        FungibleAssetProcessor,
        ObjectsProcessor,
        StakeProcessor,
        TokenV2Processor,
        TransactionMetadataProcessor,
        UserTransactionProcessor,
    ]
};

/// Starts a single processor.
///
/// Needs to await a task to bring up the prerequisite services and perform the DB migration,
/// shared among all processors.
///
/// The function returns two futures:
/// - One that resolves when the processor is up.
/// - One that resolves when the processor stops (which it should not under normal operation).
fn start_processor(
    fut_prerequisites: &(impl Future<Output = Result<(u16, u16), ArcError>> + Clone + Send + 'static),
    processor_name: &ProcessorName,
) -> (
    impl Future<Output = Result<()>>,
    impl Future<Output = Result<()>>,
) {
    let fut_prerequisites_ = fut_prerequisites.clone();
    let processor_name_ = processor_name.to_owned();
    let handle_processor = tokio::spawn(async move {
        let (postgres_port, indexer_grpc_port) = fut_prerequisites_.await?;

        no_panic_println!("Starting processor {}..", processor_name_);

        let config = IndexerGrpcProcessorConfig {
            processor_config: get_processor_config(&processor_name_)?,
            postgres_connection_string: get_postgres_connection_string(postgres_port),
            indexer_grpc_data_service_address: get_data_service_url(indexer_grpc_port),

            auth_token: "notused".to_string(),
            grpc_http2_config: Default::default(),
            starting_version: None,
            ending_version: None,
            number_concurrent_processing_tasks: None,
            enable_verbose_logging: None,
            // The default at the time of writing is 30 but we don't need that
            // many in a localnet environment.
            db_pool_size: Some(8),
            gap_detection_batch_size: 50,
            pb_channel_txn_chunk_size: 100_000,
            per_table_chunk_sizes: Default::default(),
            transaction_filter: Default::default(),
            grpc_response_item_timeout_in_secs: 10,
            deprecated_tables: Default::default(),
            parquet_gap_detection_batch_size: DEFAULT_GAP_DETECTION_BATCH_SIZE,
        };

        config.run().await
    });

    let fut_processor_finish = async move {
        handle_processor
            .await
            .map_err(|err| anyhow!("failed to join task handle: {}", err))?
    };

    let fut_prerequisites_ = fut_prerequisites.clone();
    let processor_name_ = processor_name.to_owned();
    let fut_processor_ready = async move {
        let (postgres_port, _indexer_grpc_port) = fut_prerequisites_.await?;

        let processor_health_checker = HealthChecker::Processor(
            get_postgres_connection_string(postgres_port),
            processor_name_.to_string(),
        );

        processor_health_checker.wait(None).await?;

        no_panic_println!("Processor {} is ready.", processor_name_);

        Ok(())
    };

    (fut_processor_ready, fut_processor_finish)
}

/// Starts the indexer processor services. See [`PROCESSOR_NAMES`] for the full list.
///
/// Prerequisites
/// - Node API
/// - Node indexer gRPC
/// - Postgres DB
///
/// The function returns two futures:
/// - One that resolves when all processors are up.
/// - One that resolves when any of the processors stops (which it should not under normal operation).
pub fn start_all_processors(
    fut_node_api: impl Future<Output = Result<u16, ArcError>> + Clone + Send + 'static,
    fut_indexer_grpc: impl Future<Output = Result<u16, ArcError>> + Clone + Send + 'static,
    fut_postgres: impl Future<Output = Result<u16, ArcError>> + Clone + Send + 'static,
) -> (
    impl Future<Output = Result<()>>,
    impl Future<Output = Result<()>>,
) {
    let fut_migration = async move {
        let postgres_port = fut_postgres
            .await
            .context("failed to run migration: postgres did not start successfully")?;

        no_panic_println!("Starting migration..");

        let connection_string = get_postgres_connection_string(postgres_port);

        tokio::task::spawn_blocking(move || {
            // This lets us use the connection like a normal diesel connection. See more:
            // https://docs.rs/diesel-async/latest/diesel_async/async_connection_wrapper/type.AsyncConnectionWrapper.html
            let mut conn: AsyncConnectionWrapper<AsyncPgConnection> =
                AsyncConnectionWrapper::establish(&connection_string).with_context(|| {
                    format!("Failed to connect to postgres at {}", connection_string)
                })?;
            run_pending_migrations(&mut conn);
            anyhow::Ok(())
        })
        .await
        .map_err(|err| anyhow!("failed to join task handle: {}", err))??;

        no_panic_println!("Migration done.");

        Ok(postgres_port)
    };

    let fut_prerequisites = make_shared(async move {
        let (_node_api_port, indexer_grpc_port, postgres_port) = try_join!(
            fut_node_api.map_err(|err| anyhow!(err)),
            fut_indexer_grpc.map_err(|err| anyhow!(err)),
            fut_migration
        )
        .context(
            "failed to start processors: one or more prerequisites did not start successfully",
        )?;

        Ok((postgres_port, indexer_grpc_port))
    });

    let mut futs_ready = vec![];
    let mut futs_finish = vec![];

    for processor_name in PROCESSOR_NAMES {
        let (fut_ready, fut_finish) = start_processor(&fut_prerequisites, processor_name);

        futs_ready.push(fut_ready);
        futs_finish.push(fut_finish);
    }

    let fut_all_processors_ready = async move {
        try_join_all(futs_ready)
            .await
            .map_err(|err| err.context("one or more processors did not start successfully"))?;
        Ok(())
    };

    let fut_any_processor_finish = async move {
        let mut futs: FuturesUnordered<_> = futs_finish.into_iter().collect();
        futs.next().await.expect("there must be at least 1 future")
    };

    (fut_all_processors_ready, fut_any_processor_finish)
}
