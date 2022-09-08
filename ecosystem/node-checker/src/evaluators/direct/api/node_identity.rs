// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    configuration::{EvaluatorArgs, NodeAddress},
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::{format_err, Result};
use aptos_config::config::RoleType;
use aptos_sdk::types::chain_id::ChainId;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, time::Duration};
use thiserror::Error as ThisError;

use super::{super::DirectEvaluatorInput, API_CATEGORY};

/// This function hits the `/` endpoint of the API and returns the chain ID
/// and role type, extracted from the IndexResponse.
pub async fn get_node_identity(
    node_address: &NodeAddress,
    timeout: Duration,
) -> Result<(ChainId, RoleType)> {
    let index_response = node_address
        .get_index_response(timeout)
        .await
        .map_err(|e| {
            format_err!(
                "Failed to get response from index (/) of API. Make sure \
            your API port ({}) is open: {}",
                node_address.get_api_port(),
                e
            )
        })?;
    Ok((
        ChainId::new(index_response.chain_id),
        index_response.node_role,
    ))
}

#[derive(Debug, ThisError)]
pub enum NodeIdentityEvaluatorError {}

// TODO: Consider taking chain_id and role_type here instead.
#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct NodeIdentityEvaluatorArgs {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct NodeIdentityEvaluator {
    args: NodeIdentityEvaluatorArgs,
}

impl NodeIdentityEvaluator {
    pub fn new(args: NodeIdentityEvaluatorArgs) -> Self {
        Self { args }
    }

    fn help_build_evaluation_result<T: Display + PartialEq>(
        &self,
        baseline_value: T,
        target_value: T,
        attribute_str: &str,
    ) -> EvaluationResult {
        let (headline, score, explanation) = if baseline_value == target_value {
            (
                format!("{} reported by baseline and target match", attribute_str),
                100,
                format!(
                    "The node under investigation reported the same {} {} \
                as is reported by the baseline node.",
                    attribute_str, target_value
                ),
            )
        } else {
            (
                format!(
                    "{} reported by the target does not match the baseline",
                    attribute_str
                ),
                0,
                format!(
                    "The node under investigation reported the {} {} while the \
                baseline reported {}. These values should match. Confirm that \
                the baseline you're using is appropriate for the node you're testing.",
                    attribute_str, target_value, baseline_value
                ),
            )
        };
        self.build_evaluation_result(headline, score, explanation)
    }
}

#[async_trait::async_trait]
impl Evaluator for NodeIdentityEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = NodeIdentityEvaluatorError;

    /// Assert that the node identity (role type and chain ID) of the two nodes match.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let evaluation_results = vec![
            self.help_build_evaluation_result(
                input.get_baseline_chain_id(),
                input.get_target_chain_id(),
                "Chain ID",
            ),
            self.help_build_evaluation_result(
                input.baseline_node_information.role_type,
                input.target_index_response.node_role,
                "Role Type",
            ),
        ];

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        API_CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "node_identity".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.node_identity_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(_: &EvaluatorArgs) -> Result<EvaluatorType> {
        unreachable!();
    }
}
