// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{schema::QueryRoot, ApiV2Config};
use anyhow::Result;
use async_graphql::{http::GraphiQLSource, EmptyMutation, EmptySubscription, Schema};
use async_graphql_poem::GraphQL;
use poem::{get, handler, web::Html, IntoEndpoint, IntoResponse, Route};

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/v2/read").finish())
}

pub fn build_api_v2_routes(_config: ApiV2Config) -> Result<impl IntoEndpoint> {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription).finish();

    // Build routes for the read API
    let routes = Route::new().at("/read", get(graphiql).post(GraphQL::new(schema)));

    Ok(routes)
}
