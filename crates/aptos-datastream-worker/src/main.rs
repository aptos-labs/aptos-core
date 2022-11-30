// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_datastream_worker::redis_pool_client::{RedisClient, RedisWork};
use aptos_protos::datastream::v1 as datastream;
use futures;
use futures::StreamExt;
use std::sync::Arc;
use tokio::runtime::Builder;
use tokio::sync::mpsc;

const DEFAULT_PROCESSOR_TASK_COUNT: u64 = 10;
const DEFAULT_FETCHER_TASK_COUNT: u64 = 10;
const DEFAULT_REDIS_WORK_CHANNEL_SIZE: usize = 10_000;

pub fn get_indexer_rpc_address() -> String {
    std::env::var("INDEXER_RPC_ADDRESS").expect("INDEXER_RPC_ADDRESS is required.")
}

pub fn get_redis_address() -> String {
    std::env::var("REDIS_ADDRESS").expect("REDIS_ADDRESS is required.")
}

async fn run_forever() {
    let mut rpc_client = datastream::node_data_service_client::NodeDataServiceClient::connect(
        format!("http://{}", get_indexer_rpc_address()),
    )
    .await
    .unwrap();
    let (tx, mut rx) = mpsc::channel::<Arc<RedisWork>>(DEFAULT_REDIS_WORK_CHANNEL_SIZE);
    let redis_client = Arc::new(RedisClient::new(get_redis_address()));
    // Redis processing.
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            redis_client
                .set(
                    message.as_ref().key.to_owned(),
                    message.as_ref().val.to_owned(),
                )
                .await;
        }
    });
    // Re-connect if lost.
    loop {
        let request = tonic::Request::new(datastream::RawDatastreamRequest {
            processor_task_count: DEFAULT_PROCESSOR_TASK_COUNT,
            fetcher_count: DEFAULT_FETCHER_TASK_COUNT,
            starting_version: 0,
        });

        let response = rpc_client.raw_datastream(request).await.unwrap();
        let mut resp_stream = response.into_inner();
        while let Some(received) = resp_stream.next().await {
            let received = received.unwrap();
            for txn in received.data.unwrap().transactions {
                tx.send(Arc::new(RedisWork::new(
                    txn.version,
                    txn.encoded_proto_data,
                )))
                .await
                .unwrap();
            }
        }
    }
}

fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_name("aptos-datastream-worker")
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer] failed to create runtime");

    // Start processing.
    runtime.spawn(async move {
        run_forever().await;
    });

    std::thread::park();
}
