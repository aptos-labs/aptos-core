// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// TODO: Sometimes build_commit_hash is an empty string (so far I've noticed
// this happens when targeting a node running from a container). Figure out
// what to do in this case.

use super::{
    common::{get_value, GetValueResult},
    types::{SystemInformationEvaluatorError, SystemInformationEvaluatorInput},
    CATEGORY,
};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
    metric_collector::SystemInformation,
};
use anyhow::Result;
use clap::Parser;
use log::debug;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

// TODO: Use the key in crates/aptos-telemetry/src/build_information.rs
const BUILD_COMMIT_HASH_KEY: &str = "build_commit_hash";

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct BuildVersionEvaluatorArgs {}

#[allow(dead_code)]
#[derive(Debug)]
pub struct BuildVersionEvaluator {
    args: BuildVersionEvaluatorArgs,
}

impl BuildVersionEvaluator {
    pub fn new(args: BuildVersionEvaluatorArgs) -> Self {
        Self { args }
    }

    fn get_build_commit_hash(&self, system_information: &SystemInformation) -> GetValueResult {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "Build commit hash value missing".to_string(),
                0,
                format!(
                    "The build information from the node is missing: {}",
                    BUILD_COMMIT_HASH_KEY
                ),
            )
        };
        get_value(
            system_information,
            BUILD_COMMIT_HASH_KEY,
            evaluation_on_missing_fn,
        )
    }
}

#[async_trait::async_trait]
impl Evaluator for BuildVersionEvaluator {
    type Input = SystemInformationEvaluatorInput;
    type Error = SystemInformationEvaluatorError;

    /// Assert that the build commit hashes match.
    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut evaluation_results = vec![];

        let baseline_build_commit_hash = match self
            .get_build_commit_hash(&input.baseline_system_information)
        {
            GetValueResult::Present(value) => value,
            GetValueResult::Missing(_evaluation_result) => {
                return
                    Err(SystemInformationEvaluatorError::BaselineMissingKey(
                        BUILD_COMMIT_HASH_KEY.to_string(),
                        format!("The latest set of metrics from the baseline node did not contain the necessary key \"{}\"", BUILD_COMMIT_HASH_KEY),
                    ));
            }
        };

        let target_build_commit_hash =
            match self.get_build_commit_hash(&input.target_system_information) {
                GetValueResult::Present(value) => Some(value),
                GetValueResult::Missing(evaluation_result) => {
                    evaluation_results.push(evaluation_result);
                    None
                }
            };

        match target_build_commit_hash {
            Some(target_build_commit_hash) => {
                evaluation_results.push({
                    if baseline_build_commit_hash == target_build_commit_hash {
                        self.build_evaluation_result(
                            "Build commit hashes match".to_string(),
                            100,
                            format!(
                                "The build commit hash from the target node ({}) matches the build commit hash from the baseline node ({}).",
                                target_build_commit_hash, baseline_build_commit_hash
                            ),
                        )
                    } else {
                        self.build_evaluation_result(
                            "Build commit hash mismatch".to_string(),
                            50,
                            format!(
                                "The build commit hash from the target node ({}) does not match the build commit hash from the baseline node ({}).",
                                target_build_commit_hash, baseline_build_commit_hash
                            ),
                        )
                    }
                });
            }
            None => debug!(
                "Not evaluating build commit hash because we're missing data from the target"
            ),
        }

        Ok(evaluation_results)
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "build_commit_hash".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.build_version_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::SystemInformation(Box::new(
            Self::from_evaluator_args(evaluator_args)?,
        )))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    fn get_system_information(build_commit_hash: &str) -> SystemInformation {
        let mut inner = HashMap::new();
        inner.insert(
            BUILD_COMMIT_HASH_KEY.to_string(),
            build_commit_hash.to_string(),
        );
        SystemInformation(inner)
    }

    async fn test_evaluator(
        baseline_build_commit_hash: Option<&str>,
        target_build_commit_hash: Option<&str>,
        expected_score: u8,
    ) {
        let baseline_system_information = match baseline_build_commit_hash {
            Some(v) => get_system_information(v),
            None => SystemInformation(HashMap::new()),
        };

        let target_system_information = match target_build_commit_hash {
            Some(v) => get_system_information(v),
            None => SystemInformation(HashMap::new()),
        };

        let evaluator = BuildVersionEvaluator::new(BuildVersionEvaluatorArgs {});

        let system_information_evaluator_input = SystemInformationEvaluatorInput {
            baseline_system_information,
            target_system_information,
        };

        let evaluations = evaluator
            .evaluate(&system_information_evaluator_input)
            .await
            .expect("Failed to evaluate system information");

        assert_eq!(evaluations.len(), 1);
        assert_eq!(evaluations[0].score, expected_score);
    }

    #[tokio::test]
    async fn test_same() {
        test_evaluator(Some("aaaaaaaaaa"), Some("aaaaaaaaaa"), 100).await;
    }

    #[tokio::test]
    async fn test_different() {
        test_evaluator(Some("aaaaaaaaaa"), Some("bbbbbbbbbb"), 50).await;
    }

    #[tokio::test]
    async fn test_missing_target_metric() {
        test_evaluator(Some("aaaaaaaaaa"), None, 0).await;
    }

    #[tokio::test]
    #[should_panic(expected = "did not contain the necessary key")]
    async fn test_both_missing_metrics() {
        test_evaluator(None, None, 0).await;
    }
}
