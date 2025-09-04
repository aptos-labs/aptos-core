// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
use crate::{
    get_provider,
    provider::{api_index::ApiIndexProvider, Provider, ProviderCollection},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};

const LINK: &str =
    "https://velor.dev/nodes/node-health-checker/node-health-checker-faq#how-does-the-latency-evaluator-work";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LatencyCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    /// The number of times to hit the node to check latency.
    #[serde(default = "LatencyCheckerConfig::default_num_samples")]
    pub num_samples: u16,

    /// The delay between each call.
    #[serde(default = "LatencyCheckerConfig::default_delay_between_samples_ms")]
    pub delay_between_samples_ms: u64,

    /// The number of responses that are allowed to be errors.
    #[serde(default)]
    pub num_allowed_errors: u16,

    /// If the average latency exceeds this value, it will fail the evaluation.
    /// This value is not the same as regular latency , e.g. with the ping tool.
    /// Instead, this measures the total RTT for an API call to the node. See
    /// https://velor.dev/nodes/node-health-checker/node-health-checker-faq#how-does-the-latency-evaluator-work
    /// for more information.
    pub max_api_latency_ms: u64,
}

impl LatencyCheckerConfig {
    const fn default_num_samples() -> u16 {
        5
    }

    const fn default_delay_between_samples_ms() -> u64 {
        50
    }
}

#[derive(Debug)]
pub struct LatencyChecker {
    config: LatencyCheckerConfig,
}

impl LatencyChecker {
    pub fn new(config: LatencyCheckerConfig) -> Self {
        Self { config }
    }

    async fn get_latency_datapoint(&self, provider: &ApiIndexProvider) -> Result<Duration> {
        let start = Instant::now();
        provider.provide().await?;
        Ok(start.elapsed())
    }
}

#[async_trait::async_trait]
impl Checker for LatencyChecker {
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_api_index_provider = get_provider!(
            providers.target_api_index_provider,
            self.config.common.required,
            ApiIndexProvider
        );

        let mut errors = vec![];

        let mut latencies = vec![];
        for _ in 0..self.config.num_samples {
            match self.get_latency_datapoint(target_api_index_provider).await {
                Ok(latency) => latencies.push(latency),
                Err(e) => errors.push(e),
            }
            if errors.len() as u16 > self.config.num_allowed_errors {
                return Ok(vec![
                    Self::build_result(
                        "Node returned too many errors while checking API latency".to_string(),
                        0,
                        format!(
                            "The node returned too many errors while checking API RTT (Round trip time), the tolerance was {} errors out of {} calls: {}. Note, this latency is not the same as standard ping latency, see the attached link.",
                            self.config.num_allowed_errors, self.config.num_samples, errors.into_iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", "),
                        )
                    ).links(vec![LINK.to_string()])
                ]);
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                self.config.delay_between_samples_ms,
            ))
            .await;
        }

        let average_latency =
            latencies.iter().sum::<Duration>().as_millis() as u64 / latencies.len() as u64;

        let evaluation_result = if average_latency > self.config.max_api_latency_ms {
            Self::build_result(
                "Average API latency too high".to_string(),
                50,
                format!(
                    "The average API latency was {}ms, which is higher than the maximum allowed latency of {}ms. Note, this latency is not the same as standard ping latency, see the attached link.",
                    average_latency, self.config.max_api_latency_ms
                )
            ).links(vec![LINK.to_string()])
        } else {
            Self::build_result(
                "Average API latency is good".to_string(),
                100,
                format!(
                    "The average API latency was {}ms, which is below the maximum allowed latency of {}ms. Note, this latency is not the same as standard ping latency, see the attached link.",
                    average_latency, self.config.max_api_latency_ms
                )
            ).links(vec![LINK.to_string()])
        };

        Ok(vec![evaluation_result])
    }
}
