// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::PgDbPool,
    indexer::{
        errors::TransactionProcessingError, fetcher::TransactionFetcher,
        processing_result::ProcessingResult, transaction_processor::TransactionProcessor,
    },
};
use aptos_logger::info;
use aptos_rest_client::Transaction;
use std::{fmt::Debug, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};
use url::{ParseError, Url};

diesel_migrations::embed_migrations!();

#[derive(Clone)]
pub struct Tailer {
    transaction_fetcher: Arc<Mutex<TransactionFetcher>>,
    processors: Vec<Arc<dyn TransactionProcessor>>,
    connection_pool: PgDbPool,
}

impl Tailer {
    pub fn new(node_url: &str, connection_pool: PgDbPool) -> Result<Tailer, ParseError> {
        let url = Url::parse(node_url)?;
        let transaction_fetcher = TransactionFetcher::new(url, None);
        Ok(Self {
            transaction_fetcher: Arc::new(Mutex::new(transaction_fetcher)),
            processors: vec![],
            connection_pool,
        })
    }

    pub fn run_migrations(&self) {
        info!("Running migrations...");
        embedded_migrations::run_with_output(
            &self
                .connection_pool
                .get()
                .expect("Could not get connection for migrations"),
            &mut std::io::stdout(),
        )
        .expect("migrations failed!");
        info!("Migrations complete!");
    }

    pub fn add_processor(&mut self, processor: Arc<dyn TransactionProcessor>) {
        info!("Adding processor to indexer: {}", processor.name());
        self.processors.push(processor);
    }

    /// For all versions which have an `success=false` in the `processor_status` table, re-run them
    pub async fn handle_previous_errors(&self) {
        info!("Checking for previously errored versions...");
        let mut tasks = vec![];
        for processor in &self.processors {
            let processor2 = processor.clone();
            let self2 = self.clone();
            let task = tokio::task::spawn(async move {
                let errored_versions = processor2.get_error_versions();
                let err_count = errored_versions.len();
                info!(
                    "Found {} previously errored versions for {}",
                    err_count,
                    processor2.name(),
                );
                if err_count == 0 {
                    return;
                }
                let mut fixed = 0;
                for version in errored_versions {
                    let txn = self2.get_txn(version).await;
                    if processor2
                        .process_transaction_with_status(txn)
                        .await
                        .is_ok()
                    {
                        fixed += 1;
                    };
                }
                info!(
                    "Fixed {}/{} previously errored versions for {}",
                    fixed,
                    err_count,
                    processor2.name(),
                );
            });
            tasks.push(task);
        }
        await_tasks(tasks).await;
        info!("Fixing previously errored versions complete!");
    }

    /// Sets the version of the fetcher to the lowest version among all processors
    pub async fn set_fetcher_to_lowest_processor_version(&self) -> u64 {
        let mut lowest = u64::MAX;
        for processor in &self.processors {
            let max_version = processor.get_max_version().unwrap_or_default();
            aptos_logger::debug!(
                "Processor {} max version is {}",
                processor.name(),
                max_version
            );
            if max_version < lowest {
                lowest = max_version;
            }
        }
        aptos_logger::info!("Lowest version amongst all processors is {}", lowest);
        self.set_fetcher_version(lowest).await;
        lowest
    }

    pub async fn set_fetcher_version(&self, version: u64) -> u64 {
        self.transaction_fetcher.lock().await.set_version(version);
        aptos_logger::info!("Will start fetching from version {}", version);
        version
    }

    pub async fn process_next(
        &mut self,
    ) -> anyhow::Result<Vec<Result<ProcessingResult, TransactionProcessingError>>> {
        let txn = self.get_next_txn().await;
        self.process_transaction(txn).await
    }

    pub async fn process_version(
        &mut self,
        version: u64,
    ) -> anyhow::Result<Vec<Result<ProcessingResult, TransactionProcessingError>>> {
        let txn = self.get_txn(version).await;
        self.process_transaction(txn).await
    }

    pub async fn process_next_batch(
        &mut self,
        batch_size: u8,
    ) -> Vec<anyhow::Result<Vec<Result<ProcessingResult, TransactionProcessingError>>>> {
        let mut tasks = vec![];
        for _ in 0..batch_size {
            let mut self2 = self.clone();
            let task = tokio::task::spawn(async move { self2.process_next().await });
            tasks.push(task);
        }
        let results = await_tasks(tasks).await;
        results
    }

    pub async fn process_transaction(
        &self,
        txn: Arc<Transaction>,
    ) -> anyhow::Result<Vec<Result<ProcessingResult, TransactionProcessingError>>> {
        let mut tasks = vec![];
        for processor in &self.processors {
            let txn2 = txn.clone();
            let processor2 = processor.clone();
            let task = tokio::task::spawn(async move {
                processor2.process_transaction_with_status(txn2).await
            });
            tasks.push(task);
        }
        let results = await_tasks(tasks).await;
        Ok(results)
    }

    pub async fn get_next_txn(&mut self) -> Arc<Transaction> {
        Arc::new(self.transaction_fetcher.lock().await.fetch_next().await)
    }

    pub async fn get_txn(&self, version: u64) -> Arc<Transaction> {
        Arc::new(
            self.transaction_fetcher
                .lock()
                .await
                .fetch_version(version)
                .await,
        )
    }
}

pub async fn await_tasks<T: Debug>(tasks: Vec<JoinHandle<T>>) -> Vec<T> {
    let mut results = vec![];
    for task in tasks {
        let result = task.await;
        if result.is_err() {
            aptos_logger::error!("Error joining task: {:?}", &result);
        }
        results.push(result.unwrap());
    }
    results
}
