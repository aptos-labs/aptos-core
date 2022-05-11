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
    /// TODO: also handle gaps in sequence numbers (pg query for this is super easy)
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
    use crate::{
        database::{new_db_pool, PgPoolConnection},
        default_processor::DefaultTransactionProcessor,
        models::transactions::TransactionModel,
    };
    use diesel::Connection;
    use serde_json::json;

    pub fn wipe_database(conn: &PgPoolConnection) {
        for table in [
            "write_set_changes",
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
        if crate::should_skip_pg_tests() {
            return;
        }
        let (conn_pool, tailer) = setup_indexer().unwrap();
        // An abridged genesis transaction
        let genesis_txn: Transaction = serde_json::from_value(json!(
            {
               "type":"genesis_transaction",
               "version":"0",
               "hash":"0xa4d0d270d71cf031476dd2674d1e4a247489dfc3521c871ee37f42bd71a0a234",
               "state_root_hash":"0x27b382a98a32256a9e6403ca1f6e26998273d77afa9e8666e7ee13679af40a7a",
               "event_root_hash":"0xcbdbb1b830d1016d45a828bb3171ea81826e8315f14140acfbd7886f49fbcb40",
               "gas_used":"0",
               "success":true,
               "vm_status":"Executed successfully",
               "accumulator_root_hash":"0x6a527d06063dfd42c6b3a862574d5f3ec1660afb8058135edda5072712bfdb51",
               "changes":[
                  {
                     "type":"write_resource",
                     "address":"0x1",
                     "state_key_hash":"3502b05382fba777545b45a0a9d40e86cdde7c3afbde19c748ce8b5f142c2b46",
                     "data":{
                        "type":"0x1::Account::Account",
                        "data":{
                           "authentication_key":"0x1e4dcad3d5d94307f30d51ff66d2ce784e0c2822d3138766907179bcb61f9edc",
                           "self_address":"0x1",
                           "sequence_number":"0"
                        }
                     }
                  },
                  {
                     "type":"write_module",
                     "address":"0x1",
                     "state_key_hash":"e428253ccf0b18f3d8300c6a0d29de93abcdc526e88728abeb85d57aec558935",
                     "data":{
                        "bytecode":"0xa11ceb0b050000000a01000a020a04030e2305310e073f940108d3012006f3012c0a9f02050ca402370ddb020200000001000200030004000008000005000100000602000004080000000409000000030a030000020b030400010c05050000010202060c0201060c0105010307436861696e4964064572726f7273065369676e65720f53797374656d4164647265737365730954696d657374616d70036765740a696e697469616c697a65026964106173736572745f6f7065726174696e670e6173736572745f67656e65736973146173736572745f636f72655f7265736f757263650a616464726573735f6f6611616c72656164795f7075626c69736865640000000000000000000000000000000000000000000000000000000000000001030800000000000000000520000000000000000000000000000000000000000000000000000000000a550c18000201070200010001000006110207012b001000140201010000001211030a0011040a001105290020030d0b000107001106270b000b0112002d0002000000",
                        "abi":{
                           "address":"0x1",
                           "name":"ChainId",
                           "friends":[

                           ],
                           "exposed_functions":[
                              {
                                 "name":"get",
                                 "visibility":"public",
                                 "generic_type_params":[

                                 ],
                                 "params":[

                                 ],
                                 "return":[
                                    "u8"
                                 ]
                              },
                              {
                                 "name":"initialize",
                                 "visibility":"public",
                                 "generic_type_params":[

                                 ],
                                 "params":[
                                    "&signer",
                                    "u8"
                                 ],
                                 "return":[

                                 ]
                              }
                           ],
                           "structs":[
                              {
                                 "name":"ChainId",
                                 "is_native":false,
                                 "abilities":[
                                    "key"
                                 ],
                                 "generic_type_params":[

                                 ],
                                 "fields":[
                                    {
                                       "name":"id",
                                       "type":"u8"
                                    }
                                 ]
                              }
                           ]
                        }
                     }
                  }
               ],
               "payload":{
                  "type":"write_set_payload",
                  "write_set":{
                     "type":"direct_write_set",
                     "changes":[
                        {
                           "type":"write_resource",
                           "address":"0x1",
                           "state_key_hash":"3502b05382fba777545b45a0a9d40e86cdde7c3afbde19c748ce8b5f142c2b46",
                           "data":{
                              "type":"0x1::Account::Account",
                              "data":{
                                 "authentication_key":"0x1e4dcad3d5d94307f30d51ff66d2ce784e0c2822d3138766907179bcb61f9edc",
                                 "self_address":"0x1",
                                 "sequence_number":"0"
                              }
                           }
                        },
                        {
                           "type":"write_module",
                           "address":"0x1",
                           "state_key_hash":"e428253ccf0b18f3d8300c6a0d29de93abcdc526e88728abeb85d57aec558935",
                           "data":{
                              "bytecode":"0xa11ceb0b050000000a01000a020a04030e2305310e073f940108d3012006f3012c0a9f02050ca402370ddb020200000001000200030004000008000005000100000602000004080000000409000000030a030000020b030400010c05050000010202060c0201060c0105010307436861696e4964064572726f7273065369676e65720f53797374656d4164647265737365730954696d657374616d70036765740a696e697469616c697a65026964106173736572745f6f7065726174696e670e6173736572745f67656e65736973146173736572745f636f72655f7265736f757263650a616464726573735f6f6611616c72656164795f7075626c69736865640000000000000000000000000000000000000000000000000000000000000001030800000000000000000520000000000000000000000000000000000000000000000000000000000a550c18000201070200010001000006110207012b001000140201010000001211030a0011040a001105290020030d0b000107001106270b000b0112002d0002000000",
                              "abi":{
                                 "address":"0x1",
                                 "name":"ChainId",
                                 "friends":[

                                 ],
                                 "exposed_functions":[
                                    {
                                       "name":"get",
                                       "visibility":"public",
                                       "generic_type_params":[

                                       ],
                                       "params":[

                                       ],
                                       "return":[
                                          "u8"
                                       ]
                                    },
                                    {
                                       "name":"initialize",
                                       "visibility":"public",
                                       "generic_type_params":[

                                       ],
                                       "params":[
                                          "&signer",
                                          "u8"
                                       ],
                                       "return":[

                                       ]
                                    }
                                 ],
                                 "structs":[
                                    {
                                       "name":"ChainId",
                                       "is_native":false,
                                       "abilities":[
                                          "key"
                                       ],
                                       "generic_type_params":[

                                       ],
                                       "fields":[
                                          {
                                             "name":"id",
                                             "type":"u8"
                                          }
                                       ]
                                    }
                                 ]
                              }
                           }
                        }
                     ],
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
            }
        )).unwrap();

        tailer
            .process_transaction(Arc::new(genesis_txn.clone()))
            .await
            .unwrap();

        // A block_metadata_transaction
        let block_metadata_transaction: Transaction = serde_json::from_value(json!(
            {
              "type": "block_metadata_transaction",
              "version": "69158",
              "hash": "0x2b7c58ed8524d228f9d0543a82e2793d04e8871df322f976b0e7bb8c5ced4ff5",
              "state_root_hash": "0x3ead9eb40582fbc7df5e02f72280931dc3e6f1aae45dc832966b4cd972dac4b8",
              "event_root_hash": "0x2e481956dea9c59b6fc9f823fe5f4c45efce173e42c551c1fe073b5d76a65504",
              "gas_used": "0",
              "success": true,
              "vm_status": "Executed successfully",
              "accumulator_root_hash": "0xb0ad602f805eb20c398f0f29a3504a9ef38bcc52c9c451deb9ec4a2d18807b49",
              "id": "0xeef99391a3fc681f16963a6c03415bc0b1b12b56c00429308fa8bf46ac9eddf0",
              "round": "57600",
              "previous_block_votes": [],
              "epoch": "1",
              "previous_block_votes_bitmap": [
                true,
                true,
                false,
                true
              ],
              "proposer": "0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
              "timestamp": "1649395495746947",
              "changes": [
                {
                  "type": "write_resource",
                  "address": "0xa550c18",
                  "state_key_hash": "0x220a03e13099533097731c551fe037bbf404dcf765fe4df8743022a298650e6e",
                  "data": {
                    "type": "0x1::Block::BlockMetadata",
                    "data": {
                      "height": "1",
                      "new_block_events": {
                        "counter": "1",
                        "guid": {
                          "guid": {
                            "id": {
                              "addr": "0xa550c18",
                              "creation_num": "5"
                            }
                          },
                          "len_bytes": 40
                        }
                      }
                    }
                  }
                },
                {
                  "type": "write_resource",
                  "address": "0xa550c18",
                  "state_key_hash": "0xf113db06626eb7724773e4e9dacecc8a6cb3a710b8b70365768168b24fe06ce3",
                  "data": {
                    "type": "0x1::Timestamp::CurrentTimeMicroseconds",
                    "data": {
                      "microseconds": "1650419261396337"
                    }
                  }
                }
              ]
            }
        )).unwrap();

        tailer
            .process_transaction(Arc::new(block_metadata_transaction.clone()))
            .await
            .unwrap();

        // This is a block metadata transaction
        let (tx1, ut1, bmt1, events1, wsc1) =
            TransactionModel::get_by_version(69158, &conn_pool.get().unwrap()).unwrap();
        assert_eq!(tx1.type_, "block_metadata_transaction");
        assert!(ut1.is_none());
        assert!(bmt1.is_some());
        assert_eq!(events1.len(), 0);
        assert_eq!(wsc1.len(), 2);

        // This is the genesis transaction
        let (tx0, ut0, bmt0, events0, wsc0) =
            TransactionModel::get_by_version(0, &conn_pool.get().unwrap()).unwrap();
        assert_eq!(tx0.type_, "genesis_transaction");
        assert!(ut0.is_none());
        assert!(bmt0.is_none());
        assert_eq!(events0.len(), 1);
        assert_eq!(wsc0.len(), 2);

        // A user transaction, with fake events
        let user_txn: Transaction = serde_json::from_value(json!(
            {
              "type": "user_transaction",
              "version": "691595",
              "hash": "0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb",
              "state_root_hash": "0xebfe1eb7aa5321e7a7d741d927487163c34c821eaab60646ae0efd02b286c97c",
              "event_root_hash": "0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
              "gas_used": "43",
              "success": true,
              "vm_status": "Executed successfully",
              "accumulator_root_hash": "0x97bfd5949d32f6c9a9efad93411924bfda658a8829de384d531ee73c2f740971",
              "sender": "0xdfd557c68c6c12b8c65908b3d3c7b95d34bb12ae6eae5a43ee30aa67a4c12494",
              "sequence_number": "21386",
              "max_gas_amount": "1000",
              "gas_unit_price": "1",
              "expiration_timestamp_secs": "1649713172",
              "payload": {
                "type": "script_function_payload",
                "function": "0x1::TestCoin::mint",
                "type_arguments": [],
                "arguments": [
                  "0x45b44793724a5ecc6ad85fa60949d0824cfc7f61d6bd74490b13598379313142",
                  "20000"
                ]
              },
              "signature": {
                "type": "ed25519_signature",
                "public_key": "0x14ff6646855dad4a2dab30db773cdd4b22d6f9e6813f3e50142adf4f3efcf9f8",
                "signature": "0x70781112e78cc8b54b86805c016cef2478bccdef21b721542af0323276ab906c989172adffed5bf2f475f2ec3a5b284a0ac46a6aef0d79f0dbb6b85bfca0080a"
              },
              "events": [
                {
                  "key": "0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
                  "sequence_number": "0",
                  "type": "0x1::Whatever::FakeEvent1",
                  "data": {
                    "amazing": "1"
                  }
                },
                {
                  "key": "0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
                  "sequence_number": "1",
                  "type": "0x1::Whatever::FakeEvent2",
                  "data": {
                    "amazing": "2"
                  }
                }
              ],
              "timestamp": "1649713141723410",
              "changes": [
                {
                  "type": "write_resource",
                  "address": "0xa550c18",
                  "state_key_hash": "0x220a03e13099533097731c551fe037bbf404dcf765fe4df8743022a298650e6e",
                  "data": {
                    "type": "0x1::Block::BlockMetadata",
                    "data": {
                      "height": "1",
                      "new_block_events": {
                        "counter": "1",
                        "guid": {
                          "guid": {
                            "id": {
                              "addr": "0xa550c18",
                              "creation_num": "5"
                            }
                          },
                          "len_bytes": 40
                        }
                      }
                    }
                  }
                },
                {
                  "type": "write_resource",
                  "address": "0xa550c18",
                  "state_key_hash": "0xf113db06626eb7724773e4e9dacecc8a6cb3a710b8b70365768168b24fe06ce3",
                  "data": {
                    "type": "0x1::Timestamp::CurrentTimeMicroseconds",
                    "data": {
                      "microseconds": "1650419261396337"
                    }
                  }
                }
              ]
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
        let (tx2, ut2, bmt2, events2, wsc2) =
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
        assert_eq!(wsc2.len(), 2);

        // Fetch the latest status
        let latest_version = tailer.set_fetcher_to_lowest_processor_version().await;
        assert_eq!(latest_version, 691595);
    }
}
