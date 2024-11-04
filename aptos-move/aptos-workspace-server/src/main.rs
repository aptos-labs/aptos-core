// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use aptos::node::local_testnet::{docker, HealthChecker};
use aptos_config::config::{NodeConfig, TableInfoServiceMode};
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig};
use aptos_node::{load_node_config, start_and_report_ports};
use aptos_types::network_address::{NetworkAddress, Protocol};
use bollard::{
    container::{
        CreateContainerOptions, InspectContainerOptions, StartContainerOptions,
        WaitContainerOptions,
    },
    network::CreateNetworkOptions,
    secret::{ContainerInspectResponse, HostConfig, PortBinding},
};
use futures::{channel::oneshot, future::Shared, FutureExt, TryStreamExt};
use maplit::hashmap;
use rand::{rngs::StdRng, SeedableRng};
use std::{
    future::Future,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};
use url::Url;
use uuid::Uuid;

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

/// Starts a local node and returns three futures:
/// 1. A future for the node API, which resolves to the port number once the service is fully up.
/// 2. A future for the indexer gRPC, which resolves to the port number once the service is fully up.
/// 3. A final future that resolves when the node stops.
fn start_node(
    test_dir: &Path,
) -> Result<(
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>>,
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

    let fut_faucet_finish = async move {
        handle_faucet
            .await
            .map_err(|err| anyhow!("failed to join handle task: {}", err))?
    };

    let fut_faucet_port = async move {
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
    };

    (fut_faucet_port, fut_faucet_finish)
}

const POSTGRES_DEFAULT_PORT: u16 = 5432;
const DATA_PATH_IN_CONTAINER: &str = "/var/lib/mydata";
const POSTGRES_IMAGE: &str = "postgres:14.11";

fn get_postgres_assigned_port(container_info: &ContainerInspectResponse) -> Option<u16> {
    if let Some(port_bindings) = container_info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
    {
        if let Some(Some(bindings)) = port_bindings.get(&format!("{}/tcp", POSTGRES_DEFAULT_PORT)) {
            if let Some(binding) = bindings.first() {
                return binding
                    .host_port
                    .as_ref()
                    .and_then(|port| port.parse::<u16>().ok());
            }
        }
    }
    None
}

fn start_postgres(
    instance_id: Uuid,
) -> Result<(
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>>,
)> {
    let (postgres_container_id_tx, postgres_container_id_rx) = oneshot::channel();

    let handle_postgres = tokio::spawn(async move {
        let docker = docker::get_docker().await?;

        let volume_name = format!("aptos-workspace-{}", instance_id);
        docker
            .create_volume(bollard::volume::CreateVolumeOptions {
                name: volume_name.as_str(),
                ..Default::default()
            })
            .await
            .context("failed to create volume for postgres")?;

        let network_name = format!("aptos-workspace-{}", instance_id);
        docker
            .create_network(CreateNetworkOptions {
                name: network_name.as_str(),
                internal: false,
                check_duplicate: true,
                ..Default::default()
            })
            .await
            .context("failed to create network for postgres")?;

        let host_config = Some(HostConfig {
            // Bind the container to the network we created in the pre_run. This does
            // not prevent the binary in the container from exposing itself to the host
            // on 127.0.0.1. See more here: https://stackoverflow.com/a/77432636/3846032.
            network_mode: Some(network_name.clone()),
            port_bindings: Some(hashmap! {
                POSTGRES_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: None,
                }]),
            }),
            // Mount the volume in to the container. We use a volume because they are
            // more performant and easier to manage via the Docker API.
            binds: Some(vec![format!("{}:{}", volume_name, DATA_PATH_IN_CONTAINER,)]),
            ..Default::default()
        });

        let config = bollard::container::Config {
            image: Some(POSTGRES_IMAGE.to_string()),
            // We set this to false so the container keeps running after the CLI
            // shuts down by default. We manually kill the container if applicable,
            // for example if the user set --force-restart.
            tty: Some(false),
            exposed_ports: Some(hashmap! {POSTGRES_DEFAULT_PORT.to_string() => hashmap!{}}),
            host_config,
            env: Some(vec![
                // We run postgres without any auth + no password.
                "POSTGRES_HOST_AUTH_METHOD=trust".to_string(),
                format!("POSTGRES_USER={}", "postgres"),
                format!("POSTGRES_DB={}", "local-testnet"),
                // This tells where postgres to store the DB data on disk. This is the
                // directory inside the container that is mounted from the host system.
                format!("PGDATA={}", DATA_PATH_IN_CONTAINER),
            ]),
            cmd: Some(
                vec![
                    "postgres",
                    "-c",
                    // The default is 100 as of Postgres 14.11. Given the localnet
                    // can be composed of many different processors all with their own
                    // connection pools, 100 is insufficient.
                    "max_connections=200",
                    "-c",
                    // The default is 128MB as of Postgres 14.11. We 2x that value to
                    // match the fact that we 2x'd max_connections.
                    "shared_buffers=256MB",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            ),
            ..Default::default()
        };

        let options = Some(CreateContainerOptions {
            name: format!("aptos-workspace-{}-postgres", instance_id),
            ..Default::default()
        });

        let container_id = docker
            .create_container(options, config)
            .await
            .context("failed to create postgres container")?
            .id;

        docker
            .start_container(&container_id, None::<StartContainerOptions<&str>>)
            .await
            .context("failed to start postgres container")?;

        let container_info = docker
            .inspect_container(&container_id, Some(InspectContainerOptions::default()))
            .await
            .context("failed to inspect postgres container")?;

        postgres_container_id_tx
            .send(container_id)
            .map_err(|_port| anyhow!("failed to send postgres container id"))?;

        let postgres_port = get_postgres_assigned_port(&container_info)
            .ok_or_else(|| anyhow!("failed to get postgres port"))?;

        println!(
            "Postgres is ready. Endpoint: http://{}:{}",
            IP_LOCAL_HOST, postgres_port
        );

        // TODO: health checker
        let health_checker = HealthChecker::Postgres(format!(
            "postgres://{}@{}:{}/{}",
            "postgres",
            IP_LOCAL_HOST.to_string(),
            postgres_port,
            "local-testnet"
        ));
        health_checker.wait(None).await?;

        Ok(postgres_port)
    });

    let fut_postgres_port = async move {
        handle_postgres
            .await
            .map_err(|err| anyhow!("failed to join handle task: {}", err))?
    };

    let fut_postgres_finish = async move {
        let container_id = postgres_container_id_rx
            .await
            .context("failed to receive postgres container id")?;

        let docker = docker::get_docker().await?;

        // Wait for the container to stop (which it shouldn't).
        let _wait = docker
            .wait_container(
                &container_id,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("Failed to wait on postgres container")?;

        Ok(())
    };

    Ok((fut_postgres_port, fut_postgres_finish))
}

async fn start_all_services(test_dir: &Path) -> Result<()> {
    let instance_id = Uuid::new_v4();

    // Step 1: spawn all services.
    // Node
    let (fut_node_api, fut_indexer_grpc, fut_node_finish) = start_node(test_dir)?;

    let fut_node_api = make_shared(fut_node_api);
    let fut_indexer_grpc = make_shared(fut_indexer_grpc);

    // Faucet
    let (fut_faucet, fut_faucet_finish) = start_faucet(
        test_dir.to_owned(),
        fut_node_api.clone(),
        fut_indexer_grpc.clone(),
    );

    // Postgres
    let (fut_postgres, fut_postgres_finish) = start_postgres(instance_id)?;

    // Step 2: wait for all services to be up.
    let (res_node_api, res_indexer_grpc, res_faucet, res_postgres) =
        tokio::join!(fut_node_api, fut_indexer_grpc, fut_faucet, fut_postgres);

    res_node_api
        .map_err(anyhow::Error::msg)
        .context("failed to start node api")?;
    res_indexer_grpc
        .map_err(anyhow::Error::msg)
        .context("failed to start node api")?;
    res_faucet.context("failed to start faucet")?;
    res_postgres.context("failed to start postgres")?;

    println!(
        "Indexer API is ready. Endpoint: http://{}:0/",
        IP_LOCAL_HOST
    );

    println!("ALL SERVICES STARTED SUCCESSFULLY");

    // Step 3: wait for services to stop.
    tokio::pin!(fut_node_finish);
    tokio::pin!(fut_faucet_finish);
    tokio::pin!(fut_postgres_finish);

    let mut finished: u64 = 0;
    while finished < 3 {
        tokio::select! {
            res = &mut fut_node_finish => {
                if let Err(err) = res {
                    eprintln!("Node existed with error: {}", err);
                }
                finished += 1;
            }
            res = &mut fut_faucet_finish => {
                if let Err(err) = res {
                    eprintln!("Faucet existed with error: {}", err);
                }
                finished += 1;
            }
            res = &mut fut_postgres_finish => {
                if let Err(err) = res {
                    eprintln!("Postgres existed with error: {}", err);
                }
                finished += 1;
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let test_dir = tempfile::tempdir()?;

    println!("Test directory: {}", test_dir.path().display());

    start_all_services(test_dir.path()).await?;

    Ok(())
}
