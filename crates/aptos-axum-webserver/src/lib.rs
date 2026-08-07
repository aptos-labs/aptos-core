// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A thin axum-based webserver wrapper, mirroring `aptos-warp-webserver`.
//!
//! This provides the small pieces a Rosetta-style server needs: a `serve`
//! abstraction with optional TLS, a request-logging middleware layer, and a
//! JSON-serializable `Error` type implementing `IntoResponse`.

mod error;
mod log;
mod webserver;

pub use error::*;
pub use log::*;
pub use webserver::*;
