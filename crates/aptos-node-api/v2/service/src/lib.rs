// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod config;
mod module;
mod routes;
mod schema;

pub use config::ApiV2Config;
pub use routes::build_api_v2_routes;
