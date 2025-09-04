// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accounts::AccountsApi,
    basic::BasicApi,
    blocks::BlocksApi,
    check_size::PostSizeLimit,
    context::Context,
    error_converter::convert_error,
    events::EventsApi,
    index::IndexApi,
    log::middleware_log,
    set_failpoints,
    spec::{spec_endpoint_json, spec_endpoint_yaml},
    state::StateApi,
    transactions::TransactionsApi,
    view_function::ViewFunctionApi,
};
use anyhow::{anyhow, Context as AnyhowContext};
use velor_config::config::{ApiConfig, NodeConfig};
use velor_logger::info;
use velor_mempool::MempoolClientSender;
use velor_storage_interface::DbReader;
use velor_types::{chain_id::ChainId, indexer::indexer_db_reader::IndexerReader};
use futures::channel::oneshot;
use poem::{
    handler,
    http::Method,
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::{Compression, Cors},
    web::Html,
    EndpointExt, Route, Server,
};
use poem_openapi::{ContactObject, LicenseObject, OpenApiService};
use std::{net::SocketAddr, sync::Arc};
use tokio::runtime::{Handle, Runtime};

const VERSION: &str = include_str!("../doc/.version");

/// Create a runtime and attach the Poem webserver to it.
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
    indexer_reader: Option<Arc<dyn IndexerReader>>,
    port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<Runtime> {
    let max_runtime_workers = get_max_runtime_workers(&config.api);
    let runtime = velor_runtimes::spawn_named_runtime("api".into(), Some(max_runtime_workers));

    let context = Context::new(chain_id, db, mp_sender, config.clone(), indexer_reader);

    attach_poem_to_runtime(runtime.handle(), context.clone(), config, false, port_tx)
        .context("Failed to attach poem to runtime")?;

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

// TODOs regarding spec generation:
// TODO: https://github.com/velor-chain/velor-core/issues/2280
// TODO: https://github.com/poem-web/poem/issues/321
// TODO: https://github.com/poem-web/poem/issues/332
// TODO: https://github.com/poem-web/poem/issues/333

/// Generate the top level API service
pub fn get_api_service(
    context: Arc<Context>,
) -> OpenApiService<
    (
        AccountsApi,
        BasicApi,
        BlocksApi,
        EventsApi,
        IndexApi,
        StateApi,
        TransactionsApi,
        ViewFunctionApi,
    ),
    (),
> {
    // These APIs get merged.
    let apis = (
        AccountsApi {
            context: context.clone(),
        },
        BasicApi {
            context: context.clone(),
        },
        BlocksApi {
            context: context.clone(),
        },
        EventsApi {
            context: context.clone(),
        },
        IndexApi {
            context: context.clone(),
        },
        StateApi {
            context: context.clone(),
        },
        TransactionsApi {
            context: context.clone(),
        },
        ViewFunctionApi { context },
    );

    let version = VERSION.to_string();
    let license =
        LicenseObject::new("Apache 2.0").url("https://www.apache.org/licenses/LICENSE-2.0.html");
    let contact = ContactObject::new()
        .name("Velor Labs")
        .url("https://github.com/velor-chain/velor-core");

    OpenApiService::new(apis, "Velor Node API", version.trim())
        .server("/v1")
        .description("The Velor Node API is a RESTful API for client applications to interact with the Velor blockchain.")
        .license(license)
        .contact(contact)
        .external_document("https://github.com/velor-chain/velor-core")
}

/// Returns address it is running at.
pub fn attach_poem_to_runtime(
    runtime_handle: &Handle,
    context: Context,
    config: &NodeConfig,
    random_port: bool,
    port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<SocketAddr> {
    let context = Arc::new(context);

    let size_limit = context.content_length_limit();

    let api_service = get_api_service(context.clone());

    let spec_json = spec_endpoint_json(&api_service);
    let spec_yaml = spec_endpoint_yaml(&api_service);

    let config = config.clone();
    let mut address = config.api.address;

    if random_port {
        // Let the OS assign an open port.
        address.set_port(0);
    }

    // Build listener with or without TLS
    let listener = match (&config.api.tls_cert_path, &config.api.tls_key_path) {
        (Some(tls_cert_path), Some(tls_key_path)) => {
            info!("Using TLS for API");
            let cert = std::fs::read_to_string(tls_cert_path).context(format!(
                "Failed to read TLS cert from path: {}",
                tls_cert_path
            ))?;
            let key = std::fs::read_to_string(tls_key_path).context(format!(
                "Failed to read TLS key from path: {}",
                tls_key_path
            ))?;
            let rustls_certificate = RustlsCertificate::new().cert(cert).key(key);
            let rustls_config = RustlsConfig::new().fallback(rustls_certificate);
            TcpListener::bind(address).rustls(rustls_config).boxed()
        },
        _ => {
            info!("Not using TLS for API");
            TcpListener::bind(address).boxed()
        },
    };

    let acceptor = tokio::task::block_in_place(move || {
        runtime_handle
            .block_on(async move { listener.into_acceptor().await })
            .with_context(|| format!("Failed to bind Poem to address: {}", address))
    })?;

    let actual_address = &acceptor.local_addr()[0];
    let actual_address = *actual_address
        .as_socket_addr()
        .context("Failed to get socket addr from local addr for Poem webserver")?;

    if let Some(port_tx) = port_tx {
        port_tx
            .send(actual_address.port())
            .map_err(|_| anyhow!("Failed to send port"))?;
    }

    runtime_handle.spawn(async move {
        let cors = Cors::new()
            // To allow browsers to use cookies (for cookie-based sticky
            // routing in the LB) we must enable this:
            // https://stackoverflow.com/a/24689738/3846032
            .allow_credentials(true)
            .allow_methods(vec![Method::GET, Method::POST]);

        // Build routes for the API
        let route = Route::new()
            .at("/", poem::get(root_handler))
            .nest(
                "/v1",
                Route::new()
                    .nest("/", api_service)
                    .at("/spec.json", poem::get(spec_json))
                    .at("/spec.yaml", poem::get(spec_yaml))
                    // TODO: We add this manually outside of the OpenAPI spec for now.
                    // https://github.com/poem-web/poem/issues/364
                    .at(
                        "/set_failpoint",
                        poem::get(set_failpoints::set_failpoint_poem).data(context.clone()),
                    ),
            )
            .with(cors)
            .with_if(config.api.compression_enabled, Compression::new())
            .with(PostSizeLimit::new(size_limit))
            // NOTE: Make sure to keep this after all the `with` middleware.
            .catch_all_error(convert_error)
            .around(middleware_log);
        Server::new_with_acceptor(acceptor)
            .run(route)
            .await
            .map_err(anyhow::Error::msg)
    });

    info!("API server is running at {}", actual_address);

    Ok(actual_address)
}

#[handler]
async fn root_handler() -> Html<&'static str> {
    let response = "<html>
<head>
    <title>Velor Node API</title>
</head>
<body>
    <p>
        Welcome! The latest node API can be found at <a href=\"/v1\">/v1<a/>.
    </p>
    <p>
        Learn more about the v1 node API here: <a href=\"/v1/spec\">/v1/spec<a/>.
    </p>
</body>
</html>";
    Html(response)
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
    use velor_api_test_context::{new_test_context, TestContext};
    use velor_config::config::{ApiConfig, NodeConfig};
    use velor_types::chain_id::ChainId;
    use std::time::Duration;

    // TODO: Unignore this when I figure out why this only works when being
    // run alone (it fails when run with other tests).
    // https://github.com/velor-chain/velor-core/issues/2977
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
