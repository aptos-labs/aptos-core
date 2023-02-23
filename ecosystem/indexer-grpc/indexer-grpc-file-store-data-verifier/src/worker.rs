// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator, config::IndexerGrpcConfig, create_grpc_client,
    file_store_operator::FileStoreOperator,
};
use aptos_protos::datastream::v1::{raw_datastream_response::Response, RawDatastreamRequest};
use futures::{self, StreamExt};
use tokio::sync::mpsc::channel;

pub struct Worker {
    config: IndexerGrpcConfig,
}

impl Worker {
    pub fn new(config: IndexerGrpcConfig) -> Self {
        Self { config }
    }

    pub async fn run(&mut self) {
        let redis_client = redis::Client::open(format!("redis://{}", self.config.redis_address))
            .expect("Create redis client failed.");
        let conn = redis_client
            .get_async_connection()
            .await
            .expect("Create redis connection failed.");
        let mut cache_operator = CacheOperator::new(conn);
        let chain_id = cache_operator
            .get_chain_id()
            .await
            .expect("Get chain id failed.");

        let (file_rx, mut file_tx) = channel::<String>(100_000);
        let (grpc_rx, mut grpc_tx) = channel::<String>(100_000);

        let indexer_address = self.config.fullnode_grpc_address.as_ref().unwrap().clone();
        let file_store_bucket_name = self.config.file_store_bucket_name.clone();
        // Spawn one task to fetch data from grpc server.
        tokio::spawn(async move {
            let gprc_sender = grpc_rx.clone();
            let mut grpc_client = create_grpc_client(format!("http://{}", indexer_address)).await;
            let req = RawDatastreamRequest {
                starting_version: 0,
                ..RawDatastreamRequest::default()
            };
            let mut stream = grpc_client.raw_datastream(req).await.unwrap().into_inner();

            let mut tmap = std::collections::BTreeMap::<u64, String>::new();
            while let Some(resp) = stream.next().await {
                let resp = resp.unwrap();
                let response_type = resp.response;
                if let Some(response) = response_type {
                    match response {
                        Response::Data(d) => {
                            for t in d.transactions {
                                tmap.insert(t.version, t.encoded_proto_data);
                            }
                        },
                        Response::Status(d) => match d.r#type {
                            0 => {},
                            1 => {
                                for i in tmap.values() {
                                    gprc_sender.send(i.clone()).await.unwrap();
                                }
                                tmap.clear();
                            },
                            _ => {
                                panic!("Unknown status type.")
                            },
                        },
                    }
                }
            }
        });
        tokio::spawn(async move {
            let file_rx = file_rx.clone();
            let file_store_operator = FileStoreOperator::new(file_store_bucket_name);
            file_store_operator.bootstrap().await;

            let mut starting_version = 0;
            loop {
                // Metadata exists.
                let metadata = file_store_operator.get_file_store_metadata().await.unwrap();
                if metadata.chain_id != chain_id {
                    panic!("Chain id not match.");
                }

                let file_version = metadata.version;
                while starting_version < file_version {
                    let data = file_store_operator
                        .get_transactions_file(starting_version)
                        .await
                        .unwrap();

                    let len = data.transactions.len() as u64;
                    for t in data.transactions {
                        file_rx.send(t).await.unwrap();
                    }
                    starting_version += len;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        aptos_logger::info!("Worker started.");
        let mut verified_count = 0;
        loop {
            let file_data = file_tx.recv().await.unwrap();
            let grpc_data = grpc_tx.recv().await.unwrap();
            if file_data != grpc_data {
                panic!("Data not match. {}", verified_count);
            }
            verified_count += 1;
            if verified_count % 1000 == 0 {
                aptos_logger::info!(verified_count = verified_count, "Succssefully verified.");
            }
        }
    }
}
