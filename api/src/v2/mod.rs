// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos REST API v2 (Axum-based).
//!
//! This module contains the v2 API implementation served at `/v2`.
//! It wraps the existing v1 Context for shared DB/mempool access while
//! providing a cleaner, framework-agnostic interface via Axum.

pub mod batch;
pub mod context;
pub mod cursor;
pub mod endpoints;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod router;
pub mod types;

pub use context::V2Context;
pub use error::{ErrorCode, V2Error};
pub use router::build_v2_router;
pub use types::{LedgerMetadata, V2Response};
