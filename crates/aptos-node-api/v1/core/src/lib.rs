// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]
#![allow(clippy::single_component_path_imports)]
use paste; // Needed for the proc macros that generate response types.
use poem_openapi::Tags;

mod accounts;
mod basic;
mod blocks;
mod check_size;
mod config;
mod error_converter;
mod events;
mod failpoint;
mod index;
mod log;
pub mod metrics;
mod page;
mod routes;
mod service;
mod set_failpoints;
mod state;
#[cfg(test)]
pub mod tests;
mod transactions;
mod view_function;

pub use config::ApiV1Config;
pub use routes::build_api_v1_routes;
pub use service::build_api_v1_service;

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
