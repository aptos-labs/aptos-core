// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::server::ApiService;
use anyhow::{Context as AnyhowContext, Result};
use aptos_api::context::Context;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_protos::api::v2::{api_server::ApiServer, FILE_DESCRIPTOR_SET};
use aptos_storage_interface::DbReader;
use aptos_types::chain_id::ChainId;
use std::{
    net::ToSocketAddrs,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::runtime::{Builder, Runtime};
use tonic::transport::Server;

// Creates a runtime which runs the tonic server.
// TODO: Figure out what is idiomatic for ports / paths, perhaps something like:
// - /v2/grpc for the GRPC endpoint
// - /v2/everything_else for all the standard RESTful versions of the RPC functions
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Result<Runtime> {
    // TODO: Add config option to enable API v2.

    let runtime = Builder::new_multi_thread()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("api-v2-{}", id)
        })
        .disable_lifo_slot()
        .enable_all()
        .build()
        .context("[apiv2] Failed to create runtime")?;

    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Arc::new(Context::new(chain_id, db, mp_sender, node_config));
        let service = ApiService { context };
        let server = ApiServer::new(service);

        let reflection_server = tonic_reflection::server::Builder::configure()
            // This file descriptor set is the magic sauce that lets the reflection
            // service reflect the API.
            .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
            .build()
            .unwrap();

        Server::builder()
            .add_service(reflection_server)
            .add_service(server)
            .serve("0.0.0.0:60001".to_socket_addrs().unwrap().next().unwrap())
            .await
            .unwrap();
        info!("[api] Started GRPC server at 50051");
    });

    Ok(runtime)
}
