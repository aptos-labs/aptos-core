// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::ForgeConfig,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, MetricsThreshold, StateProgressThreshold,
        SuccessCriteria, SystemMetricsThreshold,
    },
    AdminTest, AptosTest, EmitJobMode, EmitJobRequest, GenesisConfigFn, NetworkTest,
    NodeResourceOverride, OverrideNodeConfigFn,
};
use anyhow::{Context, Result};
use aptos_config::config::NodeConfig;
use aptos_transaction_workloads_lib::args::TransactionTypeArg;
use serde::{Deserialize, Serialize};
use std::{num::NonZeroUsize, path::Path, sync::Arc, time::Duration};

/// Components provided by the test code registry (not serializable)
pub struct TestCodeComponents {
    pub network_tests: Vec<Box<dyn NetworkTest>>,
    pub admin_tests: Vec<Box<dyn AdminTest>>,
    pub aptos_tests: Vec<Box<dyn AptosTest>>,
    /// Extra genesis helm config closure from the registry (composed with YAML overrides)
    pub extra_genesis_helm_config_fn: Option<GenesisConfigFn>,
    /// Extra validator override closure from the registry (composed with YAML overrides)
    pub extra_validator_override_fn: Option<OverrideNodeConfigFn>,
    /// Extra fullnode override closure from the registry (composed with YAML overrides)
    pub extra_fullnode_override_fn: Option<OverrideNodeConfigFn>,
}

impl Default for TestCodeComponents {
    fn default() -> Self {
        Self {
            network_tests: vec![],
            admin_tests: vec![],
            aptos_tests: vec![],
            extra_genesis_helm_config_fn: None,
            extra_validator_override_fn: None,
            extra_fullnode_override_fn: None,
        }
    }
}

/// Serializable forge test configuration that can be loaded from YAML files.
/// This contains all the "data" portions of a ForgeConfig that don't require
/// closures or trait objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgeTestConfig {
    /// Name that maps to a test code registry entry
    pub test_name: String,

    /// Number of validators
    #[serde(default = "default_validator_count")]
    pub initial_validator_count: usize,

    /// Number of validator fullnodes
    #[serde(default)]
    pub initial_fullnode_count: usize,

    /// Number of PFNs
    #[serde(default)]
    pub num_pfns: usize,

    /// Whether to enable multi-region config
    #[serde(default)]
    pub multi_region_config: bool,

    /// Whether to retain debug logs for all nodes
    #[serde(default)]
    pub retain_debug_logs: bool,

    /// Existing DB tag to use
    #[serde(default)]
    pub existing_db_tag: Option<String>,

    /// Genesis helm config overrides (merged into helm values)
    #[serde(default)]
    pub genesis_helm_config: Option<serde_yaml::Value>,

    /// Validator node config overrides (partial NodeConfig YAML merged onto defaults)
    #[serde(default)]
    pub validator_config_override: Option<serde_yaml::Value>,

    /// Fullnode node config overrides (partial NodeConfig YAML merged onto defaults)
    #[serde(default)]
    pub fullnode_config_override: Option<serde_yaml::Value>,

    /// Emit job configuration
    #[serde(default)]
    pub emit_job: Option<EmitJobConfig>,

    /// Success criteria
    pub success_criteria: SuccessCriteriaConfig,

    /// Validator resource overrides
    #[serde(default)]
    pub validator_resource_override: Option<NodeResourceOverride>,

    /// Fullnode resource overrides
    #[serde(default)]
    pub fullnode_resource_override: Option<NodeResourceOverride>,

    /// Extra test-specific parameters (consumed by the registry factory)
    #[serde(default)]
    pub extra: Option<serde_yaml::Value>,
}

fn default_validator_count() -> usize {
    1
}

/// Serializable emit job configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitJobConfig {
    /// Emit job mode
    pub mode: EmitJobModeConfig,

    /// Gas price override
    #[serde(default)]
    pub gas_price: Option<u64>,

    /// Init gas price multiplier
    #[serde(default)]
    pub init_gas_price_multiplier: Option<u64>,

    /// Transaction expiration time in seconds
    #[serde(default)]
    pub txn_expiration_time_secs: Option<u64>,

    /// Init expiration multiplier
    #[serde(default)]
    pub init_expiration_multiplier: Option<f64>,

    /// Latency polling interval in milliseconds
    #[serde(default)]
    pub latency_polling_interval_ms: Option<u64>,

    /// Single transaction type
    #[serde(default)]
    pub transaction_type: Option<TransactionTypeArg>,

    /// Mix of transaction types with weights
    #[serde(default)]
    pub transaction_mix: Option<Vec<TransactionMixEntry>>,
}

/// A single entry in a transaction mix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMixEntry {
    pub transaction_type: TransactionTypeArg,
    pub weight: usize,
}

/// Serializable emit job mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EmitJobModeConfig {
    MaxLoad { mempool_backlog: usize },
    ConstTps { tps: usize },
    WaveTps {
        average_tps: usize,
        wave_ratio: f32,
        num_waves: usize,
    },
}

/// Serializable success criteria config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessCriteriaConfig {
    pub min_avg_tps: f64,

    #[serde(default)]
    pub check_no_restarts: bool,

    #[serde(default = "default_check_no_errors")]
    pub check_no_errors: bool,

    #[serde(default)]
    pub check_no_fullnode_failures: bool,

    #[serde(default)]
    pub max_expired_tps: Option<f64>,

    #[serde(default)]
    pub max_failed_submission_tps: Option<f64>,

    #[serde(default)]
    pub wait_for_catchup_s: Option<u64>,

    #[serde(default)]
    pub latency_thresholds: Vec<LatencyThresholdEntry>,

    #[serde(default)]
    pub latency_breakdown_thresholds: Option<LatencyBreakdownThresholdConfig>,

    #[serde(default)]
    pub system_metrics: Option<SystemMetricsConfig>,

    #[serde(default)]
    pub chain_progress: Option<StateProgressThreshold>,
}

fn default_check_no_errors() -> bool {
    true
}

/// A single latency threshold entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyThresholdEntry {
    pub threshold_s: f32,
    pub latency_type: LatencyType,
}

/// Latency breakdown threshold config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBreakdownThresholdConfig {
    pub thresholds: Vec<LatencyBreakdownEntry>,
    #[serde(default)]
    pub max_breach_pct: usize,
}

/// A single latency breakdown entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyBreakdownEntry {
    pub slice: crate::prometheus_metrics::LatencyBreakdownSlice,
    pub max_s: f64,
}

/// System metrics threshold config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsConfig {
    pub cpu_threshold: MetricsThreshold,
    pub memory_threshold: MetricsThreshold,
}

impl ForgeTestConfig {
    /// Parse from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).context("Failed to parse ForgeTestConfig YAML")
    }

    /// Load from file path
    pub fn from_file(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::from_yaml(&contents)
    }

    /// Convert emit job config to EmitJobRequest
    fn build_emit_job_request(&self) -> EmitJobRequest {
        let Some(ref emit_config) = self.emit_job else {
            return EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
                mempool_backlog: 40000,
            });
        };

        let mut req = EmitJobRequest::default();

        // Set mode
        req = req.mode(match &emit_config.mode {
            EmitJobModeConfig::MaxLoad { mempool_backlog } => EmitJobMode::MaxLoad {
                mempool_backlog: *mempool_backlog,
            },
            EmitJobModeConfig::ConstTps { tps } => EmitJobMode::ConstTps { tps: *tps },
            EmitJobModeConfig::WaveTps {
                average_tps,
                wave_ratio,
                num_waves,
            } => EmitJobMode::WaveTps {
                average_tps: *average_tps,
                wave_ratio: *wave_ratio,
                num_waves: *num_waves,
            },
        });

        if let Some(gas_price) = emit_config.gas_price {
            req = req.gas_price(gas_price);
        }
        if let Some(mult) = emit_config.init_gas_price_multiplier {
            req = req.init_gas_price_multiplier(mult);
        }
        if let Some(secs) = emit_config.txn_expiration_time_secs {
            req = req.txn_expiration_time_secs(secs);
        }
        if let Some(mult) = emit_config.init_expiration_multiplier {
            req = req.init_expiration_multiplier(mult);
        }
        if let Some(ms) = emit_config.latency_polling_interval_ms {
            req = req.latency_polling_interval(Duration::from_millis(ms));
        }
        if let Some(ref txn_type) = emit_config.transaction_type {
            req = req.transaction_type(txn_type.materialize_default());
        }
        if let Some(ref mix) = emit_config.transaction_mix {
            let mix_vec: Vec<_> = mix
                .iter()
                .map(|e| (e.transaction_type.materialize_default(), e.weight))
                .collect();
            req = req.transaction_mix(mix_vec);
        }

        req
    }

    /// Convert success criteria config to SuccessCriteria
    fn build_success_criteria(&self) -> SuccessCriteria {
        let sc = &self.success_criteria;
        let mut criteria = SuccessCriteria::new_float(sc.min_avg_tps);

        if sc.check_no_restarts {
            criteria = criteria.add_no_restarts();
        }
        if !sc.check_no_errors {
            criteria = criteria.allow_errors();
        }
        if sc.check_no_fullnode_failures {
            criteria = criteria.add_no_fullnode_failures();
        }
        if let Some(max) = sc.max_expired_tps {
            criteria = criteria.add_max_expired_tps(max);
        }
        if let Some(max) = sc.max_failed_submission_tps {
            criteria = criteria.add_max_failed_submission_tps(max);
        }
        if let Some(secs) = sc.wait_for_catchup_s {
            criteria = criteria.add_wait_for_catchup_s(secs);
        }
        for entry in &sc.latency_thresholds {
            criteria = criteria.add_latency_threshold(entry.threshold_s, entry.latency_type.clone());
        }
        if let Some(ref breakdown) = sc.latency_breakdown_thresholds {
            let thresholds: Vec<_> = breakdown
                .thresholds
                .iter()
                .map(|e| (e.slice.clone(), e.max_s))
                .collect();
            criteria = criteria.add_latency_breakdown_threshold(
                LatencyBreakdownThreshold::new_with_breach_pct(thresholds, breakdown.max_breach_pct),
            );
        }
        if let Some(ref sys) = sc.system_metrics {
            criteria = criteria.add_system_metrics_threshold(SystemMetricsThreshold::new(
                sys.cpu_threshold.clone(),
                sys.memory_threshold.clone(),
            ));
        }
        if let Some(ref progress) = sc.chain_progress {
            criteria = criteria.add_chain_progress(progress.clone());
        }

        criteria
    }

    /// Build a genesis_helm_config_fn from the YAML overrides
    fn build_genesis_helm_config_fn(&self) -> Option<GenesisConfigFn> {
        self.genesis_helm_config.clone().map(|overrides| {
            Arc::new(move |helm_values: &mut serde_yaml::Value| {
                deep_merge_yaml(helm_values, &overrides);
            }) as GenesisConfigFn
        })
    }

    /// Build a validator override node config fn from partial NodeConfig YAML
    fn build_node_config_override_fn(
        override_yaml: &Option<serde_yaml::Value>,
    ) -> Option<OverrideNodeConfigFn> {
        override_yaml.clone().map(|yaml_override| {
            Arc::new(move |config: &mut NodeConfig, _base: &mut NodeConfig| {
                // Serialize current config to YAML value, merge, deserialize back
                let mut config_value =
                    serde_yaml::to_value(&*config).expect("NodeConfig must serialize");
                deep_merge_yaml(&mut config_value, &yaml_override);
                *config =
                    serde_yaml::from_value(config_value).expect("Merged NodeConfig must deserialize");
            }) as OverrideNodeConfigFn
        })
    }

    /// Assemble a ForgeConfig from this test config and test code components.
    /// The YAML-derived overrides run first, then any extra closures from the registry.
    pub fn to_forge_config(self, code: TestCodeComponents) -> Result<ForgeConfig> {
        let mut config = ForgeConfig::default();

        // Set data fields
        config.initial_validator_count =
            NonZeroUsize::new(self.initial_validator_count).unwrap_or(NonZeroUsize::new(1).unwrap());
        config.initial_fullnode_count = self.initial_fullnode_count;
        config.num_pfns = self.num_pfns;
        config.multi_region_config = self.multi_region_config;
        config.retain_debug_logs = self.retain_debug_logs;
        config.existing_db_tag = self.existing_db_tag.clone();
        config.emit_job_request = self.build_emit_job_request();
        config.success_criteria = self.build_success_criteria();

        if let Some(res) = self.validator_resource_override {
            config.validator_resource_override = res;
        }
        if let Some(res) = self.fullnode_resource_override {
            config.fullnode_resource_override = res;
        }

        // Build genesis helm config fn: compose YAML overrides + extra code
        let yaml_genesis_fn = self.build_genesis_helm_config_fn();
        config.genesis_helm_config_fn = compose_config_fns(yaml_genesis_fn, code.extra_genesis_helm_config_fn);

        // Build validator override fn: compose YAML overrides + extra code
        let yaml_validator_fn = Self::build_node_config_override_fn(&self.validator_config_override);
        config.validator_override_node_config_fn =
            compose_override_fns(yaml_validator_fn, code.extra_validator_override_fn);

        // Build fullnode override fn: compose YAML overrides + extra code
        let yaml_fullnode_fn = Self::build_node_config_override_fn(&self.fullnode_config_override);
        config.fullnode_override_node_config_fn =
            compose_override_fns(yaml_fullnode_fn, code.extra_fullnode_override_fn);

        // Set test objects from registry
        config.network_tests = code.network_tests;
        config.admin_tests = code.admin_tests;
        config.aptos_tests = code.aptos_tests;

        Ok(config)
    }
}

/// Compose two optional GenesisConfigFn closures (YAML first, then code)
fn compose_config_fns(
    yaml_fn: Option<GenesisConfigFn>,
    code_fn: Option<GenesisConfigFn>,
) -> Option<GenesisConfigFn> {
    match (yaml_fn, code_fn) {
        (None, None) => None,
        (Some(f), None) | (None, Some(f)) => Some(f),
        (Some(yaml_f), Some(code_f)) => Some(Arc::new(move |helm_values: &mut serde_yaml::Value| {
            yaml_f(helm_values);
            code_f(helm_values);
        })),
    }
}

/// Compose two optional OverrideNodeConfigFn closures (YAML first, then code)
fn compose_override_fns(
    yaml_fn: Option<OverrideNodeConfigFn>,
    code_fn: Option<OverrideNodeConfigFn>,
) -> Option<OverrideNodeConfigFn> {
    match (yaml_fn, code_fn) {
        (None, None) => None,
        (Some(f), None) | (None, Some(f)) => Some(f),
        (Some(yaml_f), Some(code_f)) => {
            Some(Arc::new(move |config: &mut NodeConfig, base: &mut NodeConfig| {
                yaml_f(config, base);
                code_f(config, base);
            }))
        },
    }
}

/// Recursively merge `source` YAML into `target`.
/// For mappings, values from source override target.
/// For non-mapping values, source replaces target.
fn deep_merge_yaml(target: &mut serde_yaml::Value, source: &serde_yaml::Value) {
    match (target, source) {
        (serde_yaml::Value::Mapping(target_map), serde_yaml::Value::Mapping(source_map)) => {
            for (key, source_val) in source_map {
                if let Some(target_val) = target_map.get_mut(key) {
                    deep_merge_yaml(target_val, source_val);
                } else {
                    target_map.insert(key.clone(), source_val.clone());
                }
            }
        },
        (target, source) => {
            *target = source.clone();
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deep_merge_yaml() {
        let mut target: serde_yaml::Value = serde_yaml::from_str(
            r#"
            chain:
              epoch_duration_secs: 300
              name: test
            validator:
              count: 4
            "#,
        )
        .unwrap();

        let source: serde_yaml::Value = serde_yaml::from_str(
            r#"
            chain:
              epoch_duration_secs: 600
            validator:
              memory: 8Gi
            "#,
        )
        .unwrap();

        deep_merge_yaml(&mut target, &source);

        assert_eq!(
            target["chain"]["epoch_duration_secs"],
            serde_yaml::Value::Number(600.into())
        );
        assert_eq!(
            target["chain"]["name"],
            serde_yaml::Value::String("test".to_string())
        );
        assert_eq!(
            target["validator"]["count"],
            serde_yaml::Value::Number(4.into())
        );
        assert_eq!(
            target["validator"]["memory"],
            serde_yaml::Value::String("8Gi".to_string())
        );
    }

    #[test]
    fn test_parse_simple_yaml_config() {
        let yaml = r#"
test_name: simple_validator_upgrade
initial_validator_count: 4
success_criteria:
  min_avg_tps: 5000
  wait_for_catchup_s: 240
genesis_helm_config:
  chain:
    epoch_duration_secs: 60
"#;
        let config = ForgeTestConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.test_name, "simple_validator_upgrade");
        assert_eq!(config.initial_validator_count, 4);
        assert_eq!(config.success_criteria.min_avg_tps, 5000.0);
        assert_eq!(config.success_criteria.wait_for_catchup_s, Some(240));
        assert!(config.genesis_helm_config.is_some());
    }

    #[test]
    fn test_parse_config_with_emit_job() {
        let yaml = r#"
test_name: performance_test
initial_validator_count: 7
emit_job:
  mode:
    type: ConstTps
    tps: 5000
  gas_price: 500
success_criteria:
  min_avg_tps: 4500
  check_no_restarts: true
  latency_thresholds:
    - threshold_s: 3.5
      latency_type: P50
    - threshold_s: 4.5
      latency_type: P90
"#;
        let config = ForgeTestConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.initial_validator_count, 7);
        let emit = config.emit_job.as_ref().unwrap();
        assert!(matches!(emit.mode, EmitJobModeConfig::ConstTps { tps: 5000 }));
        assert_eq!(emit.gas_price, Some(500));
        assert_eq!(config.success_criteria.latency_thresholds.len(), 2);
    }
}
