// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use tonic::{Request, Response, Status};

use aptos_protos::datastream::v1::node_data_service_server::NodeDataService;
use aptos_protos::datastream::v1::{RawDatastreamRequest, RawDatastreamResponse, TransactionsData};
use futures::Stream;
use std::{pin::Pin, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

type ResponseStream = Pin<Box<dyn Stream<Item = Result<RawDatastreamResponse, Status>> + Send>>;

#[derive(Debug)]
pub struct DatastreamServer {}

#[tonic::async_trait]
impl NodeDataService for DatastreamServer {
    type RawDatastreamStream = ResponseStream;

    async fn raw_datastream(
        &self,
        req: Request<RawDatastreamRequest>,
    ) -> Result<Response<Self::RawDatastreamStream>, Status> {
        println!("EchoServer::server_streaming_echo");
        println!("\tclient connected from: {:?}", req.remote_addr());

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(RawDatastreamResponse {
            data: Some(TransactionsData {
                ..TransactionsData::default()
            }),
            ..RawDatastreamResponse::default()
        });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(128);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
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
