// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod latency;
mod node_identity;
mod transaction_presence;

use anyhow::Error;
pub use latency::{LatencyEvaluator, LatencyEvaluatorArgs};
pub use node_identity::{
    get_node_identity, NodeIdentityEvaluator, NodeIdentityEvaluatorArgs, NodeIdentityEvaluatorError,
};
use thiserror::Error as ThisError;
pub use transaction_presence::{TransactionPresenceEvaluator, TransactionPresenceEvaluatorArgs};

pub const API_CATEGORY: &str = "api";

#[derive(Debug, ThisError)]
pub enum ApiEvaluatorError {
    #[error("API returned an error for endpoint {0}: {1}")]
    EndpointError(String, Error),
}
