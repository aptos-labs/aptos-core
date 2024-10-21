// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos::node::local_testnet::HealthChecker;
use aptos_config::config::{NodeConfig, TableInfoServiceMode};
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig};
use aptos_node::{load_node_config, start_and_report_ports};
use aptos_types::network_address::{NetworkAddress, Protocol};
use futures::{channel::oneshot, future::Shared, FutureExt};
use rand::{rngs::StdRng, SeedableRng};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
    path::Path,
    sync::Arc,
    thread,
};
use url::Url;

const IP_LOCAL_HOST: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Converts a future into a shared one by putting the error into an Arc.
fn make_shared<F, T, E>(fut: F) -> Shared<impl Future<Output = Result<T, Arc<E>>>>
where
    T: Clone,
    F: Future<Output = Result<T, E>>,
{
    fut.map(|r| r.map_err(|err| Arc::new(err))).shared()
}

/// Sets all ports in the node config to zero so the OS can assign them random ones.
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

/// Starts a local node and returns two futures:
/// 1. A future for the node API, which resolves to the port number once the service is fully up.
/// 2. A future for the indexer gRPC, which resolves to the port number once the service is fully up.
fn start_node(
    test_dir: &Path,
) -> Result<(
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<u16>>,
)> {
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

    node_config.indexer_table_info.table_info_service_mode = TableInfoServiceMode::IndexingOnly;

    node_config.api.address.set_ip(IP_LOCAL_HOST);
    node_config.indexer_grpc.address.set_ip(IP_LOCAL_HOST);

    node_config.admin_service.address = IP_LOCAL_HOST.to_string();
    node_config.inspection_service.address = IP_LOCAL_HOST.to_string();

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

    let node_thread_handle = thread::spawn(run_node);

    let fut_node_finish = async {
        let join_handle = tokio::task::spawn_blocking(move || -> Result<()> {
            node_thread_handle
                .join()
                .map_err(|_err| anyhow!("failed to wait for node thread"))?
        });

        join_handle
            .await
            .map_err(|err| anyhow!("failed to join node task: {}", err))?
    };

    let fut_api = async move {
        let api_port = api_port_rx.await?;

        let api_health_checker = HealthChecker::NodeApi(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, api_port)).unwrap(),
        );
        api_health_checker.wait(None).await?;

        println!(
            "Node API is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST, api_port
        );

        Ok(api_port)
    };

    let fut_indexer_grpc = async move {
        let indexer_grpc_port = indexer_grpc_port_rx.await?;

        let indexer_grpc_health_checker = HealthChecker::DataServiceGrpc(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, indexer_grpc_port)).unwrap(),
        );

        indexer_grpc_health_checker.wait(None).await?;
        println!(
            "Transaction stream is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST, indexer_grpc_port
        );

        Ok(indexer_grpc_port)
    };

    Ok((fut_api, fut_indexer_grpc, fut_node_finish))
}

/// Starts the faucet service and returns two futures.
/// 1. A future that resolves to the port used, once the faucet service is fully up.
/// 2. A future that resolves, when the service stops.
fn start_faucet(
    test_dir: PathBuf,
    fut_node_api: impl Future<Output = Result<u16, Arc<anyhow::Error>>> + Send + 'static,
    fut_indexer_grpc: impl Future<Output = Result<u16, Arc<anyhow::Error>>> + Send + 'static,
) -> (
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>> + 'static,
) {
    let (faucet_port_tx, faucet_port_rx) = oneshot::channel();

    let handle_faucet = tokio::spawn(async move {
        let api_port = fut_node_api
            .await
            .map_err(anyhow::Error::msg)
            .context("failed to start faucet: node api did not start successfully")?;

        fut_indexer_grpc
            .await
            .map_err(anyhow::Error::msg)
            .context("failed to start faucet: indexer grpc did not start successfully")?;

        let faucet_run_config = RunConfig::build_for_cli(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, api_port)).unwrap(),
            IP_LOCAL_HOST.to_string(),
            0,
            FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
            false,
            None,
        );

        faucet_run_config.run_and_report_port(faucet_port_tx).await
    });

    let fut_api = async move {
        let api_port = api_port_rx.await?;

        let api_health_checker = HealthChecker::NodeApi(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, api_port)).unwrap(),
        );
        api_health_checker.wait(None).await?;

        println!(
            "Node API is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST, api_port
        );

        Ok(api_port)
    };

    let fut_indexer_grpc = async move {
        let indexer_grpc_port = indexer_grpc_port_rx.await?;

        let indexer_grpc_health_checker = HealthChecker::DataServiceGrpc(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, indexer_grpc_port)).unwrap(),
        );

        indexer_grpc_health_checker.wait(None).await?;
        println!(
            "Transaction stream is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST, indexer_grpc_port
        );

        Ok(indexer_grpc_port)
    };

    Ok((fut_api, fut_indexer_grpc))
}

/// Starts the faucet service.
/// The port used will be returned once the service is fully up.
async fn start_faucet(
    test_dir: &Path,
    fut_node_api: impl Future<Output = Result<u16, Arc<anyhow::Error>>>,
    fut_indexer_grpc: impl Future<Output = Result<u16, Arc<anyhow::Error>>>,
) -> Result<u16> {
    let api_port = fut_node_api
        .await
        .map_err(anyhow::Error::msg)
        .context("failed to start faucet: node api did not start successfully")?;

    fut_indexer_grpc
        .await
        .map_err(anyhow::Error::msg)
        .context("failed to start faucet: indexer grpc did not start successfully")?;

    let faucet_run_config = RunConfig::build_for_cli(
        Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, api_port)).unwrap(),
        IP_LOCAL_HOST.to_string(),
        0,
        FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
        false,
        None,
    );

    let (faucet_port_tx, faucet_port_rx) = oneshot::channel();
    tokio::spawn(async move {
        if let Err(err) = faucet_run_config.run_and_report_port(faucet_port_tx).await {
            eprintln!("Faucet exited with error {:?}", err)
        }
    });

    let faucet_port = faucet_port_rx
        .await
        .context("failed to receive faucet port")?;

    let faucet_health_checker =
        HealthChecker::http_checker_from_port(faucet_port, "Faucet".to_string());
    faucet_health_checker.wait(None).await?;

    println!(
        "Faucet is ready. Endpoint: http://{}:{}",
        IP_LOCAL_HOST, faucet_port
    );

    Ok(faucet_port)
}

async fn start_all_services(test_dir: &Path) -> Result<()> {
    let (fut_node_api, fut_indexer_grpc) = start_node(test_dir)?;

    let fut_node_api = make_shared(fut_node_api);
    let fut_indexer_grpc = make_shared(fut_indexer_grpc);
    let fut_faucet = start_faucet(test_dir, fut_node_api.clone(), fut_indexer_grpc.clone());

    let (res_node_api, res_indexer_grpc, res_faucet) =
        tokio::join!(fut_node_api, fut_indexer_grpc, fut_faucet);

    res_node_api
        .map_err(anyhow::Error::msg)
        .context("failed to start node api")?;
    res_indexer_grpc
        .map_err(anyhow::Error::msg)
        .context("failed to start node api")?;
    res_faucet.context("failed to start faucet")?;

    println!(
        "Indexer API is ready. Endpoint: http://{}:0/",
        IP_LOCAL_HOST
    );

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = tempfile::tempdir()?;

    println!("Test directory: {}", test_dir.path().display());

    start_all_services(test_dir.path()).await?;

    Ok(())
}
