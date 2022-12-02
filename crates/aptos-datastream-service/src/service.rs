// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use tonic::{Request, Response, Status};

use aptos_datastream_worker::redis_pool_client::RedisClient;
use aptos_protos::datastream::v1::node_data_service_server::NodeDataService;
use aptos_protos::datastream::v1::transactions_data::TransactionData;
use aptos_protos::datastream::v1::{RawDatastreamRequest, RawDatastreamResponse, TransactionsData};
use futures::Stream;
use std::{pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

pub struct DatastreamServer {
    pub redis_client: RedisClient,
}

#[tonic::async_trait]
impl NodeDataService for DatastreamServer {
    type RawDatastreamStream = ResponseStream;
    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            let item = RawDatastreamResponse {
                data: Some(TransactionsData {
                    transactions: vec![TransactionData {
                        encoded_proto_data: "dummy_txn_data".to_string(),
                        version: 123,
                    }],
                }),
                ..RawDatastreamResponse::default()
            };
            loop {
                match tx.send(Result::<_, Status>::Ok(item.clone())).await {
                    Ok(_) => {}
                    Err(_) => {
                        // Client disconnects.
                        break;
                    }
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
