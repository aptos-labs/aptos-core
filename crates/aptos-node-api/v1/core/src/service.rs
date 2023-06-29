// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    accounts::AccountsApi, basic::BasicApi, blocks::BlocksApi, events::EventsApi, index::IndexApi,
    state::StateApi, transactions::TransactionsApi, view_function::ViewFunctionApi,
};
use aptos_node_api_context::Context;
use poem_openapi::{ContactObject, LicenseObject, OpenApiService};
use std::sync::Arc;

const VERSION: &str = include_str!("../../doc/.version");

/// Generate the top level API service
pub fn build_api_v1_service(
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
        .name("Aptos Labs")
        .url("https://github.com/aptos-labs/aptos-core");

    OpenApiService::new(apis, "Aptos Node API", version.trim())
        .server("/v1")
        .description("The Aptos Node API is a RESTful API for client applications to interact with the Aptos blockchain.")
        .license(license)
        .contact(contact)
        .external_document("https://github.com/aptos-labs/aptos-core")
}
