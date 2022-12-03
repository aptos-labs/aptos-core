// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use tonic::{Request, Response, Status};

use aptos_datastream_worker::redis_pool_client::RedisClient;
use aptos_protos::datastream::v1::indexer_stream_server::IndexerStream;
use aptos_protos::datastream::v1::transactions_data::TransactionData;
use aptos_protos::datastream::v1::{RawDatastreamRequest, RawDatastreamResponse, TransactionsData};
use futures::Stream;
use std::sync::Arc;
use std::{pin::Pin, time, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

pub struct DatastreamServer {
    pub redis_client: Arc<RedisClient>,
}

#[tonic::async_trait]
impl IndexerStream for DatastreamServer {
    type RawDatastreamStream = ResponseStream;
    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        let (tx, rx) = mpsc::channel(10000);

        let mut current_version = req.into_inner().starting_version;
        let mut latest_version = self
            .redis_client
            .get("processed_version".to_string())
            .await
            .parse::<u64>()
            .unwrap();
        let redis_client = self.redis_client.clone();

        tokio::spawn(async move {
            let mut prev_time = std::time::Instant::now();
            loop {
                if current_version <= latest_version {
                    let encoded_proto_data = redis_client.get(current_version.to_string()).await;
                    let item = RawDatastreamResponse {
                        data: Some(TransactionsData {
                            transactions: vec![TransactionData {
                                encoded_proto_data,
                                version: current_version,
                            }],
                        }),
                        ..RawDatastreamResponse::default()
                    };
                    current_version += 1;

                    match tx.send(Result::<_, Status>::Ok(item.clone())).await {
                        Ok(_) => {}
                        Err(_) => {
                            // Client disconnects.
                            break;
                        }
                    }
                } else {
                    // if we hit the head,  wait 50 milliseconds.
                    std::thread::sleep(time::Duration::from_millis(50));
                    latest_version = redis_client
                        .get("processed_version".to_string())
                        .await
                        .parse::<u64>()
                        .unwrap();
                }

                if current_version % 1000 == 0 {
                    let current_time = std::time::Instant::now();
                    let diff = (current_time - prev_time).as_millis();

                    println!("[Datastream service] tps {}", 1000_000 as f64 / diff as f64);
                    prev_time = current_time;
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::RawDatastreamStream
        ))
    }
}
