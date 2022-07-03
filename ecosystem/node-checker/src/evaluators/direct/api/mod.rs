// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod latency;
mod node_identity;

pub use latency::{LatencyEvaluator, LatencyEvaluatorArgs};
pub use node_identity::{
    get_node_identity, NodeIdentityEvaluator, NodeIdentityEvaluatorArgs, NodeIdentityEvaluatorError,
};
use thiserror::Error as ThisError;

pub const API_CATEGORY: &str = "api";

#[derive(Debug, ThisError)]
pub enum ApiEvaluatorError {}
