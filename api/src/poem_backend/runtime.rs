// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{net::SocketAddr, sync::Arc};

use super::{middleware_log, AccountsApi, BasicApi, EventsApi, IndexApi};

use crate::{
    context::Context,
    poem_backend::{
        check_size::PostSizeLimit, error_converter::convert_error, StateApi, TransactionsApi,
    },
};
use anyhow::Context as AnyhowContext;
use aptos_config::config::NodeConfig;
use aptos_logger::info;
use poem::{
    http::{header, Method},
    listener::{Listener, RustlsCertificate, RustlsConfig, TcpListener},
    middleware::Cors,
    EndpointExt, Route, Server,
};
use poem_openapi::{ContactObject, LicenseObject, OpenApiService};
use tokio::runtime::Handle;

// TODOs regarding spec generation:
// TODO: https://github.com/aptos-labs/aptos-core/issues/2280
// TODO: https://github.com/poem-web/poem/issues/321
// TODO: https://github.com/poem-web/poem/issues/332
// TODO: https://github.com/poem-web/poem/issues/333

pub fn get_api_service(
    context: Arc<Context>,
) -> OpenApiService<
    (
        AccountsApi,
        BasicApi,
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

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    let license =
        LicenseObject::new("Apache 2.0").url("https://www.apache.org/licenses/LICENSE-2.0.html");
    let contact = ContactObject::new()
        .name("Aptos Labs")
        .url("https://github.com/aptos-labs/aptos-core");

    OpenApiService::new(apis, "Aptos Node API", version)
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
) -> anyhow::Result<SocketAddr> {
    let context = Arc::new(context);

    let size_limit = context.content_length_limit();

    let api_service = get_api_service(context);

    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    let mut address = config.api.address;

    // TODO: This is temporary while we serve both APIs simulatenously.
    // Doing this means the OS assigns it an unused port.
    address.set_port(0);

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
            .context("Failed to bind Poem to address with OS assigned port")
    })?;

    let actual_address = &acceptor.local_addr()[0];
    let actual_address = *actual_address
        .as_socket_addr()
        .context("Failed to get socket addr from local addr for Poem webserver")?;
    runtime_handle.spawn(async move {
        let cors = Cors::new()
            .allow_methods(vec![Method::GET, Method::POST])
            .allow_headers(vec![header::CONTENT_TYPE, header::ACCEPT]);
        let route = Route::new()
            .nest("/", api_service)
            .at("/spec.json", spec_json)
            .at("/spec.yaml", spec_yaml)
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
