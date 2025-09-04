// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    prometheus_metrics::{
        fetch_fullnode_failures, fetch_system_metrics, fetch_validator_error_metrics,
        LatencyBreakdown, LatencyBreakdownSlice, SystemMetrics,
    },
    result::TestResult,
    Swarm, SwarmExt, TestReport,
};
use anyhow::{bail, Context};
use velor::node::analyze::{analyze_validators::AnalyzeValidators, fetch_metadata::FetchMetadata};
use velor_logger::info as velor_logger_info;
use velor_transaction_emitter_lib::{TxnStats, TxnStatsRate};
use log::info;
use prometheus_http_query::response::Sample;
use serde_json::json;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Formatter},
    sync::Arc,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct StateProgressThreshold {
    pub max_non_epoch_no_progress_secs: f32,
    pub max_epoch_no_progress_secs: f32,
    pub max_non_epoch_round_gap: u64,
    pub max_epoch_round_gap: u64,
}

#[derive(Clone, Debug)]
pub enum LatencyType {
    Average,
    P50,
    P70,
    P90,
    P99,
}

#[derive(Default, Clone, Debug)]
pub struct MetricsThreshold {
    max: f64,
    // % of the data point that can breach the max threshold
    max_breach_pct: usize,

    expect_empty: bool,
}

impl MetricsThreshold {
    pub fn new(max: f64, max_breach_pct: usize) -> Self {
        Self {
            max,
            max_breach_pct,
            expect_empty: false,
        }
    }

    pub fn new_expect_empty() -> Self {
        Self {
            max: 0.0,
            max_breach_pct: 0,
            expect_empty: true,
        }
    }

    pub fn new_gb(max: f64, max_breach_pct: usize) -> Self {
        Self {
            max: max * 1024.0 * 1024.0 * 1024.0,
            max_breach_pct,
            expect_empty: false,
        }
    }

    pub fn ensure_metrics_threshold(
        &self,
        metrics_name: &str,
        metrics: &[Sample],
    ) -> anyhow::Result<()> {
        if self.expect_empty {
            if !metrics.is_empty() {
                bail!("Data found for metrics expected to be empty");
            }
            return Ok(());
        }

        if metrics.is_empty() {
            bail!("Empty metrics provided for {}", metrics_name);
        }
        let breach_count = metrics
            .iter()
            .filter(|sample| sample.value() > self.max)
            .count();
        let breach_pct = (breach_count * 100) / metrics.len();
        if breach_pct > self.max_breach_pct {
            bail!(
                "{:?} metric violated threshold of {:?}, max_breach_pct: {:?}, breach_pct: {:?} ",
                metrics_name,
                self.max,
                self.max_breach_pct,
                breach_pct
            );
        }
        Ok(())
    }
}

#[derive(Default, Clone, Debug)]
pub struct SystemMetricsThreshold {
    cpu_threshold: MetricsThreshold,
    memory_threshold: MetricsThreshold,
}

impl SystemMetricsThreshold {
    pub fn ensure_threshold(&self, metrics: &SystemMetrics) -> anyhow::Result<()> {
        self.cpu_threshold
            .ensure_metrics_threshold("cpu", metrics.cpu_core_metrics.get())?;
        self.memory_threshold
            .ensure_metrics_threshold("memory", metrics.memory_bytes_metrics.get())?;
        Ok(())
    }

    pub fn new(cpu_threshold: MetricsThreshold, memory_threshold: MetricsThreshold) -> Self {
        Self {
            cpu_threshold,
            memory_threshold,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LatencyBreakdownThreshold {
    pub thresholds: BTreeMap<LatencyBreakdownSlice, MetricsThreshold>,
}

impl LatencyBreakdownThreshold {
    pub fn new_strict(thresholds: Vec<(LatencyBreakdownSlice, f64)>) -> Self {
        Self::new_with_breach_pct(thresholds, 0)
    }

    pub fn new_with_breach_pct(
        thresholds: Vec<(LatencyBreakdownSlice, f64)>,
        max_breach_pct: usize,
    ) -> Self {
        Self {
            thresholds: thresholds
                .into_iter()
                .map(|(k, v)| (k, MetricsThreshold::new(v, max_breach_pct)))
                .collect(),
        }
    }

    pub fn ensure_threshold(
        &self,
        metrics: &LatencyBreakdown,
        traffic_name_addition: &String,
    ) -> anyhow::Result<()> {
        for (slice, threshold) in &self.thresholds {
            let samples = metrics
                .get_samples(slice)
                .expect("Could not get metric samples");
            threshold.ensure_metrics_threshold(
                &format!("{:?}{}", slice, traffic_name_addition),
                samples.get(),
            )?;
        }
        Ok(())
    }
}

#[derive(Default, Clone, Debug)]
pub struct SuccessCriteria {
    pub min_avg_tps: f64,
    latency_thresholds: Vec<(Duration, LatencyType)>,
    latency_breakdown_thresholds: Option<LatencyBreakdownThreshold>,
    check_no_restarts: bool,
    check_no_errors: bool,
    check_no_fullnode_failures: bool,
    max_expired_tps: Option<f64>,
    max_failed_submission_tps: Option<f64>,
    wait_for_all_nodes_to_catchup: Option<Duration>,
    // Maximum amount of CPU cores and memory bytes used by the nodes.
    system_metrics_threshold: Option<SystemMetricsThreshold>,
    chain_progress_check: Option<StateProgressThreshold>,
}

impl SuccessCriteria {
    pub fn new(min_avg_tps: usize) -> Self {
        Self::new_float(min_avg_tps as f64)
    }

    pub fn new_float(min_avg_tps: f64) -> Self {
        Self {
            min_avg_tps,
            latency_thresholds: Vec::new(),
            latency_breakdown_thresholds: None,
            check_no_restarts: false,
            check_no_errors: true,
            check_no_fullnode_failures: false,
            max_expired_tps: None,
            max_failed_submission_tps: None,
            wait_for_all_nodes_to_catchup: None,
            system_metrics_threshold: None,
            chain_progress_check: None,
        }
    }

    pub fn allow_errors(mut self) -> Self {
        self.check_no_errors = false;
        self
    }

    pub fn add_no_restarts(mut self) -> Self {
        self.check_no_restarts = true;
        self
    }

    pub fn add_no_fullnode_failures(mut self) -> Self {
        self.check_no_fullnode_failures = true;
        self
    }

    pub fn add_max_expired_tps(mut self, max_expired_tps: f64) -> Self {
        self.max_expired_tps = Some(max_expired_tps);
        self
    }

    pub fn add_max_failed_submission_tps(mut self, max_failed_submission_tps: f64) -> Self {
        self.max_failed_submission_tps = Some(max_failed_submission_tps);
        self
    }

    pub fn add_wait_for_catchup_s(mut self, duration_secs: u64) -> Self {
        self.wait_for_all_nodes_to_catchup = Some(Duration::from_secs(duration_secs));
        self
    }

    pub fn add_system_metrics_threshold(mut self, threshold: SystemMetricsThreshold) -> Self {
        self.system_metrics_threshold = Some(threshold);
        self
    }

    pub fn add_chain_progress(mut self, threshold: StateProgressThreshold) -> Self {
        self.chain_progress_check = Some(threshold);
        self
    }

    pub fn add_latency_threshold(mut self, threshold_s: f32, latency_type: LatencyType) -> Self {
        self.latency_thresholds
            .push((Duration::from_secs_f32(threshold_s), latency_type));
        self
    }

    pub fn add_latency_breakdown_threshold(mut self, threshold: LatencyBreakdownThreshold) -> Self {
        self.latency_breakdown_thresholds = Some(threshold);
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum CheckType {
    Hard,
    Soft,
}

#[derive(Debug)]
struct CheckError {
    name: String,
    check_type: CheckType,
    error: anyhow::Error,
}

impl CheckError {
    pub fn new(name: String, check_type: CheckType, error: anyhow::Error) -> Self {
        Self {
            name,
            check_type,
            error,
        }
    }
}

impl std::ops::Deref for CheckError {
    type Target = anyhow::Error;

    fn deref(&self) -> &Self::Target {
        &self.error
    }
}

#[derive(Debug, Default)]
pub struct SuccessCriteriaErrors {
    errors: Vec<CheckError>,
}

impl SuccessCriteriaErrors {
    fn push(&mut self, check_error: CheckError) {
        self.errors.push(check_error);
    }

    fn extend(&mut self, errors: SuccessCriteriaErrors) {
        self.errors.extend(errors.errors);
    }

    fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }
}

impl std::fmt::Display for SuccessCriteriaErrors {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut errors_json = Vec::new();

        for error in self.errors.iter() {
            let causes: Vec<String> = error.chain().map(|c| c.to_string()).collect();
            errors_json.push(json!({
                "name": error.name,
                "error": error.to_string(),
                "causes": causes
            }));
        }
        write!(f, "{}", json!({ "errors": errors_json }))?;
        Ok(())
    }
}

impl std::error::Error for SuccessCriteriaErrors {}

impl From<SuccessCriteriaErrors> for TestResult {
    fn from(errors: SuccessCriteriaErrors) -> Self {
        if errors.is_empty() {
            Self::Successful
        } else {
            let is_hard_failure = errors
                .errors
                .iter()
                .any(|e| e.check_type == CheckType::Hard);
            if is_hard_failure {
                Self::HardFailure(errors.to_string())
            } else {
                Self::SoftFailure(errors.to_string())
            }
        }
    }
}

#[derive(Default)]
pub struct SuccessCriteriaResults {
    check_errors: SuccessCriteriaErrors,
}

impl SuccessCriteriaResults {
    fn add_result(&mut self, name: &str, check_type: CheckType, result: anyhow::Result<()>) {
        if let Err(e) = result {
            self.check_errors
                .push(CheckError::new(name.to_string(), check_type, e))
        }
    }

    fn extend(&mut self, result: anyhow::Result<(), SuccessCriteriaErrors>) {
        if let Err(e) = result {
            self.check_errors.extend(e)
        }
    }

    fn evaluate(self) -> Result<(), SuccessCriteriaErrors> {
        if self.check_errors.is_empty() {
            Ok(())
        } else {
            Err(self.check_errors)
        }
    }
}

pub struct SuccessCriteriaChecker {}

impl SuccessCriteriaChecker {
    pub fn check_core_for_success(
        success_criteria: &SuccessCriteria,
        _report: &mut TestReport,
        stats_rate: &TxnStatsRate,
        latency_breakdown: Option<&LatencyBreakdown>,
        traffic_name: Option<String>,
    ) -> anyhow::Result<()> {
        let traffic_name_addition = traffic_name
            .map(|n| format!(" for {}", n))
            .unwrap_or_default();

        let mut results = SuccessCriteriaResults::default();

        results.extend(Self::check_throughput(
            success_criteria.min_avg_tps,
            success_criteria.max_expired_tps,
            success_criteria.max_failed_submission_tps,
            stats_rate,
            &traffic_name_addition,
        ));
        results.add_result("Check Latency", CheckType::Soft, {
            Self::check_latency(
                &success_criteria.latency_thresholds,
                stats_rate,
                &traffic_name_addition,
            )
        });
        results.add_result("Check Latency Breakdown", CheckType::Soft, {
            if let Some(latency_breakdown_thresholds) =
                &success_criteria.latency_breakdown_thresholds
            {
                latency_breakdown_thresholds
                    .ensure_threshold(latency_breakdown.unwrap(), &traffic_name_addition)
            } else {
                Ok(())
            }
        });

        results.evaluate()?;
        Ok(())
    }

    pub async fn check_for_success(
        success_criteria: &SuccessCriteria,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        stats: &TxnStats,
        window: Duration,
        latency_breakdown: &LatencyBreakdown,
        start_time: i64,
        end_time: i64,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<()> {
        info!(
            "End to end duration: {}s, performance measured for: {}s",
            window.as_secs(),
            stats.lasted.as_secs()
        );
        let stats_rate = stats.rate();

        let no_traffic_name_addition = "".to_string();

        let mut results = SuccessCriteriaResults::default();

        results.extend(Self::check_throughput(
            success_criteria.min_avg_tps,
            success_criteria.max_expired_tps,
            success_criteria.max_failed_submission_tps,
            &stats_rate,
            &no_traffic_name_addition,
        ));
        results.add_result(
            "Check Latency",
            CheckType::Soft,
            Self::check_latency(
                &success_criteria.latency_thresholds,
                &stats_rate,
                &no_traffic_name_addition,
            ),
        );
        if let Some(latency_breakdown_thresholds) = &success_criteria.latency_breakdown_thresholds {
            results.add_result(
                "Check Latency Breakdown",
                CheckType::Soft,
                latency_breakdown_thresholds
                    .ensure_threshold(latency_breakdown, &no_traffic_name_addition),
            );
        }
        if let Some(timeout) = success_criteria.wait_for_all_nodes_to_catchup {
            results.add_result("Check nodes catchup", CheckType::Hard, {
                swarm
                    .read()
                    .await
                    .wait_for_all_nodes_to_catchup_to_next(timeout)
                    .await
                    .context("Failed waiting for all nodes to catchup to next version")
            });
        }
        if success_criteria.check_no_restarts {
            results.add_result("Check no validator restart", CheckType::Hard, {
                let swarm_read = swarm.read().await;
                swarm_read
                    .ensure_no_validator_restart()
                    .await
                    .context("Failed ensuring no validator restarted")
            });

            results.add_result("Check no fullnode restart", CheckType::Hard, {
                let swarm_read = swarm.read().await;
                swarm_read
                    .ensure_no_fullnode_restart()
                    .await
                    .context("Failed ensuring no fullnode restarted")
            });
        }

        if success_criteria.check_no_errors {
            results.add_result("Check no error", CheckType::Hard, {
                Self::check_no_errors(swarm.clone()).await
            });
        }
        if success_criteria.check_no_fullnode_failures {
            results.add_result("Check no fullnode failures", CheckType::Hard, {
                Self::check_no_fullnode_failures(swarm.clone()).await
            });
        }
        if let Some(system_metrics_threshold) = success_criteria.system_metrics_threshold.clone() {
            results.add_result("Check system metrics", CheckType::Hard, {
                Self::check_system_metrics(
                    swarm.clone(),
                    start_time,
                    end_time,
                    system_metrics_threshold,
                )
                .await
            });
        }

        if let Some(chain_progress_threshold) = &success_criteria.chain_progress_check {
            results.add_result("Check chain progress", CheckType::Hard, {
                Self::check_chain_progress(
                    swarm.clone(),
                    report,
                    chain_progress_threshold,
                    start_version,
                    end_version,
                )
                .await
                .context("Failed check chain progress")
            });
        }

        results.evaluate()?;
        Ok(())
    }

    async fn check_chain_progress(
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        chain_progress_threshold: &StateProgressThreshold,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<()> {
        // Choose client with newest ledger version to fetch NewBlockEvents from:
        let (_max_v, client) = {
            swarm
                .read()
                .await
                .get_client_with_newest_ledger_version()
                .await
                .context("No clients replied in check_chain_progress")?
        };

        let epochs = FetchMetadata::fetch_new_block_events(&client, None, None)
            .await
            .unwrap();

        let gap_info = AnalyzeValidators::analyze_gap(
            epochs
                .iter()
                .flat_map(|epoch| epoch.blocks.iter())
                .filter(|b| b.version > start_version && b.version < end_version),
        );

        let gap_text = format!(
            "Max non-epoch-change gap was: {} [limit {}], {} [limit {}].",
            gap_info.non_epoch_round_gap.to_string_as_round(),
            chain_progress_threshold.max_non_epoch_round_gap,
            gap_info.non_epoch_time_gap.to_string_as_time(),
            chain_progress_threshold.max_non_epoch_no_progress_secs,
        );

        let epoch_gap_text = format!(
            "Max epoch-change gap was: {} [limit {}], {} [limit {}].",
            gap_info.epoch_round_gap.to_string_as_round(),
            chain_progress_threshold.max_epoch_round_gap,
            gap_info.epoch_time_gap.to_string_as_time(),
            chain_progress_threshold.max_epoch_no_progress_secs,
        );

        velor_logger_info!(
            max_non_epoch_round_gap = gap_info.non_epoch_round_gap.max_gap,
            max_epoch_round_gap = gap_info.epoch_round_gap.max_gap,
            max_non_epoch_time_gap = gap_info.non_epoch_time_gap.max_gap,
            max_epoch_time_gap = gap_info.epoch_time_gap.max_gap,
            "Max gap values",
        );

        report.report_text(gap_text.clone());
        report.report_text(epoch_gap_text.clone());

        if gap_info.non_epoch_round_gap.max_gap.round() as u64
            > chain_progress_threshold.max_non_epoch_round_gap
            || gap_info.non_epoch_time_gap.max_gap
                > chain_progress_threshold.max_non_epoch_no_progress_secs
        {
            bail!(
                "Failed non-epoch-change chain progress check. {}",
                &gap_text
            );
        }
        info!("Passed non-epoch-change progress check. {}", gap_text);

        if gap_info.epoch_round_gap.max_gap.round() as u64
            > chain_progress_threshold.max_epoch_round_gap
            || gap_info.epoch_time_gap.max_gap > chain_progress_threshold.max_epoch_no_progress_secs
        {
            bail!(
                "Failed epoch-change chain progress check. {}",
                &epoch_gap_text
            );
        }
        info!("Passed epoch-change progress check. {}", epoch_gap_text);

        Ok(())
    }

    pub fn check_tps(
        min_avg_tps: f64,
        stats_rate: &TxnStatsRate,
        traffic_name_addition: &String,
    ) -> anyhow::Result<()> {
        let avg_tps = stats_rate.committed;
        if avg_tps < min_avg_tps {
            bail!(
                "TPS requirement{} failed. Average TPS {}, minimum TPS requirement {}. Full stats: {}",
                traffic_name_addition,
                avg_tps,
                min_avg_tps,
                stats_rate,
            )
        } else {
            info!(
                "TPS is {} and is within limit of {}",
                stats_rate.committed, min_avg_tps
            );
            Ok(())
        }
    }

    fn check_max_value(
        max_config: Option<f64>,
        stats_rate: &TxnStatsRate,
        value: f64,
        value_desc: &str,
        traffic_name_addition: &String,
    ) -> anyhow::Result<()> {
        if let Some(max) = max_config {
            if value > max {
                bail!(
                    "{} requirement{} failed. {} TPS: average {}, maximum requirement {}. Full stats: {}",
                    value_desc,
                    traffic_name_addition,
                    value_desc,
                    value,
                    max,
                    stats_rate,
                )
            } else {
                info!(
                    "{} TPS is {} and is below max limit of {}",
                    value_desc, value, max
                );
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    pub fn check_throughput(
        min_avg_tps: f64,
        max_expired_config: Option<f64>,
        max_failed_submission_config: Option<f64>,
        stats_rate: &TxnStatsRate,
        traffic_name_addition: &String,
    ) -> anyhow::Result<(), SuccessCriteriaErrors> {
        let mut runner = SuccessCriteriaResults::default();

        runner.add_result(
            "Min avg tps",
            CheckType::Soft,
            Self::check_tps(min_avg_tps, stats_rate, traffic_name_addition),
        );
        runner.add_result(
            "Max expired tps",
            CheckType::Hard,
            Self::check_max_value(
                max_expired_config,
                stats_rate,
                stats_rate.expired,
                "expired",
                traffic_name_addition,
            ),
        );
        runner.add_result(
            "Max failed submission",
            CheckType::Soft,
            Self::check_max_value(
                max_failed_submission_config,
                stats_rate,
                stats_rate.failed_submission,
                "submission",
                traffic_name_addition,
            ),
        );
        runner.evaluate()
    }

    pub fn check_latency(
        latency_thresholds: &[(Duration, LatencyType)],
        stats_rate: &TxnStatsRate,
        traffic_name_addition: &String,
    ) -> anyhow::Result<()> {
        let mut failures = Vec::new();
        for (latency_threshold, latency_type) in latency_thresholds {
            let latency = Duration::from_millis(match latency_type {
                LatencyType::Average => stats_rate.latency as u64,
                LatencyType::P50 => stats_rate.p50_latency,
                LatencyType::P70 => stats_rate.p70_latency,
                LatencyType::P90 => stats_rate.p90_latency,
                LatencyType::P99 => stats_rate.p99_latency,
            });

            if latency > *latency_threshold {
                failures.push(
                    format!(
                        "{:?} latency{} is {}s and exceeds limit of {}s",
                        latency_type,
                        traffic_name_addition,
                        latency.as_secs_f32(),
                        latency_threshold.as_secs_f32()
                    )
                    .to_string(),
                );
            } else {
                info!(
                    "{:?} latency{} is {}s and is within limit of {}s",
                    latency_type,
                    traffic_name_addition,
                    latency.as_secs_f32(),
                    latency_threshold.as_secs_f32()
                );
            }
        }
        if !failures.is_empty() {
            bail!("Failed latency check, for {:?}", failures);
        } else {
            Ok(())
        }
    }

    /// Checks if there are any fullnode failures. Note: this currently
    /// only checks if consensus observer falls back to state sync.
    async fn check_no_fullnode_failures(
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> anyhow::Result<()> {
        let fullnode_failures = fetch_fullnode_failures(swarm).await?;
        if fullnode_failures > 0 {
            bail!(
                "Error! The number of fullnode failures was > 0 ({}), but must be 0!",
                fullnode_failures
            );
        } else {
            info!("No fullnode failures detected.");
            Ok(())
        }
    }

    async fn check_no_errors(
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    ) -> anyhow::Result<()> {
        let error_count = fetch_validator_error_metrics(swarm).await?;
        if error_count > 0 {
            bail!(
                "error!() count in validator logs was {}, and must be 0",
                error_count
            );
        } else {
            info!("No error!() found in validator logs");
            Ok(())
        }
    }

    async fn check_system_metrics(
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        start_time: i64,
        end_time: i64,
        threshold: SystemMetricsThreshold,
    ) -> anyhow::Result<()> {
        let system_metrics = fetch_system_metrics(swarm, start_time, end_time).await?;
        threshold.ensure_threshold(&system_metrics)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[tokio::test]
    async fn test_empty_metrics_threshold() {
        let cpu_threshold = MetricsThreshold::new(10.0, 30);
        let memory_threshold = MetricsThreshold::new(100.0, 40);
        let threshold = SystemMetricsThreshold::new(cpu_threshold, memory_threshold);
        let metrics = SystemMetrics::new(vec![], vec![]);
        threshold.ensure_threshold(&metrics).unwrap_err();
    }
}
