// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_data_service::service::DatastreamServer;
use aptos_indexer_grpc_utils::register_probes_and_metrics_handler;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_server::IndexerStreamServer,
        FILE_DESCRIPTOR_SET as DATASTREAM_V1_FILE_DESCRIPTOR_SET,
    },
    transaction::testing1::v1::FILE_DESCRIPTOR_SET as TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET,
    util::timestamp::FILE_DESCRIPTOR_SET as UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET,
};
use clap::Parser;
use std::{
    collections::HashSet,
    net::ToSocketAddrs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tonic::{
    metadata::{Ascii, MetadataValue},
    transport::Server,
    Request, Status,
};

#[derive(Parser)]
pub struct Args {
    #[clap(short, long)]
    pub config_path: String,
}

fn main() {
    aptos_logger::Logger::new().init();
    aptos_crash_handler::setup_panic_handler();

    // Load config.
    let args = Args::parse();
    let config = aptos_indexer_grpc_utils::config::IndexerGrpcConfig::load(
        std::path::PathBuf::from(args.config_path),
    )
    .unwrap();

    let grpc_address = config
        .data_service_grpc_listen_address
        .clone()
        .expect("grpc_address not set");
    let health_port = config.health_check_port;

    let token_set = build_auth_token_set(config.whitelisted_auth_tokens.clone());
    let authentication_inceptor = move |req: Request<()>| {
        let metadata = req.metadata();
        if let Some(token) =
            metadata.get(aptos_indexer_grpc_utils::constants::GRPC_AUTH_TOKEN_HEADER)
        {
            if token_set.contains(token) {
                Ok(req)
            } else {
                Err(Status::unauthenticated("Invalid token"))
            }
        } else {
            Err(Status::unauthenticated("Missing token"))
        }
    };
    let runtime = aptos_runtimes::spawn_named_runtime("indexerdata".to_string(), None);

    let reflection_service = tonic_reflection::server::Builder::configure()
        // Note: It is critical that the file descriptor set is registered for every
        // file that the top level API proto depends on recursively. If you don't,
        // compilation will still succeed but reflection will fail at runtime.
        //
        // TODO: Add a test for this / something in build.rs, this is a big footgun.
        .register_encoded_file_descriptor_set(DATASTREAM_V1_FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(TRANSACTION_V1_TESTING_FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(UTIL_TIMESTAMP_FILE_DESCRIPTOR_SET)
        .build()
        .expect("Failed to build reflection service");

    // Add authentication interceptor.
    runtime.spawn(async move {
        let server = DatastreamServer::new(config);
        let svc = IndexerStreamServer::with_interceptor(server, authentication_inceptor);
        Server::builder()
            .add_service(reflection_service)
            .add_service(svc)
            .serve(grpc_address.to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
    });

    // Start liveness and readiness probes.
    runtime.spawn(async move {
        register_probes_and_metrics_handler(health_port).await;
    });

    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}

/// Build a set of whitelisted auth tokens. Invalid tokens are ignored.
pub fn build_auth_token_set(
    whitelisted_auth_tokens: Option<Vec<String>>,
) -> HashSet<MetadataValue<Ascii>> {
    whitelisted_auth_tokens
        .unwrap_or_default()
        .into_iter()
        .map(|token| token.parse::<MetadataValue<Ascii>>())
        .filter_map(Result::ok)
        .collect::<HashSet<_>>()
}
