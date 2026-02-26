// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{context::Context, transactions::TransactionsApi};
use anyhow::Context as AnyhowContext;
use aptos_config::config::{ApiConfig, NodeConfig};
use aptos_mempool::MempoolClientSender;
use aptos_storage_interface::DbReader;
use aptos_types::{chain_id::ChainId, indexer::indexer_db_reader::IndexerReader};
use futures::channel::oneshot;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create a runtime and attach the Axum webserver to it.
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    indexer_reader: Option<Arc<dyn IndexerReader>>,
    port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<Runtime> {
    let max_runtime_workers = get_max_runtime_workers(&config.api);
    let runtime = aptos_runtimes::spawn_named_runtime("api".into(), Some(max_runtime_workers));

    let context = Context::new(chain_id, db, mp_sender, config.clone(), indexer_reader);

    crate::runtime_axum::attach_axum_to_runtime(
        runtime.handle(),
        context.clone(),
        config,
        false,
        port_tx,
    )
    .context("Failed to attach axum to runtime")?;

    let context_cloned = context.clone();
    if let Some(period_ms) = config.api.periodic_gas_estimation_ms {
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(period_ms));
            loop {
                interval.tick().await;
                let context_cloned = context_cloned.clone();
                tokio::task::spawn_blocking(move || {
                    if let Ok(latest_ledger_info) =
                        context_cloned.get_latest_ledger_info::<crate::response::BasicError>()
                    {
                        if let Ok(gas_estimation) = context_cloned
                            .estimate_gas_price::<crate::response::BasicError>(&latest_ledger_info)
                        {
                            TransactionsApi::log_gas_estimation(&gas_estimation);
                        }
                    }
                })
                .await
                .unwrap_or(());
            }
        });
    }

    let context_cloned = context.clone();
    if let Some(period_sec) = config.api.periodic_function_stats_sec {
        runtime.spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(period_sec));
            loop {
                interval.tick().await;
                let context_cloned = context_cloned.clone();
                tokio::task::spawn_blocking(move || {
                    context_cloned.view_function_stats().log_and_clear();
                    context_cloned.simulate_txn_stats().log_and_clear();
                })
                .await
                .unwrap_or(());
            }
        });
    }

    Ok(runtime)
}

/// Returns the maximum number of runtime workers to be given to the
/// API runtime. Defaults to 2 * number of CPU cores if not specified
/// via the given config.
fn get_max_runtime_workers(api_config: &ApiConfig) -> usize {
    api_config
        .max_runtime_workers
        .unwrap_or_else(|| num_cpus::get() * api_config.runtime_worker_multiplier)
}

#[cfg(test)]
mod tests {
    use super::bootstrap;
    use crate::runtime::get_max_runtime_workers;
    use aptos_api_test_context::{new_test_context, TestContext};
    use aptos_config::config::{ApiConfig, NodeConfig};
    use aptos_types::chain_id::ChainId;
    use std::time::Duration;

    // TODO: Unignore this when I figure out why this only works when being
    // run alone (it fails when run with other tests).
    // https://github.com/aptos-labs/aptos-core/issues/2977
    #[ignore]
    #[test]
    fn test_bootstrap_jsonprc_and_api_configured_at_different_port() {
        let mut cfg = NodeConfig::default();
        cfg.randomize_ports();
        bootstrap_with_config(cfg);
    }

    #[test]
    fn test_max_runtime_workers() {
        // Specify the number of workers for the runtime
        let max_runtime_workers = 100;
        let api_config = ApiConfig {
            max_runtime_workers: Some(max_runtime_workers),
            ..Default::default()
        };

        // Verify the correct number of workers is returned
        assert_eq!(get_max_runtime_workers(&api_config), max_runtime_workers);

        // Don't specify the number of workers for the runtime
        let api_config = ApiConfig {
            max_runtime_workers: None,
            ..Default::default()
        };

        // Verify the correct number of workers is returned
        assert_eq!(
            get_max_runtime_workers(&api_config),
            num_cpus::get() * api_config.runtime_worker_multiplier
        );

        // Update the multiplier
        let api_config = ApiConfig {
            runtime_worker_multiplier: 10,
            ..Default::default()
        };

        // Verify the correct number of workers is returned
        assert_eq!(
            get_max_runtime_workers(&api_config),
            num_cpus::get() * api_config.runtime_worker_multiplier
        );
    }

    pub fn bootstrap_with_config(cfg: NodeConfig) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let context = runtime.block_on(new_test_context_async(
            "test_bootstrap_jsonprc_and_api_configured_at_different_port".to_string(),
        ));
        let ret = bootstrap(
            &cfg,
            ChainId::test(),
            context.db.clone(),
            context.mempool.ac_client.clone(),
            None,
            None,
        );
        assert!(ret.is_ok());

        assert_web_server(cfg.api.address.port());
    }

    pub fn assert_web_server(port: u16) {
        let base_url = format!("http://localhost:{}/v1", port);
        let client = reqwest::blocking::Client::new();
        // first call have retry to ensure the server is ready to serve
        let api_resp = with_retry(|| Ok(client.get(&base_url).send()?)).unwrap();
        assert_eq!(api_resp.status(), 200);
        let healthy_check_resp = client
            .get(format!("{}/-/healthy", base_url))
            .send()
            .unwrap();
        assert_eq!(healthy_check_resp.status(), 200);
    }

    fn with_retry<F>(f: F) -> anyhow::Result<reqwest::blocking::Response>
    where
        F: Fn() -> anyhow::Result<reqwest::blocking::Response>,
    {
        let mut remaining_attempts = 60;
        loop {
            match f() {
                Ok(r) => return Ok(r),
                Err(_) if remaining_attempts > 0 => {
                    remaining_attempts -= 1;
                    std::thread::sleep(Duration::from_millis(100));
                },
                Err(error) => return Err(error),
            }
        }
    }

    pub async fn new_test_context_async(test_name: String) -> TestContext {
        new_test_context(test_name, NodeConfig::default(), false)
    }
}
