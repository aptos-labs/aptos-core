// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

/// These evaluators are only valuable in certain contexts. For example, this is
/// not a useful evaluator for node registration for the AITs, since each node
/// is running in their own isolated network, where no consensus is occurring.
/// This is useful for the AIT itself though, where the nodes are participating
/// in a real network.

///
use super::{
    super::{
        common::{get_metric, GetMetricResult},
        types::{MetricsEvaluatorError, MetricsEvaluatorInput},
    },
    CATEGORY,
};
use crate::{
    configuration::EvaluatorArgs,
    evaluator::{EvaluationResult, Evaluator},
    evaluators::{metrics::common::Label, EvaluatorType},
};
use anyhow::Result;
use clap::Parser;
use once_cell::sync::Lazy;
use poem_openapi::Object as PoemObject;
use prometheus_parse::Scrape as PrometheusScrape;
use serde::{Deserialize, Serialize};

//////////////////////////////////////////////////////////////////////////////
// Common stuff.
//////////////////////////////////////////////////////////////////////////////

const METRIC: &str = "aptos_connections";

static INBOUND_LABEL: Lazy<Label> = Lazy::new(|| Label {
    key: "direction",
    value: "inbound",
});
static OUTBOUND_LABEL: Lazy<Label> = Lazy::new(|| Label {
    key: "direction",
    value: "outbound",
});

enum ConnectionType {
    Inbound,
    Outbound,
}

impl ConnectionType {
    fn get_name(&self) -> &'static str {
        match &self {
            ConnectionType::Inbound => "inbound",
            ConnectionType::Outbound => "outbound",
        }
    }

    fn get_particle(&self) -> &'static str {
        match &self {
            ConnectionType::Inbound => "from",
            ConnectionType::Outbound => "to",
        }
    }

    fn get_opposite_particle(&self) -> &'static str {
        match &self {
            ConnectionType::Inbound => "to",
            ConnectionType::Outbound => "from",
        }
    }
}

// This trait defines common functions for both network peer evaluators here.
trait NetworkPeersEvaluator: Evaluator {
    fn get_metrics(&self, metrics: &PrometheusScrape) -> Result<(u64, u64), Vec<EvaluationResult>>
    where
        Self: Sized,
    {
        let evaluation_on_missing_fn = || {
            self.build_evaluation_result(
                "Missing metric".to_string(),
                0,
                format!(
                    "The metrics from the node are missing the metric: {}",
                    METRIC
                ),
            )
        };
        let (inbound, outbound) = (
            get_metric(
                metrics,
                METRIC,
                Some(&INBOUND_LABEL),
                evaluation_on_missing_fn,
            ),
            get_metric(
                metrics,
                METRIC,
                Some(&OUTBOUND_LABEL),
                evaluation_on_missing_fn,
            ),
        );
        if let (GetMetricResult::Present(inbound), GetMetricResult::Present(outbound)) =
            (&inbound, &outbound)
        {
            return Ok((*inbound, *outbound));
        }
        let mut evaluation_results = vec![];
        if let GetMetricResult::Missing(evaluation_result) = inbound {
            evaluation_results.push(evaluation_result);
        }
        if let GetMetricResult::Missing(evaluation_result) = outbound {
            evaluation_results.push(evaluation_result);
        }
        Err(evaluation_results)
    }
}

//////////////////////////////////////////////////////////////////////////////
// Evaluator for minimum number of peers.
//////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct NetworkMinimumPeersEvaluatorArgs {
    /// The minimum number of inbound connections required to be able to pass.
    /// For fullnodes, it only matters that this is greater than zero if the
    /// node operator wants to seed data to other nodes.
    #[clap(long, default_value_t = 0)]
    pub minimum_peers_inbound: u64,

    /// The minimum number of outbound connections required to be able to pass.
    /// This must be greater than zero for the node to be able to synchronize.
    #[clap(long, default_value_t = 1)]
    pub minimum_peers_outbound: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct NetworkMinimumPeersEvaluator {
    args: NetworkMinimumPeersEvaluatorArgs,
}

impl NetworkMinimumPeersEvaluator {
    pub fn new(args: NetworkMinimumPeersEvaluatorArgs) -> Self {
        Self { args }
    }

    #[allow(clippy::comparison_chain)]
    fn build_evaluation(
        &self,
        connections: u64,
        minimum: u64,
        connection_type: &ConnectionType,
    ) -> EvaluationResult {
        let name = connection_type.get_name();
        let particle = connection_type.get_particle();
        let opposite_particle = connection_type.get_opposite_particle();
        let explanation = format!(
            "There are {} {} connections {} other nodes {} the target node (the minimum is {}).",
            connections, name, particle, opposite_particle, minimum
        );
        if connections >= minimum {
            self.build_evaluation_result(
                format!(
                    "There are sufficient {} connections {} the target node",
                    name, particle
                ),
                100,
                explanation,
            )
        } else {
            self.build_evaluation_result_with_links(
                format!(
                    "There are not enough {} connections {} the target node",
                    name, particle
                ),
                50,
                format!("{} Try setting explicit peers.", explanation),
                vec![
                    "https://aptos.dev/nodes/full-node/troubleshooting-fullnode-setup".to_string(),
                ],
            )
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for NetworkMinimumPeersEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let (inbound_connections, outbound_connections) =
            match self.get_metrics(&input.latest_target_metrics) {
                Ok((inbound_connections, outbound_connections)) => {
                    (inbound_connections, outbound_connections)
                }
                Err(evaluation_results) => return Ok(evaluation_results),
            };

        Ok(vec![
            self.build_evaluation(
                inbound_connections,
                self.args.minimum_peers_inbound,
                &ConnectionType::Inbound,
            ),
            self.build_evaluation(
                outbound_connections,
                self.args.minimum_peers_outbound,
                &ConnectionType::Outbound,
            ),
        ])
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "minimum_peers".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(evaluator_args.network_minimum_peers_args.clone()))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Metrics(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}

impl NetworkPeersEvaluator for NetworkMinimumPeersEvaluator {}

//////////////////////////////////////////////////////////////////////////////
// Evaluator for number of peers within tolerance.
//////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Debug, Deserialize, Parser, PoemObject, Serialize)]
pub struct NetworkPeersWithinToleranceEvaluatorArgs {
    /// The evaluator will ensure that the inbound connections count is
    /// within this tolerance of the value retrieved from the baseline.
    #[clap(long, default_value_t = 10)]
    pub inbound_peers_tolerance: u64,

    /// The evaluator will ensure that the outbound connections count is
    /// within this tolerance of the value retrieved from the baseline.
    #[clap(long, default_value_t = 10)]
    pub outbound_peers_tolerance: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct NetworkPeersWithinToleranceEvaluator {
    args: NetworkPeersWithinToleranceEvaluatorArgs,
}

impl NetworkPeersWithinToleranceEvaluator {
    pub fn new(args: NetworkPeersWithinToleranceEvaluatorArgs) -> Self {
        Self { args }
    }

    #[allow(clippy::comparison_chain)]
    fn build_evaluation(
        &self,
        target_connections: u64,
        baseline_connections: u64,
        tolerance: u64,
        connection_type: &ConnectionType,
    ) -> EvaluationResult {
        let name = connection_type.get_name();
        let particle = connection_type.get_particle();
        let opposite_particle = connection_type.get_opposite_particle();
        if target_connections < baseline_connections.saturating_sub(tolerance)
            || target_connections > baseline_connections.saturating_add(tolerance)
        {
            self.build_evaluation_result(
                format!("The number of {} connections {} the target node is too different compared to the baseline", name, opposite_particle),
                50,
                format!("There are {} {} connections {} other nodes to the target node, which is too different compared to the baseline: {} (tolerance: {})", target_connections, name, particle, baseline_connections, tolerance),
            )
        } else {
            self.build_evaluation_result(
                format!("The number of {} connections {} the target node looks good", name, opposite_particle),
                100,
                format!("There are {} {} connections {} other nodes to the target node, which is within tolerance ({}) of the value from the baseline: {}", target_connections, name, particle, tolerance, baseline_connections),
            )
        }
    }
}

#[async_trait::async_trait]
impl Evaluator for NetworkPeersWithinToleranceEvaluator {
    type Input = MetricsEvaluatorInput;
    type Error = MetricsEvaluatorError;

    async fn evaluate(&self, input: &Self::Input) -> Result<Vec<EvaluationResult>, Self::Error> {
        let (baseline_inbound_connections, baseline_outbound_connections) =
            match self.get_metrics(&input.latest_baseline_metrics) {
                Ok((inbound_connections, outbound_connections)) => {
                    (inbound_connections, outbound_connections)
                }
                Err(_) => {
                    return Err(MetricsEvaluatorError::MissingBaselineMetric(
                        METRIC.to_string(),
                        "The baseline node is missing the required metric".to_string(),
                    ))
                }
            };

        let (target_inbound_connections, target_outbound_connections) =
            match self.get_metrics(&input.latest_target_metrics) {
                Ok((inbound_connections, outbound_connections)) => {
                    (inbound_connections, outbound_connections)
                }
                Err(evaluation_results) => return Ok(evaluation_results),
            };

        Ok(vec![
            self.build_evaluation(
                baseline_inbound_connections,
                target_inbound_connections,
                self.args.inbound_peers_tolerance,
                &ConnectionType::Inbound,
            ),
            self.build_evaluation(
                baseline_outbound_connections,
                target_outbound_connections,
                self.args.outbound_peers_tolerance,
                &ConnectionType::Outbound,
            ),
        ])
    }

    fn get_category_name() -> String {
        CATEGORY.to_string()
    }

    fn get_evaluator_name() -> String {
        "peers_within_tolerance".to_string()
    }

    fn from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<Self> {
        Ok(Self::new(
            evaluator_args.network_peers_tolerance_args.clone(),
        ))
    }

    fn evaluator_type_from_evaluator_args(evaluator_args: &EvaluatorArgs) -> Result<EvaluatorType> {
        Ok(EvaluatorType::Metrics(Box::new(Self::from_evaluator_args(
            evaluator_args,
        )?)))
    }
}

impl NetworkPeersEvaluator for NetworkPeersWithinToleranceEvaluator {}
