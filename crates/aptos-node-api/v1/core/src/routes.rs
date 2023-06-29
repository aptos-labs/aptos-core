// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_size::PostSizeLimit, error_converter::convert_error, log::middleware_log,
    service::build_api_v1_service, set_failpoints, ApiV1Config,
};
use anyhow::Result;
use aptos_node_api_v1_types::X_APTOS_CLIENT;
use poem::{
    http::{header, Method},
    middleware::Cors,
    EndpointExt, IntoEndpoint, Route,
};

/// Build the routes for the v1 node API.
pub fn build_api_v1_routes(config: ApiV1Config) -> Result<impl IntoEndpoint> {
    let size_limit = config.context.content_length_limit();

    let api_service = build_api_v1_service(config.context.clone());

    let spec_json = api_service.spec_endpoint();
    let spec_yaml = api_service.spec_endpoint_yaml();

    let cors = Cors::new()
        // To allow browsers to use cookies (for cookie-based sticky
        // routing in the LB) we must enable this:
        // https://stackoverflow.com/a/24689738/3846032
        .allow_credentials(true)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![
            header::HeaderName::from_static(X_APTOS_CLIENT),
            header::CONTENT_TYPE,
            header::ACCEPT,
        ]);

    // Build routes for the API
    let routes = Route::new()
        .nest("/", api_service)
        .at("/spec.json", spec_json)
        .at("/spec.yaml", spec_yaml)
        // TODO: We add this manually outside of the OpenAPI spec for now.
        // https://github.com/poem-web/poem/issues/364
        .at(
            "/set_failpoint",
            poem::get(set_failpoints::set_failpoint_poem).data(config.context),
        )
        .with(cors)
        .with(PostSizeLimit::new(size_limit))
        // NOTE: Make sure to keep this after all the `with` middleware.
        .catch_all_error(convert_error)
        .around(middleware_log);

    Ok(routes)
}
