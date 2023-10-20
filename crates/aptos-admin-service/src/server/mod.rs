// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::reply_with_status;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    thread,
};

#[cfg(target_os = "linux")]
mod profiling;
mod utils;

/// Starts the admin service that listens on the configured address and handles various endpoint
/// requests.
pub fn start_admin_service(node_config: &NodeConfig) {
    // Fetch the service port and address
    let service_port = node_config.admin_service.port;
    let service_address = node_config.admin_service.address.clone();

    // Create the admin service socket address
    let address: SocketAddr = (service_address.as_str(), service_port)
        .to_socket_addrs()
        .unwrap_or_else(|_| {
            panic!(
                "Failed to parse {}:{} as address",
                service_address, service_port
            )
        })
        .next()
        .unwrap();

    // Create a runtime for the admin service
    let runtime = aptos_runtimes::spawn_named_runtime("admin".into(), None);

    // TODO(grao): Consider support enabling the service through an authenticated request.
    let enabled = node_config.admin_service.enabled.unwrap_or(false);
    thread::spawn(move || {
        let make_service = make_service_fn(move |_conn| async move {
            Ok::<_, Infallible>(service_fn(move |req| serve_requests(req, enabled)))
        });

        runtime
            .block_on(async move {
                let server = Server::bind(&address).serve(make_service);
                info!("Started AdminService at {address:?}, enabled: {enabled}.");
                server.await
            })
            .unwrap();
    });
}

async fn serve_requests(req: Request<Body>, enabled: bool) -> hyper::Result<Response<Body>> {
    if !enabled {
        return Ok(reply_with_status(
            StatusCode::NOT_FOUND,
            "AdminService is not enabled.",
        ));
    }
    match (req.method().clone(), req.uri().path()) {
        #[cfg(target_os = "linux")]
        (Method::GET, "/profilez") => profiling::handle_cpu_profiling_request(req).await,
        _ => Ok(reply_with_status(StatusCode::NOT_FOUND, "Not found.")),
    }
}
