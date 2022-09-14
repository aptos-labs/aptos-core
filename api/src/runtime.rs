// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{net::SocketAddr, sync::Arc};

use crate::{
    accounts::AccountsApi, basic::BasicApi, blocks::BlocksApi, check_size::PostSizeLimit,
    context::Context, error_converter::convert_error, events::EventsApi, index::IndexApi,
    log::middleware_log, set_failpoints, state::StateApi, transactions::TransactionsApi,
};
use anyhow::Context as AnyhowContext;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use poem::{
    http::{header, Method},
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::Cors,
    EndpointExt, Route, Server,
};
use poem_openapi::{ContactObject, LicenseObject, OpenApiService};
use std::sync::atomic::{AtomicUsize, Ordering};
use storage_interface::DbReader;
use tokio::runtime::{Builder, Handle, Runtime};

const VERSION: &str = include_str!("../doc/.version");

/// Create a runtime and attach the Poem webserver to it.
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> anyhow::Result<Runtime> {
    let runtime = Builder::new_multi_thread()
        .thread_name_fn(|| {
            static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
            let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
            format!("api-{}", id)
        })
        .disable_lifo_slot()
        .enable_all()
        .build()
        .context("[api] failed to create runtime")?;

    let context = Context::new(chain_id, db, mp_sender, config.clone());

    attach_poem_to_runtime(runtime.handle(), context, config, false)
        .context("Failed to attach poem to runtime")?;

    Ok(runtime)
}

// TODOs regarding spec generation:
// TODO: https://github.com/aptos-labs/aptos-core/issues/2280
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
        TransactionsApi { context },
    );

    let version = VERSION.to_string();
    let license =
        LicenseObject::new("Apache 2.0").url("https://www.apache.org/licenses/LICENSE-2.0.html");
    let contact = ContactObject::new()
        .name("Aptos Labs")
        .url("https://github.com/aptos-labs/aptos-core");

    OpenApiService::new(apis, "Aptos Node API", version.trim())
        .server("/v1")
        .description("The Aptos Node API is a RESTful API for client applications to interact with the Aptos blockchain.")
        .license(license)
        .contact(contact)
        .external_document("https://github.com/aptos-labs/aptos-core")
}

/// Returns address it is running at.
pub fn attach_poem_to_runtime(
    runtime_handle: &Handle,
    context: Context,
    config: &NodeConfig,
    random_port: bool,
) -> anyhow::Result<SocketAddr> {
    let context = Arc::new(context);

    let size_limit = context.content_length_limit();

    let api_service = get_api_service(context.clone());

    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

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
        }
        _ => {
            info!("Not using TLS for API");
            TcpListener::bind(address).boxed()
        }
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
    runtime_handle.spawn(async move {
        let cors = Cors::new()
            // To allow browsers to use cookies (for cookie-based sticky
            // routing in the LB) we must enable this:
            // https://stackoverflow.com/a/24689738/3846032
            .allow_credentials(true)
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_headers(vec![header::CONTENT_TYPE, header::ACCEPT]);

        // Build routes for the API
        let route = Route::new()
            .nest(
                "/v1",
                Route::new()
                    .nest("/", api_service)
                    .at("/spec.json", spec_json)
                    .at("/spec.yaml", spec_yaml)
                    // TODO: We add this manually outside of the OpenAPI spec for now.
                    // https://github.com/poem-web/poem/issues/364
                    .at(
                        "/set_failpoint",
                        poem::get(set_failpoints::set_failpoint_poem).data(context.clone()),
                    ),
            )
            .with(cors)
            .with(PostSizeLimit::new(size_limit))
            // NOTE: Make sure to keep this after all the `with` middleware.
            .catch_all_error(convert_error)
            .around(middleware_log);
        Server::new_with_acceptor(acceptor)
            .run(route)
            .await
            .map_err(anyhow::Error::msg)
    });

    info!(
        "Poem is running at {}, behind the reverse proxy at the API port",
        actual_address
    );

    Ok(actual_address)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use aptos_api_test_context::{new_test_context, TestContext};
    use aptos_config::config::NodeConfig;
    use aptos_types::chain_id::ChainId;

    use super::bootstrap;

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
                }
                Err(error) => return Err(error),
            }
        }
    }

    pub async fn new_test_context_async(test_name: String) -> TestContext {
        new_test_context(test_name, false)
    }
}
