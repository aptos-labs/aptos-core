// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::Tags;

mod accept_type;
mod accounts;
mod basic;
mod bcs_payload;
mod blocks;
mod check_size;
pub mod context;
mod error_converter;
mod events;
mod failpoint;
mod index;
mod log;
pub mod metrics;
mod page;
mod response;
mod runtime;
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
