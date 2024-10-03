// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos::node::local_testnet::HealthChecker;
use aptos_config::config::NodeConfig;
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig};
use aptos_node::{load_node_config, start_and_report_ports};
use aptos_types::network_address::{NetworkAddress, Protocol};
use futures::channel::oneshot;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::Path,
    thread,
    time::Duration,
};
use url::Url;

pub fn zero_all_ports(config: &mut NodeConfig) {
    // TODO: Double check if all ports are covered.

    config.admin_service.port = 0;
    config.api.address.set_port(0);
    config.inspection_service.port = 0;
    config.storage.backup_service_address.set_port(0);
    config.indexer_grpc.address.set_port(0);

    if let Some(network) = config.validator_network.as_mut() {
        network.listen_address = NetworkAddress::from_protocols(vec![
            Protocol::Ip4("0.0.0.0".parse().unwrap()),
            Protocol::Tcp(0),
        ])
        .unwrap();
    }
    for network in config.full_node_networks.iter_mut() {
        network.listen_address = NetworkAddress::from_protocols(vec![
            Protocol::Ip4("0.0.0.0".parse().unwrap()),
            Protocol::Tcp(0),
        ])
        .unwrap();
    }
}

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

    zero_all_ports(&mut node_config);
    node_config.indexer_grpc.enabled = true;
    node_config.indexer_grpc.use_data_service_interface = true;
    node_config.storage.enable_indexer = true;

    node_config
        .api
        .address
        .set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    node_config
        .indexer_grpc
        .address
        .set_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    node_config.admin_service.address = "127.0.0.1".to_string();
    node_config.inspection_service.address = "127.0.0.1".to_string();

    let (api_port_tx, api_port_rx) = oneshot::channel();
    let (indexer_grpc_port_tx, indexer_grpc_port_rx) = oneshot::channel();

    let run_node = {
        let test_dir = test_dir.to_owned();
        let node_config = node_config.clone();
        move || -> Result<()> {
            start_and_report_ports(
                node_config,
                Some(test_dir.join("validator.log")),
                false,
                Some(api_port_tx),
                Some(indexer_grpc_port_tx),
            )
        }
    };

    let _node_thread_handle = thread::spawn(move || {
        let res = run_node();

        if let Err(err) = res {
            println!("Node stopped unexpectedly {:?}", err);
        }
    });

    let api_port = api_port_rx.await?;
    let indexer_grpc_port = indexer_grpc_port_rx.await?;

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
    eprintln!(
        "Node API is ready. Endpoint: http://127.0.0.1:{}/",
        api_port
    );

    indexer_grpc_health_checker.wait(None).await?;
    eprintln!(
        "Transaction stream is ready. Endpoint: http://127.0.0.1:{}/",
        indexer_grpc_port
    );

    let faucet_run_config = RunConfig::build_for_cli(
        Url::parse(&format!(
            "http://{}:{}",
            node_config.api.address.ip(),
            api_port
        ))
        .unwrap(),
        "127.0.0.1".to_string(),
        0,
        FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
        false,
        None,
    );

    let (faucet_port_tx, faucet_port_rx) = oneshot::channel();
    tokio::spawn(faucet_run_config.run_and_report_port(faucet_port_tx));

    let faucet_port = faucet_port_rx.await?;

    let faucet_health_checker =
        HealthChecker::http_checker_from_port(faucet_port, "Faucet".to_string());
    faucet_health_checker.wait(None).await?;
    eprintln!(
        "Faucet is ready. Endpoint: http://127.0.0.1:{}",
        faucet_port
    );

    eprintln!("Indexer API is ready. Endpoint: http://127.0.0.1:0/");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = tempfile::tempdir()?;

    println!("Test directory: {}", test_dir.path().display());

    spawn_node(test_dir.path()).await?;

    loop {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
