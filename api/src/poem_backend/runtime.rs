// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::NodeConfig;
use poem::{listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;
use tokio::runtime::Runtime;

use super::api::Api;

pub fn attach_poem_to_runtime(runtime: &Runtime, config: &NodeConfig) -> anyhow::Result<()> {
    let api = Api {};

    // todo make this configurable
    let api_endpoint = "/".to_string();
    let api_service = build_openapi_service(api);
    let ui = api_service.swagger_ui();
    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    let address = config.api.address;

    runtime.spawn(async move {
        Server::new(TcpListener::bind(address))
            .run(
                Route::new()
                    .nest(api_endpoint, api_service)
                    .nest("/spec.html", ui)
                    .at("/spec.yaml", spec_json)
                    .at("/spec.json", spec_yaml),
            )
            .await
            .map_err(anyhow::Error::msg)
    });

    Ok(())
}

pub fn build_openapi_service(api: Api) -> OpenApiService<Api, ()> {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".to_string());
    OpenApiService::new(api, "Aptos Node API", version)
}
