// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::DirectEvaluatorInput;
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
};
use anyhow::Result;
use clap::Parser;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tokio::time::{Duration, Instant};
use url::Url;

pub const CATEGORY: &str = "performance";

#[derive(Debug, ThisError)]
pub enum LatencyEvaluatorError {}

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

    fn build_evaluation_result(
        &self,
        headline: String,
        score: u8,
        explanation: String,
    ) -> EvaluationResult {
        EvaluationResult {
            headline,
            score,
            explanation,
            category: CATEGORY.to_string(),
            evaluator_name: Self::get_name(),
            links: vec![],
        }
    }

    async fn get_latency_datapoint(&self, target_url: Url) -> Result<Duration> {
        let start = Instant::now();
        reqwest::get(target_url).await?;
        Ok(start.elapsed())
    }
}

#[async_trait::async_trait]
impl Evaluator for LatencyEvaluator {
    type Input = DirectEvaluatorInput;
    type Error = LatencyEvaluatorError;

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
                        self.args.num_allowed_errors, self.args.num_samples, errors.into_iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ")
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

    fn get_name() -> String {
        format!("{}_latency", CATEGORY)
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.latency_args.clone()))
    }
}
