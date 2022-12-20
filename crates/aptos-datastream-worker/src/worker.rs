// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    APTOS_DATASTREAM_WORKER_CONFIG_PATH_VAR, CHAIN_ID_REDIS_KEY, REDIS_PROCESS_STATUS,
};
use crate::DatastreamWorkerConfig;
use deadpool_redis::{redis::cmd, Config, Pool, Runtime};
use moving_average::MovingAverage;

use aptos_logger::{error, info};
use aptos_protos::datastream::v1 as datastream;
use futures;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

pub fn get_worker_config_file_path() -> String {
    std::env::var(APTOS_DATASTREAM_WORKER_CONFIG_PATH_VAR).expect("WORKER_CONFIG_PATH is required.")
}

pub struct Worker {
    pub redis_pool: Arc<Pool>,
    pub config: DatastreamWorkerConfig,
    /// Next version to process. It is used to determine the starting version of the next batch.
    pub next_version: Arc<AtomicU64>,
}

impl Worker {
    pub async fn new() -> Self {
        let config_path = get_worker_config_file_path();
        let config = DatastreamWorkerConfig::load(PathBuf::from(config_path)).unwrap();
        let redis_address = match &config.redis_address {
            Some(addr) => addr.clone(),
            _ => "127.0.0.1".to_string(),
        };
        let redis_port = match config.redis_port {
            Some(port) => port,
            _ => 6379_u64,
        };
        let cfg = Config::from_url(format!("redis://{}:{}", redis_address, redis_port));

        let redis_pool = Arc::new(cfg.create_pool(Some(Runtime::Tokio1)).unwrap());

        let starting_version = Arc::new(AtomicU64::new(match config.starting_version {
            Some(num) => num,
            _ => 0_u64,
        }));

        Self {
            redis_pool,
            config,
            next_version: starting_version,
        }
    }

    pub async fn run(&self) {
        let mut conn = self.redis_pool.get().await.unwrap();

        // Before everything starts, verify the chain id. If not present, set it.
        let chan_id_exists: bool = cmd("EXISTS")
            .arg(&[CHAIN_ID_REDIS_KEY.to_string()])
            .query_async(&mut conn)
            .await
            .expect("[Redis] Get the chain id.");
        if !chan_id_exists {
            cmd("SET")
                .arg(&[
                    CHAIN_ID_REDIS_KEY.to_string(),
                    self.config.chain_id.to_string(),
                ])
                .query_async::<_, ()>(&mut conn)
                .await
                .expect("[Redis] Set the chain id.");
        } else {
            let chain_id: String = cmd("GET")
                .arg(&[CHAIN_ID_REDIS_KEY.to_string()])
                .query_async(&mut conn)
                .await
                .expect("[Redis] Get the chain id.");
            assert_eq!(chain_id, self.config.chain_id.to_string());
        }
        // Re-connect if lost.
        loop {
            let mut rpc_client =
                match datastream::indexer_stream_client::IndexerStreamClient::connect(format!(
                    "http://{}:{}",
                    self.config.indexer_address, self.config.indexer_port
                ))
                .await
                {
                    Ok(client) => client,
                    Err(e) => {
                        error!(
                            indexer_address = self.config.indexer_address,
                            indexer_port = self.config.indexer_port,
                            "[Datasteram Worker]Error connecting to indexer"
                        );
                        panic!("[Datastream Worker] Error connecting to indexer: {}", e);
                    }
                };
            let mut ma = MovingAverage::new(10_000);
            let request = tonic::Request::new(datastream::RawDatastreamRequest {
                processor_task_count: match self.config.processor_task_count {
                    Some(num) => num,
                    _ => 10,
                },
                processor_batch_size: match self.config.processor_batch_size {
                    Some(num) => num,
                    _ => 10,
                },
                // Loads from the recent successful starting version.
                starting_version: self.next_version.load(Ordering::SeqCst),
                chain_id: self.config.chain_id as u32,
                output_batch_size: match self.config.output_transaction_batch_size {
                    Some(num) => num,
                    _ => 100_u64,
                },
            });

            let response = rpc_client.raw_datastream(request).await.unwrap();
            let mut resp_stream = response.into_inner();
            let mut init_signal_received = false;
            while let Some(received) = resp_stream.next().await {
                let received = match received {
                    Ok(r) => r,
                    Err(e) => {
                        // If the connection is lost, reconnect.
                        error!(
                            "[Datastream Worker] Error receiving datastream response: {}",
                            e
                        );
                        break;
                    }
                };
                match received.response.unwrap() {
                    datastream::raw_datastream_response::Response::Status(status) => {
                        match status.r#type {
                            0 => {
                                if init_signal_received {
                                    error!("[Datastream Worker]Multiple init signals received. Restarting...");
                                    break;
                                }
                                init_signal_received = true;

                                // If requested starting version doesn't match, restart.
                                if self.next_version.load(Ordering::SeqCst) != status.start_version
                                {
                                    {
                                        error!("[Datastream Worker] Current processing contains gap. Restarting...");
                                        break;
                                    }
                                }
                            }
                            1 => {
                                self.next_version
                                    .store(status.end_version.unwrap() + 1, Ordering::SeqCst);
                                cmd("SET")
                                    .arg(&[
                                        REDIS_PROCESS_STATUS.to_string(),
                                        status.end_version.unwrap().to_string(),
                                    ])
                                    .query_async::<_, ()>(&mut conn)
                                    .await
                                    .unwrap();
                            }
                            _ => {
                                // There might be protobuf inconsistency between server and client.
                                // Panic to block running.
                                panic!("[Datastream Worker] Unknown RawDatastreamResponse status type.");
                            }
                        }
                    }
                    datastream::raw_datastream_response::Response::Data(data) => {
                        let batch_start_version =
                            data.transactions.as_slice().first().unwrap().version;
                        let batch_end_version =
                            data.transactions.as_slice().last().unwrap().version;

                        let kv_pairs: Vec<String> = data
                            .transactions
                            .into_iter()
                            .flat_map(|e| vec![e.version.to_string(), e.encoded_proto_data])
                            .collect();
                        let batch_count = (&kv_pairs.len() / 2) as u64;
                        cmd("MSET")
                            .arg(&kv_pairs)
                            .query_async::<_, ()>(&mut conn)
                            .await
                            .unwrap();
                        ma.tick_now(batch_count);
                        info!(
                            batch_start_version = batch_start_version,
                            batch_end_version = batch_end_version,
                            tps = (ma.avg() * 1000.0) as u64,
                            "[Datastream Worker] Sent batch successfully"
                        );
                    }
                };
            }
        }
    }
}
