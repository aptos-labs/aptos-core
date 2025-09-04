// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckResult, Checker, CheckerError, CommonCheckerConfig};
/// These Checkers are only valuable in certain contexts. For example, this is
/// not a useful Checker for node registration for the AITs, since each node
/// is running in their own isolated network, where no consensus is occurring.
/// This is useful for the AIT itself though, where the nodes are participating
/// in a real network.
use crate::{
    get_provider,
    provider::{
        metrics::{get_metric, GetMetricResult, Label, MetricsProvider},
        Provider, ProviderCollection,
    },
};
use anyhow::Result;
use once_cell::sync::Lazy;
use prometheus_parse::Scrape;
use serde::{Deserialize, Serialize};

/// Checker for minimum number of peers.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MinimumPeersCheckerConfig {
    #[serde(flatten)]
    pub common: CommonCheckerConfig,

    /// The minimum number of inbound connections required to be able to pass.
    /// For fullnodes, it only matters that this is greater than zero if the
    /// node operator wants to seed data to other nodes.
    #[serde(default = "MinimumPeersCheckerConfig::default_minimum_peers_inbound")]
    pub minimum_peers_inbound: u64,

    /// The minimum number of outbound connections required to be able to pass.
    /// This must be greater than zero for the node to be able to synchronize.
    #[serde(default = "MinimumPeersCheckerConfig::default_minimum_peers_outbound")]
    pub minimum_peers_outbound: u64,
}

impl MinimumPeersCheckerConfig {
    pub fn default_minimum_peers_inbound() -> u64 {
        0
    }

    pub fn default_minimum_peers_outbound() -> u64 {
        1
    }
}

#[derive(Debug)]
pub struct MinimumPeersChecker {
    config: MinimumPeersCheckerConfig,
}

impl MinimumPeersChecker {
    pub fn new(config: MinimumPeersCheckerConfig) -> Self {
        Self { config }
    }

    #[allow(clippy::comparison_chain)]
    fn build_evaluation(
        &self,
        connections: u64,
        minimum: u64,
        connection_type: &ConnectionType,
    ) -> CheckResult {
        let name = connection_type.get_name();
        let particle = connection_type.get_particle();
        let opposite_particle = connection_type.get_opposite_particle();
        let explanation = format!(
            "There are {} {} connections {} other nodes {} the target node (the minimum is {}).",
            connections, name, particle, opposite_particle, minimum
        );
        if connections >= minimum {
            Self::build_result(
                format!(
                    "There are sufficient {} connections {} the target node",
                    name, opposite_particle
                ),
                100,
                explanation,
            )
        } else {
            let additional_info = match connection_type {
                ConnectionType::Inbound => "This means that no downstream nodes are connected to your node.",
                ConnectionType::Outbound => "This means your node is not connected to upstream nodes and therefore cannot state sync. Try setting explicit peers.",
            };
            Self::build_result(
                format!(
                    "There are not enough {} connections {} the target node",
                    name, opposite_particle
                ),
                50,
                format!("{} {}", explanation, additional_info),
            )
            .links(vec!["https://velor.dev/issues-and-workarounds/".to_string()])
        }
    }
}

#[async_trait::async_trait]
impl Checker for MinimumPeersChecker {
    async fn check(
        &self,
        providers: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let target_metrics_provider = get_provider!(
            providers.target_metrics_provider,
            self.config.common.required,
            MetricsProvider
        );
        let scrape = match target_metrics_provider.provide().await {
            Ok(scrape) => scrape,
            Err(e) => {
                return Ok(vec![Self::build_result(
                    "Failed to check node peers".to_string(),
                    0,
                    format!("Failed to scrape metrics from your node: {:#}", e),
                )])
            },
        };
        let (inbound_connections, outbound_connections) = match get_metrics(&scrape) {
            Ok((inbound_connections, outbound_connections)) => {
                (inbound_connections, outbound_connections)
            },
            Err(evaluation_results) => return Ok(evaluation_results),
        };

        Ok(vec![
            self.build_evaluation(
                inbound_connections,
                self.config.minimum_peers_inbound,
                &ConnectionType::Inbound,
            ),
            self.build_evaluation(
                outbound_connections,
                self.config.minimum_peers_outbound,
                &ConnectionType::Outbound,
            ),
        ])
    }
}

//////////////////////////////////////////////////////////////////////////////
// Helpers.
//////////////////////////////////////////////////////////////////////////////

const METRIC: &str = "velor_connections";

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

/// Given a Scrape, pull the metrics telling us the number of inbound and
/// outbound connections.
fn get_metrics(metrics: &Scrape) -> Result<(u64, u64), Vec<CheckResult>> {
    let result_on_missing_fn = || {
        MinimumPeersChecker::build_result(
            "Could not determine result".to_string(),
            0,
            format!("The metrics from the node are missing the key: {}", METRIC),
        )
    };
    let (inbound, outbound) = (
        get_metric(metrics, METRIC, Some(&INBOUND_LABEL), result_on_missing_fn),
        get_metric(metrics, METRIC, Some(&OUTBOUND_LABEL), result_on_missing_fn),
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
