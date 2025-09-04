// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use poem::{
    endpoint::{make_sync, Endpoint},
    Response,
};
use poem_openapi::{OpenApi, OpenApiService, Webhook};

/// Get the spec as JSON. We implement our own function because poem-openapi versions
/// greater than 2.0.11 add this charset thing to the content type. This causes issues
/// with our current Accept logic and messes up some code generators and our spec page,
/// so we remove it.
pub fn get_spec<T, W>(service: &OpenApiService<T, W>, yaml: bool) -> String
where
    T: OpenApi,
    W: Webhook,
{
    let spec = if yaml {
        service.spec_yaml()
    } else {
        service.spec()
    };
    spec.replace("; charset=utf-8", "")
}

/// Create an endpoint to serve the OpenAPI specification as json. We define this
/// ourselves because we need to use our custom `get_spec` function that changes the
/// spec to remove charset from the content type.
pub fn spec_endpoint_json<T, W>(service: &OpenApiService<T, W>) -> impl Endpoint
where
    T: OpenApi,
    W: Webhook,
{
    let spec = get_spec(service, false);
    make_sync(move |_| {
        Response::builder()
            .content_type("application/json")
            .body(spec.clone())
    })
}

/// Create an endpoint to serve the OpenAPI specification as yaml. We define this
/// ourselves because we need to use our custom `get_spec` function that changes the
/// spec to remove charset from the content type.
pub fn spec_endpoint_yaml<T, W>(service: &OpenApiService<T, W>) -> impl Endpoint
where
    T: OpenApi,
    W: Webhook,
{
    let spec = get_spec(service, true);
    make_sync(move |_| {
        Response::builder()
            .content_type("application/x-yaml")
            .header("Content-Disposition", "inline; filename=\"spec.yaml\"")
            .body(spec.clone())
    })
}
