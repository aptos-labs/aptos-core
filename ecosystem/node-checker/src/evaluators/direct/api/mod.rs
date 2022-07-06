// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod latency;
mod node_identity;
mod transaction_availability;

use anyhow::{Error, Result};
use aptos_rest_client::IndexResponse;
pub use latency::{LatencyEvaluator, LatencyEvaluatorArgs};
pub use node_identity::{
    get_node_identity, NodeIdentityEvaluator, NodeIdentityEvaluatorArgs, NodeIdentityEvaluatorError,
};
use thiserror::Error as ThisError;
pub use transaction_availability::{
    TransactionAvailabilityEvaluator, TransactionAvailabilityEvaluatorArgs,
};

use crate::{configuration::NodeAddress, evaluator::EvaluationResult};

pub const API_CATEGORY: &str = "api";

#[derive(Debug, ThisError)]
pub enum ApiEvaluatorError {
    #[error("API returned an error for endpoint {0}: {1}")]
    EndpointError(String, Error),
}

pub async fn get_index_response(node_address: &NodeAddress) -> Result<IndexResponse> {
    Ok(node_address
        .get_api_client()
        .get_index()
        .await?
        .into_inner())
}

pub async fn get_index_response_or_evaluation_result(
    node_address: &NodeAddress,
) -> Result<IndexResponse, EvaluationResult> {
    match get_index_response(node_address).await {
        Ok(index_response) => Ok(index_response),
        Err(error) => Err(EvaluationResult {
            headline: "Failed to read response from / on API".to_string(),
            score: 0,
            explanation: format!("We received an error response hitting / (the index) of the API of your node, make sure your API port ({}) is publicly accessible: {}", node_address.api_port, error),
            category: API_CATEGORY.to_string(),
            evaluator_name: "index_response".to_string(),
            links: vec![],
        })
    }
}
