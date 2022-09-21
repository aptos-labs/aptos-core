// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::models::ledger_info::LedgerInfo;
use crate::{
    database::{execute_with_better_error, PgDbPool},
    indexer::{
        errors::TransactionProcessingError,
        fetcher::{TransactionFetcher, TransactionFetcherOptions, TransactionFetcherTrait},
        processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    schema::ledger_infos::{self, dsl},
};
use anyhow::{ensure, Context, Result};
use aptos_api::context::Context as ApiContext;
use aptos_logger::{debug, info};
use chrono::ParseError;
use diesel::{
    prelude::*,
    sql_query,
    sql_types::{BigInt, Text},
    RunQueryDsl,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::{fmt::Debug, sync::Arc};
use tokio::{sync::Mutex, task::JoinHandle};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[derive(Clone)]
pub struct Tailer {
    pub transaction_fetcher: Arc<Mutex<dyn TransactionFetcherTrait>>,
    processor: Arc<dyn TransactionProcessor>,
    connection_pool: PgDbPool,
}

impl Tailer {
    pub fn new(
        context: Arc<ApiContext>,
        connection_pool: PgDbPool,
        processor: Arc<dyn TransactionProcessor>,
        options: TransactionFetcherOptions,
    ) -> Result<Tailer, ParseError> {
        let resolver = Arc::new(context.move_resolver().unwrap());
        let transaction_fetcher = TransactionFetcher::new(context, resolver, 0, options);

        Ok(Self {
            transaction_fetcher: Arc::new(Mutex::new(transaction_fetcher)),
            connection_pool,
            processor,
        })
    }

    pub fn run_migrations(&self) {
        let _ = &self
            .connection_pool
            .get()
            .expect("Could not get connection for migrations")
            .run_pending_migrations(MIGRATIONS)
            .expect("migrations failed!");
    }

    /// If chain id doesn't exist, save it. Otherwise, make sure that we're indexing the same chain
    pub async fn check_or_update_chain_id(&self) -> Result<u64> {
        info!(
            processor_name = self.processor.name(),
            "Checking if chain id is correct"
        );
        let mut conn = self
            .connection_pool
            .get()
            .expect("DB connection should be available at this stage");

        let query_chain = dsl::ledger_infos
            .select(dsl::chain_id)
            .load::<i64>(&mut conn)
            .expect("Error loading chain id from db");
        let maybe_existing_chain_id = query_chain.first();

        let new_chain_id = self
            .transaction_fetcher
            .lock()
            .await
            .fetch_ledger_info()
            .chain_id as i64;

        match maybe_existing_chain_id {
            Some(chain_id) => {
                ensure!(*chain_id == new_chain_id, "Wrong chain detected! Trying to index chain {} now but existing data is for chain {}", new_chain_id, chain_id);
                info!(
                    processor_name = self.processor.name(),
                    chain_id = chain_id,
                    "Chain id matches! Continue to index...",
                );
                Ok(*chain_id as u64)
            }
            None => {
                info!(
                    processor_name = self.processor.name(),
                    chain_id = new_chain_id,
                    "Adding chain id to db, continue to index.."
                );
                execute_with_better_error(
                    &mut conn,
                    diesel::insert_into(ledger_infos::table).values(LedgerInfo {
                        chain_id: new_chain_id,
                    }),
                    None,
                )
                .context(r#"Error updating chain_id!"#)
                .map(|_| new_chain_id as u64)
            }
        }
    }

    pub async fn set_fetcher_version(&self, version: u64) {
        self.transaction_fetcher
            .lock()
            .await
            .set_version(version)
            .await;
        info!(version = version, "Will start fetching from version");
    }

    pub async fn process_next_batch(
        &self,
    ) -> (u64, Result<ProcessingResult, TransactionProcessingError>) {
        let transactions = self
            .transaction_fetcher
            .lock()
            .await
            .fetch_next_batch()
            .await;

        let num_txns = transactions.len() as u64;
        let start_version = transactions.first().unwrap().version();
        let end_version = transactions.last().unwrap().version();

        debug!(
            num_txns = num_txns,
            start_version = start_version,
            end_version = end_version,
            "Starting processing of transaction batch"
        );

        let batch_start = chrono::Utc::now().naive_utc();

        let results = self
            .processor
            .process_transactions_with_status(transactions)
            .await;

        let batch_millis = (chrono::Utc::now().naive_utc() - batch_start).num_milliseconds();

        info!(
            num_txns = num_txns,
            time_millis = batch_millis,
            start_version = start_version,
            end_version = end_version,
            "Finished processing of transaction batch"
        );

        (num_txns, results)
    }

    /// Get starting version from database. Starting version is defined as the first version that's either
    /// not successful or missing from the DB.
    pub fn get_start_version(&self, processor_name: &String) -> Option<i64> {
        let mut conn = self
            .connection_pool
            .get()
            .expect("DB connection should be available to get starting version");

        // This query gets the first version that isn't equal to the next version (versions would be sorted of course).
        // There's also special handling if the gap happens in the beginning.
        let sql = "
          WITH raw_boundaries AS
          (
              SELECT
                  MAX(version) AS MAX_V,
                  MIN(version) AS MIN_V
              FROM
                  processor_statuses
              WHERE
                  name = $1
                  AND success = TRUE
          ),
          boundaries AS
          (
              SELECT
                  MAX(version) AS MAX_V,
                  MIN(version) AS MIN_V
              FROM
                  processor_statuses, raw_boundaries
              WHERE
                  name = $1
                  AND success = true
                  and version >= GREATEST(MAX_V - $2, 0)
          ),
          gap AS
          (
              SELECT
                  MIN(version) + 1 AS maybe_gap
              FROM
                  (
                      SELECT
                          version,
                          LEAD(version) OVER (
                      ORDER BY
                          version ASC) AS next_version
                      FROM
                          processor_statuses,
                          boundaries
                      WHERE
                          name = $1
                          AND success = TRUE
                          AND version >= GREATEST(MAX_V - $2, 0)
                  ) a
              WHERE
                  version + 1 <> next_version
          )
          SELECT
              CASE
                  WHEN
                      MIN_V <> GREATEST(MAX_V - $2, 0)
                  THEN
                      GREATEST(MAX_V - $2, 0)
                  ELSE
                      COALESCE(maybe_gap, MAX_V + 1)
              END
              AS version
          FROM
              gap, boundaries
          ";
        #[derive(Debug, QueryableByName)]
        pub struct Gap {
            #[diesel(sql_type = BigInt)]
            pub version: i64,
        }
        let mut res: Vec<Option<Gap>> = sql_query(sql)
            .bind::<Text, _>(processor_name)
            // This is the number used to determine how far we look back for gaps. Increasing it may result in slower startup
            .bind::<BigInt, _>(1500000)
            .get_results(&mut conn)
            .unwrap();
        res.pop().unwrap().map(|g| g.version)
    }
}

pub async fn await_tasks<T: Debug>(tasks: Vec<JoinHandle<T>>) -> Vec<T> {
    let mut results = vec![];
    for task in tasks {
        let result = task.await;
        match result {
            Ok(_) => results.push(result.unwrap()),
            Err(err) => {
                panic!("Error joining task: {:?}", err);
            }
        }
    }
    results
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        database::{new_db_pool, PgPoolConnection},
        models::transactions::TransactionModel,
        processors::default_processor::DefaultTransactionProcessor,
    };
    use aptos_api_test_context::new_test_context;
    use aptos_api_types::{LedgerInfo as APILedgerInfo, Transaction, U64};
    use serde_json::json;

    struct FakeFetcher {
        version: u64,
        chain_id: u8,
    }

    impl FakeFetcher {
        fn new(_starting_version: Option<u64>) -> Self {
            Self {
                version: 0,
                chain_id: 0,
            }
        }
    }

    #[async_trait::async_trait]
    impl TransactionFetcherTrait for FakeFetcher {
        async fn fetch_next_batch(&mut self) -> Vec<Transaction> {
            unimplemented!();
        }

        fn fetch_ledger_info(&mut self) -> APILedgerInfo {
            APILedgerInfo {
                chain_id: self.chain_id,
                epoch: U64::from(0),
                ledger_version: U64::from(0),
                ledger_timestamp: U64::from(0),
                oldest_ledger_version: U64::from(0),
                oldest_block_height: U64::from(0),
                block_height: U64::from(0),
            }
        }

        async fn set_version(&mut self, version: u64) {
            self.version = version;
            // Super hacky way of mocking chain_id
            self.chain_id = version as u8;
        }

        async fn start(&mut self) {
            // do nothing
        }
    }

    pub fn wipe_database(conn: &mut PgPoolConnection) {
        for command in [
            "DROP SCHEMA public CASCADE",
            "CREATE SCHEMA public",
            "GRANT ALL ON SCHEMA public TO postgres",
            "GRANT ALL ON SCHEMA public TO public",
        ] {
            diesel::sql_query(command).execute(conn).unwrap();
        }
    }

    pub fn setup_indexer() -> Result<(PgDbPool, Tailer)> {
        let database_url = std::env::var("INDEXER_DATABASE_URL")
            .expect("must set 'INDEXER_DATABASE_URL' to run tests!");
        let conn_pool = new_db_pool(database_url.as_str())?;
        wipe_database(&mut conn_pool.get()?);

        let test_context = new_test_context("doesnt_matter".to_string(), true);
        let context: Arc<ApiContext> = Arc::new(test_context.context);
        let pg_transaction_processor = DefaultTransactionProcessor::new(conn_pool.clone());
        let mut tailer = Tailer::new(
            context,
            conn_pool.clone(),
            Arc::new(pg_transaction_processor),
            TransactionFetcherOptions::default(),
        )?;
        tailer.transaction_fetcher = Arc::new(Mutex::new(FakeFetcher::new(None)));
        tailer.run_migrations();

        Ok((conn_pool, tailer))
    }

    #[tokio::test(flavor = "multi_thread")]
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
               "state_change_hash":"0x27b382a98a32256a9e6403ca1f6e26998273d77afa9e8666e7ee13679af40a7a",
               "event_root_hash":"0xcbdbb1b830d1016d45a828bb3171ea81826e8315f14140acfbd7886f49fbcb40",
               "gas_used":"0",
               "block_height":"0",
               "success":true,
               "vm_status":"Executed successfully",
               "accumulator_root_hash":"0x6a527d06063dfd42c6b3a862574d5f3ec1660afb8058135edda5072712bfdb51",
               "changes":[
                  {
                     "type":"write_resource",
                     "address":"0x1",
                     "state_key_hash":"3502b05382fba777545b45a0a9d40e86cdde7c3afbde19c748ce8b5f142c2b46",
                     "data":{
                        "type":"0x1::account::Account",
                        "data":{
                           "authentication_key":"0x1e4dcad3d5d94307f30d51ff66d2ce784e0c2822d3138766907179bcb61f9edc",
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
                              "type":"0x1::account::Account",
                              "data":{
                                 "authentication_key":"0x1e4dcad3d5d94307f30d51ff66d2ce784e0c2822d3138766907179bcb61f9edc",
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
                           "guid":{
                              "account_address":"0xa550c18",
                              "creation_number":"4",
                           },
                           "sequence_number":"0",
                           "type":"0x1::reconfiguration::NewEpochEvent",
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
                     "guid":{
                        "account_address":"0xa550c18",
                        "creation_number":"4",
                    },
                     "sequence_number":"0",
                     "type":"0x1::reconfiguration::NewEpochEvent",
                     "data":{
                        "epoch":"1"
                     }
                  }
               ]
            }
        )).unwrap();

        tailer
            .processor
            .process_transactions_with_status(vec![genesis_txn.clone()])
            .await
            .unwrap();

        // A block_metadata_transaction
        let block_metadata_transaction: Transaction = serde_json::from_value(json!(
            {
              "type": "block_metadata_transaction",
              "version": "69158",
              "block_height": "100",
              "hash": "0x2b7c58ed8524d228f9d0543a82e2793d04e8871df322f976b0e7bb8c5ced4ff5",
              "state_change_hash": "0x3ead9eb40582fbc7df5e02f72280931dc3e6f1aae45dc832966b4cd972dac4b8",
              "event_root_hash": "0x2e481956dea9c59b6fc9f823fe5f4c45efce173e42c551c1fe073b5d76a65504",
              "gas_used": "0",
              "success": true,
              "vm_status": "Executed successfully",
              "accumulator_root_hash": "0xb0ad602f805eb20c398f0f29a3504a9ef38bcc52c9c451deb9ec4a2d18807b49",
              "id": "0xeef99391a3fc681f16963a6c03415bc0b1b12b56c00429308fa8bf46ac9eddf0",
              "round": "57600",
              "failed_proposer_indices": [],
              "epoch": "1",
              "previous_block_votes_bitvec": [],
              "proposer": "0x68f04222bd9f8846cda028ea5ba3846a806b04a47e1f1a4f0939f350d713b2eb",
              "timestamp": "1649395495746947",
              "events": [
                 {
                    "key":"0x0600000000000000000000000000000000000000000000000000000000000000000000000a550c18",
                    "guid":{
                        "account_address":"0xa550c18",
                        "creation_number":"6",
                    },
                    "sequence_number": "0",
                    "type": "0x1::block::NewBlockEvent",
                    "data": {
                      "epoch": "1",
                      "failed_proposer_indices": [],
                      "previous_block_votes_bitvec": [],
                      "proposer": "0xf7c109be515785bba951fc8c51063515d474f78cad150457d6ebd08c4faf2f3b",
                      "round": "1",
                      "time_microseconds": "1656565270489235"
                    }
                }
              ],
              "changes": [
                {
                  "type": "write_resource",
                  "address": "0xa550c18",
                  "state_key_hash": "0x220a03e13099533097731c551fe037bbf404dcf765fe4df8743022a298650e6e",
                  "data": {
                    "type": "0x1::block::BlockResource",
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
            .processor
            .process_transactions_with_status(vec![block_metadata_transaction.clone()])
            .await
            .unwrap();

        // This is a block metadata transaction
        let (tx1, ut1, bmt1, events1, wsc1) =
            TransactionModel::get_by_version(69158, &mut conn_pool.get().unwrap()).unwrap();
        assert_eq!(tx1.type_, "block_metadata_transaction");
        assert!(ut1.is_none());
        assert!(bmt1.is_some());
        assert_eq!(events1.len(), 1);
        assert_eq!(wsc1.len(), 2);

        // This is the genesis transaction
        let (tx0, ut0, bmt0, events0, wsc0) =
            TransactionModel::get_by_version(0, &mut conn_pool.get().unwrap()).unwrap();
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
              "block_height": "100",
              "hash": "0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb",
              "state_change_hash": "0xebfe1eb7aa5321e7a7d741d927487163c34c821eaab60646ae0efd02b286c97c",
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
                "type": "entry_function_payload",
                "function": "0x1::aptos_coin::mint",
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
                  "guid":{
                      "account_address":"0xfefefefe",
                      "creation_number":"4",
                  },
                  "sequence_number": "0",
                  "type": "0x1::Whatever::FakeEvent1",
                  "data": {
                    "amazing": "1"
                  }
                },
                {
                  "key": "0x040000000000000000000000000000000000000000000000000000000000000000000000fefefefe",
                  "guid":{
                      "account_address":"0xfefefefe",
                      "creation_number":"4",
                  },
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
                    "type": "0x1::block::BlockResource",
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
            .processor
            .process_transactions_with_status(vec![user_txn.clone()])
            .await
            .unwrap();
        tailer
            .processor
            .process_transactions_with_status(vec![user_txn.clone()])
            .await
            .unwrap();

        // This is a user transaction, so the bmt should be None
        let (tx2, ut2, bmt2, events2, wsc2) =
            TransactionModel::get_by_version(691595, &mut conn_pool.get().unwrap()).unwrap();
        assert_eq!(
            tx2.hash,
            "0xefd4c865e00c240da0c426a37ceeda10d9b030d0e8a4fb4fb7ff452ad63401fb"
        );
        assert!(ut2.is_some());
        assert!(bmt2.is_none());

        assert_eq!(events2.len(), 2);
        assert_eq!(events2.first().unwrap().type_, "0x1::Whatever::FakeEvent1");
        assert_eq!(events2.get(1).unwrap().type_, "0x1::Whatever::FakeEvent2");
        assert_eq!(wsc2.len(), 2);

        // Message Transaction -> 0xb8bbd3936b05e3643f4b4f910bb00c9b6fa817c1935c74b9a16b5b7a2c8a69a3
        let message_txn: Transaction = serde_json::from_value(json!(
            {
              "type": "user_transaction",
              "version": "260885",
              "block_height": "100",
              "hash": "0xb8bbd3936b05e3643f4b4f910bb00c9b6fa817c1935c74b9a16b5b7a2c8a69a3",
              "state_change_hash": "0xde91b595abbeef217fb0be956df0909c1459ba8d82ed12b983e226ecbf0a4ec5",
              "event_root_hash": "0x414343554d554c41544f525f504c414345484f4c4445525f4841534800000000",
              "gas_used": "143",
              "success": true,
              "vm_status": "Executed successfully",
              "accumulator_root_hash": "0xef40b1120b1873d2c3a4a91eafa4084e24ff1529a0f31959e88f6387054c8fe0",
              "changes": [
                {
                  "type": "write_resource",
                  "address": "0x2a0e66fde889cebf0401e676bb9bfa073e03caa9c009c66b739c30d24dccad81",
                  "state_key_hash": "0xd210490c73366517a3976e1585086ec85e9f820194dd29872ad49bd87d46e66e",
                  "data": {
                    "type": "0x2a0e66fde889cebf0401e676bb9bfa073e03caa9c009c66b739c30d24dccad81::Message::MessageHolder",
                    "data": {
                      "message": "he\u{0}\u{0} \u{000} w\\0007 \\0 \\00 \u{0000} \\u0000 d!",
                      "message_change_events": {
                        "counter": "0",
                        "guid": {
                          "guid": {
                            "id": {
                              "addr": "0x2a0e66fde889cebf0401e676bb9bfa073e03caa9c009c66b739c30d24dccad81",
                              "creation_num": "2"
                            }
                          },
                          "len_bytes": 40
                        }
                      }
                    }
                  }
                }
              ],
              "sender": "0x2a0e66fde889cebf0401e676bb9bfa073e03caa9c009c66b739c30d24dccad81",
              "sequence_number": "6",
              "max_gas_amount": "1000",
              "gas_unit_price": "1",
              "expiration_timestamp_secs": "1651789617",
              "payload": {
                "type": "entry_function_payload",
                "function": "0x2a0e66fde889cebf0401e676bb9bfa073e03caa9c009c66b739c30d24dccad81::Message::set_message",
                "type_arguments": [],
                "arguments": [
                  "0x68650000207707206421"
                ]
              },
              "signature": {
                "type": "ed25519_signature",
                "public_key": "0xe355b88fc001857a2cc9fe55007889cd1561aed56d187fe65729c50274c37398",
                "signature": "0x9c1fef826ead87392f945bce527169b6627205a8d3bae77c5d8293c00b6e6a7657b4464b1fe2b36b89f5a2e64468ce7a04191d5fba431f1dc084f90292c9eb04"
              },
              "events": [],
              "timestamp": "1651789018411640"
            }
        )).unwrap();

        let txns = vec![message_txn];
        tailer
            .processor
            .process_transactions_with_status(txns)
            .await
            .unwrap();

        let (_conn_pool, tailer) = setup_indexer().unwrap();
        tailer.set_fetcher_version(4).await;
        assert!(tailer.check_or_update_chain_id().await.is_ok());
        assert!(tailer.check_or_update_chain_id().await.is_ok());

        tailer.set_fetcher_version(10).await;
        assert!(tailer.check_or_update_chain_id().await.is_err());

        tailer.set_fetcher_version(4).await;
        assert!(tailer.check_or_update_chain_id().await.is_ok());
    }
}
