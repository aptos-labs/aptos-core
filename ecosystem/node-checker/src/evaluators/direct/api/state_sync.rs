// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::super::DirectEvaluatorInput;
use super::ApiEvaluatorError;
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::Result;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct StateSyncVersionEvaluatorArgs {
    #[clap(long, default_value_t = 10000)]
    pub version_delta_tolerance: u64,

    #[clap(long, default_value_t = 5)]
    pub version_fetch_delay_secs: u64,

    #[clap(long, default_value_t = 4)]
    pub api_call_timeout_secs: u64,
}

#[derive(Debug)]
pub struct StateSyncVersionEvaluator {
    args: StateSyncVersionEvaluatorArgs,
}

impl StateSyncVersionEvaluator {
    pub fn new(args: StateSyncVersionEvaluatorArgs) -> Self {
        Self { args }
    }

    fn build_state_sync_version_evaluation(
        &self,
        previous_target_version: u64,
        latest_target_version: u64,
        latest_baseline_version: u64,
    ) -> EvaluationResult {
        // We convert to i64 to avoid potential overflow if somehow the ledger version went backwards.
        let target_progress = latest_target_version as i64 - previous_target_version as i64;
        match target_progress {
            target_progress if (target_progress == 0) => self.build_evaluation_result(
                "Ledger version is not increasing".to_string(),
                25,
                format!(
                    "Successfully pulled ledger version from your node \
                        twice, but the ledger version isnt't increasing ({} both times).",
                    latest_target_version
                ),
            ),
            target_progress if (target_progress < 0) => self.build_evaluation_result(
                "Ledger version went backwards!".to_string(),
                0,
                format!(
                    "Successfully pulled ledger version from your node twice, \
                    but the second time the ledger version went backwards! \
                    First datapoint: {}, second datapoint: {}",
                    previous_target_version, latest_target_version
                ),
            ),
            _wildcard => {
                // We convert to i64 to avoid potential overflow if the target is ahead of the baseline.
                let delta_from_baseline =
                    latest_baseline_version as i64 - latest_target_version as i64;
                if delta_from_baseline > self.args.version_delta_tolerance as i64 {
                    self.build_evaluation_result(
                        "Ledger version is lagging".to_string(),
                        50,
                        format!(
                            "Successfully pulled ledger version from your node twice \
                            and saw the version was increasing, but it is lagging {} versions \
                            behind the baseline node, more than the allowed lag of {}. \
                            Target version: {}. Baseline version: {}.",
                            delta_from_baseline,
                            self.args.version_delta_tolerance,
                            latest_target_version,
                            latest_baseline_version,
                        ),
                    )
                } else {
                    self.build_evaluation_result(
                        "Ledger version is increasing".to_string(),
                        100,
                        format!(
                            "NHC pulled ledger version from your node twice, \
                            saw that the version is increasing (it increased by {} over \
                            {} seconds), and saw that it is within tolerance of the \
                            baseline node. The baseline ledger version is {} and your node's \
                            ledger version is {}, which is within the allowed lag of {} versions.",
                            target_progress,
                            self.args.version_fetch_delay_secs,
                            latest_baseline_version,
                            latest_target_version,
                            self.args.version_delta_tolerance
                        ),
                    )
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for StateSyncVersionEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = ApiEvaluatorError;

    /// Assert that the ledger version is increasing on the target node
    /// and that we're within tolerance of the baseline node's latest version.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let api_call_timeout = Duration::from_secs(self.args.api_call_timeout_secs);

        // We already have one ledger version from the target.
        let previous_target_version = input.target_index_response.ledger_version;

        // Now wait.
        tokio::time::sleep(Duration::from_secs(self.args.version_fetch_delay_secs)).await;

        // Get the target ledger version again after the delay. If this fails,
        // return an evaluation result indicating as such.
        let latest_target_version = match input
            .target_node_address
            .get_index_response_or_evaluation_result(api_call_timeout)
            .await
        {
            Ok(response) => response.ledger_version,
            Err(evaluation_result) => return Ok(vec![evaluation_result]),
        };

        // Get the latest version from the baseline node. In this case, if we
        // cannot find the value, we return an error instead of a negative evalution,
        // since this implies some issue with the baseline node / this code.
        let latest_baseline_version = match input
            .baseline_node_information
            .node_address
            .get_index_response(api_call_timeout)
            .await
        {
            Ok(response) => response.ledger_version,
            Err(e) => {
                return Err(ApiEvaluatorError::EndpointError("/".to_string(), e));
            }
        };

        // Evaluate the data, returning an evaluation.
        Ok(vec![self.build_state_sync_version_evaluation(
            previous_target_version.0,
            latest_target_version.0,
            latest_baseline_version.0,
        )])
    }

    fn get_category_name() -> String {
        "state_sync".to_string()
    }

    fn get_evaluator_name() -> String {
        "version".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.state_sync_version_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Api(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}
