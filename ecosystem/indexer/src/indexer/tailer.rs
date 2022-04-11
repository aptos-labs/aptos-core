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

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::{new_db_pool, PgPoolConnection};
    use crate::default_processor::DefaultTransactionProcessor;
    use crate::models::transactions::TransactionModel;
    use diesel::Connection;
    use serde_json::json;

    pub fn wipe_database(conn: &PgPoolConnection) {
        for table in [
            "events",
            "user_transactions",
            "block_metadata_transactions",
            "transactions",
            "processor_statuses",
            "__diesel_schema_migrations",
        ] {
            conn.execute(&format!("DROP TABLE IF EXISTS {}", table))
                .unwrap();
        }
    }

    pub fn setup_indexer() -> anyhow::Result<(PgDbPool, Tailer)> {
        let database_url = std::env::var("INDEXER_DATABASE_URL")
            .expect("must set 'INDEXER_DATABASE_URL' to run tests!");
        let conn_pool = new_db_pool(database_url.as_str())?;
        wipe_database(&conn_pool.get()?);

        let mut tailer = Tailer::new("http://fake-url.aptos.dev", conn_pool.clone())?;
        tailer.run_migrations();

        let pg_transaction_processor = DefaultTransactionProcessor::new(conn_pool.clone());
        tailer.add_processor(Arc::new(pg_transaction_processor));
        Ok((conn_pool, tailer))
    }

    #[tokio::test]
    async fn test_parsing_and_writing() {
        let (conn_pool, tailer) = setup_indexer().unwrap();
        // An abridged genesis transaction
        let genesis_txn: Transaction = serde_json::from_value(json!(
            {
               "type":"genesis_transaction",
               "version":"0",
               "hash":"0x12180a4bbccf48de4d1e23b498add134328669ffc7741c8d529c6b2e3629ac99",
               "state_root_hash":"0xb50adef3662d77e528be9e1cb5637fe5b7afd13eea317b330799f0c559c918c1",
               "event_root_hash":"0xcbdbb1b830d1016d45a828bb3171ea81826e8315f14140acfbd7886f49fbcb40",
               "gas_used":"0",
               "success":true,
               "vm_status":"Executed successfully",
               "accumulator_root_hash":"0x188ed588547d551e652f04fccd5434c2977d6cff9e7443eb8e7c3038408caad4",
               "payload":{
                  "type":"write_set_payload",
                  "write_set":{
                     "type":"direct_write_set",
                     "changes":[],
                     "events":[]
                  }
               },
               "events":[
                  {
                     "key":"0x0400000000000000000000000000000000000000000000000000000000000000000000000a550c18",
                     "sequence_number":"0",
                     "type":"0x1::Reconfiguration::NewEpochEvent",
                     "data":{
                        "epoch":"1"
                     }
                  }
               ]
            })).unwrap();

        tailer
            .process_transaction(Arc::new(genesis_txn.clone()))
            .await
            .unwrap();

        // A block_metadata_transaction
        let block_metadata_transaction: Transaction = serde_json::from_value(json!(
            {
               "type":"block_metadata_transaction",
               "version":"69158",
               "hash":"0x2b7c58ed8524d228f9d0543a82e2793d04e8871df322f976b0e7bb8c5ced4ff5",
               "state_root_hash":"0x3ead9eb40582fbc7df5e02f72280931dc3e6f1aae45dc832966b4cd972dac4b8",
               "event_root_hash":"0x2e481956dea9c59b6fc9f823fe5f4c45efce173e42c551c1fe073b5d76a65504",
               "gas_used":"0",
               "success":true,
               "vm_status":"Executed successfully",
               "accumulator_root_hash":"0xb0ad602f805eb20c398f0f29a3504a9ef38bcc52c9c451deb9ec4a2d18807b49",
               "id":"0xeef99391a3fc681f16963a6c03415bc0b1b12b56c00429308fa8bf46ac9eddf0",
               "round":"57600",
               "previous_block_votes":[
                  "0x992da26d46e6d515a070c7f6e52376a1e674e850cb4d116babc6f870da9c258",
                  "0xfb4d785594a018bd980b4a20556d120c53a3f50b1cff9d5aa2e26eee582a587",
                  "0x2b7bce01a6f55e4a863c4822b154021a25588250c762ee01169b6208d6169208",
                  "0x43a2c4cefc4725e710dadf423dd9142057208e640c623b27c6bba704380825ab",
                  "0x4c91f3949924e988144550ece1da1bd9335cbecdd1c3ce1893f80e55376d018f",
                  "0x61616c1208b6b3491496370e7783d48426c674bdd7d04ed1a96afe2e4d8a3930",
                  "0x66ccccae2058641f136b79792d4d884419437826342ba84dfbbf3e52d8b3fc7d",
                  "0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
                  "0x6bbf2564ea4a6968df450da786b40b3f56b533a7b700c681c31b3714fc30256b",
                  "0x735c0a1cb33689ecba65907ba05a485f98831ff610955a44abf0a986f2904612",
                  "0x784a9514644c8ab6235aaff425381f2ea2719315a51388bc1f1e1c5afa2daaa9",
                  "0x7a8cee78757dfe0cee3631208cc81f171d27ca6004c63ebae5814e1754a03c79",
                  "0x803160c3a2f8e025df5a6e1110163493293dc974cc8abd43d4c1896000f4a1ec",
                  "0xcece26ebddbadfcfbc541baddc989fa73b919b82915164bbf77ebd86c7edbc90",
                  "0xe7be8996cbdf7db0f64abd17aa0968074b32e4b0df6560328921470e09fd608b"
               ],
               "proposer":"0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
               "timestamp":"1649395495746947"
            }
        )).unwrap();

        tailer
            .process_transaction(Arc::new(block_metadata_transaction.clone()))
            .await
            .unwrap();

        // This is a block metadata transaction
        let (tx1, ut1, bmt1, events1) =
            TransactionModel::get_by_version(69158, &conn_pool.get().unwrap()).unwrap();
        assert_eq!(tx1.type_, "block_metadata_transaction");
        assert!(ut1.is_none());
        assert!(bmt1.is_some());
        assert_eq!(events1.len(), 0);

        // This is the genesis transaction
        let (tx0, ut0, bmt0, events0) =
            TransactionModel::get_by_version(0, &conn_pool.get().unwrap()).unwrap();
        assert_eq!(tx0.type_, "genesis_transaction");
        assert!(ut0.is_none());
        assert!(bmt0.is_none());
        assert_eq!(events0.len(), 1);

        // A user transaction, with fake events
        let user_txn: Transaction = serde_json::from_value(json!(
            {
               "type":"user_transaction",
               "version":"691595",
               "hash":"0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb",
               "state_root_hash":"0xebfe1eb7aa5321e7a7d741d927487163c34c821eaab60646ae0efd02b286c97c",
               "event_root_hash":"0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
               "gas_used":"43",
               "success":true,
               "vm_status":"Executed successfully",
               "accumulator_root_hash":"0x97bfd5949d32f6c9a9efad93411924bfda658a8829de384d531ee73c2f740971",
               "sender":"0xdfd557c68c6c12b8c65908b3d3c7b95d34bb12ae6eae5a43ee30aa67a4c12494",
               "sequence_number":"21386",
               "max_gas_amount":"1000",
               "gas_unit_price":"1",
               "gas_currency_code":"XUS",
               "expiration_timestamp_secs":"1649713172",
               "payload":{
                  "type":"script_function_payload",
                  "function":"0x1::TestCoin::mint",
                  "type_arguments":[

                  ],
                  "arguments":[
                     "0x45b44793724a5ecc6ad85fa60949d0824cfc7f61d6bd74490b13598379313142",
                     "20000"
                  ]
               },
               "signature":{
                  "type":"ed25519_signature",
                  "public_key":"0x14ff6646855dad4a2dab30db773cdd4b22d6f9e6813f3e50142adf4f3efcf9f8",
                  "signature":"0x70781112e78cc8b54b86805c016cef2478bccdef21b721542af0323276ab906c989172adffed5bf2f475f2ec3a5b284a0ac46a6aef0d79f0dbb6b85bfca0080a"
               },
               "events":[
                  {
                     "key":"0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
                     "sequence_number":"0",
                     "type":"0x1::Whatever::FakeEvent1",
                     "data":{
                        "amazing":"1"
                     }
                  },
                  {
                     "key":"0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
                     "sequence_number":"1",
                     "type":"0x1::Whatever::FakeEvent2",
                     "data":{
                        "amazing":"2"
                     }
                  }
               ],
               "timestamp":"1649713141723410"
            }
        )).unwrap();

        // We run it twice to ensure we don't explode. Idempotency!
        tailer
            .process_transaction(Arc::new(user_txn.clone()))
            .await
            .unwrap();
        tailer
            .process_transaction(Arc::new(user_txn.clone()))
            .await
            .unwrap();

        // This is a user transaction, so the bmt should be None
        let (tx2, ut2, bmt2, events2) =
            TransactionModel::get_by_version(691595, &conn_pool.get().unwrap()).unwrap();
        assert_eq!(
            tx2.hash,
            "0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb"
        );
        assert!(ut2.is_some());
        assert!(bmt2.is_none());

        assert_eq!(events2.len(), 2);
        assert_eq!(events2.get(0).unwrap().type_, "0x1::Whatever::FakeEvent1");
        assert_eq!(events2.get(1).unwrap().type_, "0x1::Whatever::FakeEvent2");

        // Fetch the latest status
        let latest_version = tailer.set_fetcher_to_lowest_processor_version().await;
        assert_eq!(latest_version, 691595);
    }
}
