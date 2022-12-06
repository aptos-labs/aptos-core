// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{
    APTOS_DATASTREAM_WORKER_CONFIG_PATH_VAR, CHAIN_ID_REDIS_KEY, REDIS_PROCESS_STATUS,
};
use crate::redis_pool_client::RedisClient;
use crate::DatastreamWorkerConfig;

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
    pub redis_client: Arc<RedisClient>,
    pub config: DatastreamWorkerConfig,
    pub current_version: Arc<AtomicU64>,
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

        let redis_client = Arc::new(RedisClient::new(format!(
            "{}:{}",
            redis_address, redis_port
        )));

        let starting_version = Arc::new(AtomicU64::new(match config.starting_version {
            Some(num) => num,
            _ => 0_u64,
        }));

        Self {
            redis_client,
            config,
            current_version: starting_version,
        }
    }

    pub async fn run(&self) {
        // Before everything starts, verify the chain id. If not present, set it.
        let current_chain_id = self
            .redis_client
            .getset(
                CHAIN_ID_REDIS_KEY.to_string(),
                self.config.chain_id.to_string(),
            )
            .await;
        assert_eq!(current_chain_id, self.config.chain_id.to_string());
        // Re-connect if lost.
        loop {
            let mut rpc_client =
                datastream::indexer_stream_client::IndexerStreamClient::connect(format!(
                    "http://{}:{}",
                    self.config.indexer_address, self.config.indexer_port
                ))
                .await
                .unwrap();

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
                starting_version: self.current_version.load(Ordering::SeqCst),
                chain_id: self.config.chain_id as u32,
                output_batch_size: match self.config.output_transaction_batch_size {
                    Some(num) => num,
                    _ => 100_u64,
                },
            });

            let response = rpc_client.raw_datastream(request).await.unwrap();
            let mut resp_stream = response.into_inner();
            let mut init_signal_received = false;
            let mut batch_count = 0_u64;
            while let Some(received) = resp_stream.next().await {
                let received = received.unwrap();
                match received.response.unwrap() {
                    datastream::raw_datastream_response::Response::Status(status) => {
                        match status.r#type {
                            0 => {
                                if init_signal_received {
                                    println!("Multiple init signals received. Restarting...");
                                    break;
                                }
                                init_signal_received = true;

                                // If requested starting version doesn't match, restart.
                                if self.current_version.load(Ordering::SeqCst)
                                    != status.start_version
                                {
                                    {
                                        println!("Current processing contains gap. Restarting...");
                                        break;
                                    }
                                }
                            }
                            1 => {
                                if status.end_version.unwrap() + 1
                                    != self.current_version.load(Ordering::SeqCst) + batch_count
                                {
                                    println!("Gap detected in processing with batch sizse{},  {} vs {}. Restarting", batch_count,  status.end_version(), self.current_version.load(Ordering::SeqCst));
                                    break;
                                }
                                self.current_version
                                    .store(status.end_version.unwrap() + 1, Ordering::SeqCst);
                                self.redis_client
                                    .set(
                                        REDIS_PROCESS_STATUS.to_string(),
                                        status.end_version.unwrap().to_string(),
                                    )
                                    .await;
                                batch_count = 0;
                            }
                            _ => {
                                // There might be protobuf inconsistency between server and client.
                                // Panic to block running.
                                panic!("Unknown RawDatastreamResponse status type.");
                            }
                        }
                    }
                    datastream::raw_datastream_response::Response::Data(data) => {
                        let kv_pairs: Vec<String> = data
                            .transactions
                            .into_iter()
                            .flat_map(|e| vec![e.version.to_string(), e.encoded_proto_data])
                            .collect();
                        batch_count += (&kv_pairs.len() / 2) as u64;
                        self.redis_client.multiset(kv_pairs).await;
                    }
                };
            }
        }
    }
}
