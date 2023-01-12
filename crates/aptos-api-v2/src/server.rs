// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_api::context::Context;
use aptos_protos::api::v2::{api_server::Api, HelloReply, HelloRequest};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct ApiService {
    pub context: Arc<Context>,
}

#[tonic::async_trait]
impl Api for ApiService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request: {:?}", request);

        let reply = HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };

        Ok(Response::new(reply))
    }
}
