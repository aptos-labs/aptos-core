// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos::node::local_testnet::HealthChecker;
use aptos_config::config::NodeConfig;
use aptos_node::{load_node_config, setup_environment_and_start_node_ex};
use futures::channel::oneshot;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use url::Url;

async fn spawn_node(test_dir: &Path) -> Result<()> {
    let rng = StdRng::from_entropy();

    let mut node_config = load_node_config(
        &None,
        &None,
        test_dir,
        false,
        false,
        false,
        aptos_cached_packages::head_release_bundle(),
        rng,
    )?;

    //node_config.zero_ports();
    node_config.indexer_grpc.enabled = true;
    node_config.indexer_grpc.use_data_service_interface = true;
    node_config.storage.enable_indexer = true;

    node_config
        .api
        .address
        .set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    //node_config.api.address.set_port(10000);
    node_config
        .indexer_grpc
        .address
        .set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    //node_config.indexer_grpc.address.set_port(10001);

    node_config.admin_service.address = "127.0.0.1".to_string();
    node_config.inspection_service.address = "127.0.0.1".to_string();

    let (api_port_tx, api_port_rx) = oneshot::channel();
    let (indexer_grpc_port_tx, indexer_grpc_port_rx) = oneshot::channel();

    let run_node = {
        let node_config = node_config.clone();
        move || -> Result<()> {
            aptos_node_identity::init(node_config.get_peer_id()).unwrap();

            // TODO: panic handler?

            setup_environment_and_start_node_ex(
                node_config,
                None,
                None,
                Some(api_port_tx),
                Some(indexer_grpc_port_tx),
            )?;

            // TODO: why is this needed?
            let term = Arc::new(AtomicBool::new(false));
            while !term.load(Ordering::Acquire) {
                thread::park();
            }

            Ok(())
        }
    };

    let node_thread_handle = thread::spawn(move || {
        let res = run_node();

        if let Err(err) = res {
            println!("Node stopped unexpectedly {:?}", err);
        }
    });

    let api_port = api_port_rx.await?;
    let indexer_grpc_port = indexer_grpc_port_rx.await?;

    println!("Node API port: {}", api_port);
    println!("Indexer GRPC port: {}", indexer_grpc_port);

    println!(
        "{}",
        format!("http://{}:{}", node_config.api.address, api_port)
    );
    println!(
        "{}",
        format!(
            "http://{}:{}",
            node_config.indexer_grpc.address, indexer_grpc_port
        )
    );

    let api_health_checker = HealthChecker::NodeApi(
        Url::parse(&format!(
            "http://{}:{}",
            node_config.api.address.ip(),
            api_port
        ))
        .unwrap(),
    );
    let indexer_grpc_health_checker = HealthChecker::DataServiceGrpc(
        Url::parse(&format!(
            "http://{}:{}",
            node_config.indexer_grpc.address.ip(),
            indexer_grpc_port
        ))
        .unwrap(),
    );

    api_health_checker.wait(None).await?;
    println!("Node API up");

    indexer_grpc_health_checker.wait(None).await?;
    println!("Indexer GRPC up");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = tempfile::tempdir()?;

    println!("Test directory: {}", test_dir.path().display());

    spawn_node(test_dir.path()).await?;

    loop {}

    Ok(())
}
