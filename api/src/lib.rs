// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use poem_openapi::Tags;

mod accept_type;
mod accept_type_axum;
mod accounts;
mod basic;
mod bcs_payload;
mod bcs_payload_axum;
mod blocks;
mod check_size;
pub mod context;
mod error_converter;
mod error_converter_axum;
mod events;
mod failpoint;
mod index;
mod log;
pub mod metrics;
mod middleware_axum;
mod page;
mod response;
pub mod response_axum;
mod routes_axum;
mod runtime;
pub mod runtime_axum;
mod set_failpoints;
pub mod spec;
mod state;
#[cfg(test)]
pub mod tests;
mod transactions;
mod view_function;

/// API categories for the OpenAPI spec
#[derive(Tags)]
pub enum ApiTags {
    /// Access to accounts, resources, and modules
    Accounts,
    /// Access to blocks
    Blocks,

    /// Access to events
    Events,

    /// Experimental APIs, no guarantees
    Experimental,

    /// General information
    General,

    /// Access to tables
    Tables,

    /// Access to transactions
    Transactions,

    /// View functions,
    View,
}

// Note: Many of these exports are just for the test-context crate, which is
// needed outside of the API, e.g. for fh-stream.
pub use context::Context;
pub use response::BasicError;
pub use runtime::{attach_poem_to_runtime, bootstrap, get_api_service};
pub use runtime_axum::attach_axum_to_runtime;
