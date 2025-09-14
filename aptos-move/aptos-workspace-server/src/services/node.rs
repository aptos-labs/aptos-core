// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{common::IP_LOCAL_HOST, no_panic_println};
use anyhow::{bail, Result};
use aptos_config::config::{NodeConfig, TableInfoServiceMode};
use aptos_localnet::health_checker::HealthChecker;
use aptos_node::{load_node_config, start_and_report_ports};
use aptos_types::network_address::{NetworkAddress, Protocol};
use futures::channel::oneshot;
use rand::{rngs::StdRng, SeedableRng};
use std::{future::Future, path::Path, thread, time::Duration};
use url::Url;

/// Sets all ports in the node config to zero so the OS can assign them random ones.
fn zero_all_ports(config: &mut NodeConfig) {
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

/// Returns the URL for connecting to the indexer grpc service.
///
/// Note: This can only be used by clients running directly on the host machine,
///       not from within a docker container.
pub fn get_data_service_url(indexer_grpc_port: u16) -> Url {
    Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, indexer_grpc_port)).unwrap()
}

/// Starts a local node and returns three futures:
/// - A future for the node API, which resolves to the port number once the service is fully up.
/// - A future for the indexer gRPC, which resolves to the port number once the service is fully up.
/// - A future that resolves when the node stops, which should not normally happen unless there is
///   an error.
pub fn start_node(
    test_dir: &Path,
) -> Result<(
    impl Future<Output = Result<u16>> + use<>,
    impl Future<Output = Result<u16>> + use<>,
    impl Future<Output = Result<()>> + use<>,
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
        no_panic_println!("Starting node..");

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

    let fut_node_finish = async move {
        // Note: we cannot join the thread here because that will cause the future to block,
        //       preventing the runtime from existing.
        loop {
            if node_thread_handle.is_finished() {
                bail!("node finished unexpectedly");
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    };

    let fut_api = async move {
        let api_port = api_port_rx.await?;

        let api_health_checker = HealthChecker::NodeApi(
            Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, api_port)).unwrap(),
        );
        api_health_checker.wait(None).await?;

        no_panic_println!(
            "Node API is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST,
            api_port
        );

        Ok(api_port)
    };

    let fut_indexer_grpc = async move {
        let indexer_grpc_port = indexer_grpc_port_rx.await?;

        let indexer_grpc_health_checker =
            HealthChecker::DataServiceGrpc(get_data_service_url(indexer_grpc_port));

        indexer_grpc_health_checker.wait(None).await?;
        no_panic_println!(
            "Transaction stream is ready. Endpoint: http://{}:{}/",
            IP_LOCAL_HOST,
            indexer_grpc_port
        );

        Ok(indexer_grpc_port)
    };

    Ok((fut_api, fut_indexer_grpc, fut_node_finish))
}
