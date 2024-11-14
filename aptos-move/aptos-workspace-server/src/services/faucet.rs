// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::IP_LOCAL_HOST;
use anyhow::{anyhow, Context, Result};
use aptos::node::local_testnet::HealthChecker;
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig};
use futures::channel::oneshot;
use std::{future::Future, path::PathBuf, sync::Arc};
use url::Url;

/// Starts the faucet service and returns two futures.
/// 1. A future that resolves to the port used, once the faucet service is fully up.
/// 2. A future that resolves, when the service stops.
pub fn start_faucet(
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

        println!("Starting faucet..");

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
            .map_err(|err| anyhow!("failed to join task handle: {}", err))?
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
