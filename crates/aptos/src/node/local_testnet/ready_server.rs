// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::node::local_testnet::health_checker::HealthChecker;
use anyhow::Result;
use clap::Parser;
use poem::{
    get, handler,
    http::StatusCode,
    listener::TcpListener,
    middleware::Tracing,
    web::{Data, Json},
    EndpointExt, IntoResponse, Route, Server,
};
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddrV4};

#[derive(Debug, Clone, Parser)]
pub struct ReadyServerConfig {
    /// The port to run the ready server. This exposes an endpoint at `/` that you can
    /// use to check if the entire local testnet is ready.
    #[clap(long, default_value_t = 8090)]
    pub ready_server_listen_port: u16,
}

/// This returns a future that runs a web server that exposes a single unified health
/// checking port. Clients can use this to check if all the services are ready.
pub async fn run_ready_server(
    health_checkers: Vec<HealthChecker>,
    config: ReadyServerConfig,
) -> Result<()> {
    let app = Route::new()
        .at("/", get(root))
        .data(HealthCheckers { health_checkers })
        .with(Tracing);
    Server::new(TcpListener::bind(SocketAddrV4::new(
        Ipv4Addr::new(0, 0, 0, 0),
        config.ready_server_listen_port,
    )))
    .name("ready-server")
    .run(app)
    .await?;
    Err(anyhow::anyhow!("Ready server exited unexpectedly"))
}

#[derive(Clone, Debug)]
struct HealthCheckers {
    pub health_checkers: Vec<HealthChecker>,
}

#[derive(Serialize)]
struct ReadyData {
    pub ready: Vec<HealthChecker>,
    pub not_ready: Vec<HealthChecker>,
}

#[handler]
async fn root(health_checkers: Data<&HealthCheckers>) -> impl IntoResponse {
    let mut ready = vec![];
    let mut not_ready = vec![];
    for health_checker in &health_checkers.health_checkers {
        match health_checker.check().await {
            Ok(()) => ready.push(health_checker.clone()),
            Err(_) => {
                not_ready.push(health_checker.clone());
            },
        }
    }
    let status_code = if not_ready.is_empty() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    Json(ReadyData { ready, not_ready }).with_status(status_code)
}
