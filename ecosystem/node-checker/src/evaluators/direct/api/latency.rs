// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{super::DirectEvaluatorInput, ApiEvaluatorError, API_CATEGORY};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::EvaluatorType,
};
use anyhow::Result;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct LatencyEvaluatorArgs {
    /// The number of times to hit the node to check latency.
    #[clap(long, default_value_t = 5)]
    pub num_samples: u16,

    /// The delay between each call.
    #[clap(long, default_value_t = 20)]
    pub delay_between_samples_ms: u64,

    /// The number of responses that are allowed to be errors.
    #[clap(long, default_value_t = 1)]
    pub num_allowed_errors: u16,

    /// If the average latency exceeds this value, it will fail the evaluation.
    #[clap(long, default_value_t = 500)]
    pub max_latency_ms: u64,
}

#[derive(Debug)]
pub struct LatencyEvaluator {
    args: LatencyEvaluatorArgs,
}

impl LatencyEvaluator {
    pub fn new(args: LatencyEvaluatorArgs) -> Self {
        Self { args }
    }

    async fn get_latency_datapoint(&self, target_url: Url) -> Result<Duration> {
        let start = Instant::now();
        let client = reqwest::ClientBuilder::new()
            .timeout(std::time::Duration::from_millis(
                self.args.max_latency_ms * 2,
            ))
            .build()
            .unwrap();
        client.get(target_url).send().await?;
        Ok(start.elapsed())
    }
}

#[async_trait::async_trait]
impl Evaluator for LatencyEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = ApiEvaluatorError;

    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let mut target_url = input.target_node_address.url.clone();
        target_url
            .set_port(Some(input.target_node_address.api_port))
            .unwrap();

        let mut errors = vec![];

        let mut latencies = vec![];
        for _ in 0..self.args.num_samples {
            match self.get_latency_datapoint(target_url.clone()).await {
                Ok(latency) => latencies.push(latency),
                Err(e) => errors.push(e),
            }
            if errors.len() as u16 > self.args.num_allowed_errors {
                return Ok(vec![self.build_evaluation_result(
                    "Node returned too many errors while checking latency".to_string(),
                    0,
                    format!(
                        "The node returned too many errors while checking latency, the tolerance was {} errors out of {} calls: {}",
                        self.args.num_allowed_errors, self.args.num_samples, errors.into_iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", "),
                    ),
                )]);
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                self.args.delay_between_samples_ms,
            ))
            .await;
        }

        let average_latency =
            latencies.iter().sum::<Duration>().as_millis() as u64 / latencies.len() as u64;

        let evaluation_result = if average_latency > self.args.max_latency_ms {
            self.build_evaluation_result(
                "Average latency too high".to_string(),
                50,
                format!(
                    "The average latency was {}ms, which is higher than the maximum allowed latency of {}ms",
                    average_latency, self.args.max_latency_ms
                ),
            )
        } else {
            self.build_evaluation_result(
                "Average latency is good".to_string(),
                100,
                format!(
                    "The average latency was {}ms, which is below the maximum allowed latency of {}ms",
                    average_latency, self.args.max_latency_ms
                ),
            )
        };

        Ok(vec![evaluation_result])
    }

    fn get_category_name() -> String {
        API_CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "latency".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.latency_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Api(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}
