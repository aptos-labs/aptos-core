// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    success_criteria::{MetricsThreshold, SuccessCriteria, SystemMetricsThreshold},
    *,
};
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_framework::ReleaseBundle;
use std::{num::NonZeroUsize, sync::Arc};

pub struct ForgeConfig {
    suite_name: Option<String>,

    pub aptos_tests: Vec<Box<dyn AptosTest>>,
    pub admin_tests: Vec<Box<dyn AdminTest>>,
    pub network_tests: Vec<Box<dyn NetworkTest>>,

    /// The initial number of validators to spawn when the test harness creates a swarm
    pub initial_validator_count: NonZeroUsize,

    /// The initial number of fullnodes to spawn when the test harness creates a swarm
    pub initial_fullnode_count: usize,

    /// The initial version to use when the test harness creates a swarm
    pub initial_version: InitialVersion,

    /// The initial genesis modules to use when starting a network
    pub genesis_config: Option<GenesisConfig>,

    /// Optional genesis helm values init function
    pub genesis_helm_config_fn: Option<GenesisConfigFn>,

    /// Optional validator node config override function
    pub validator_override_node_config_fn: Option<OverrideNodeConfigFn>,

    /// Optional fullnode node config override function
    pub fullnode_override_node_config_fn: Option<OverrideNodeConfigFn>,

    pub multi_region_config: bool,

    /// Transaction workload to run on the swarm
    pub emit_job_request: EmitJobRequest,

    /// Success criteria
    pub success_criteria: SuccessCriteria,

    /// The label of existing DBs to use, if None, will create new db.
    pub existing_db_tag: Option<String>,

    pub validator_resource_override: NodeResourceOverride,

    pub fullnode_resource_override: NodeResourceOverride,

    /// Retain debug logs and above for all nodes instead of just the first 5 nodes
    pub retain_debug_logs: bool,
}

impl ForgeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_aptos_test<T: AptosTest + 'static>(mut self, aptos_test: T) -> Self {
        self.aptos_tests.push(Box::new(aptos_test));
        self
    }

    pub fn get_suite_name(&self) -> Option<String> {
        self.suite_name.clone()
    }

    pub fn with_suite_name(mut self, suite_name: String) -> Self {
        self.suite_name = Some(suite_name);
        self
    }

    pub fn with_aptos_tests(mut self, aptos_tests: Vec<Box<dyn AptosTest>>) -> Self {
        self.aptos_tests = aptos_tests;
        self
    }

    pub fn add_admin_test<T: AdminTest + 'static>(mut self, admin_test: T) -> Self {
        self.admin_tests.push(Box::new(admin_test));
        self
    }

    pub fn with_admin_tests(mut self, admin_tests: Vec<Box<dyn AdminTest>>) -> Self {
        self.admin_tests = admin_tests;
        self
    }

    pub fn add_network_test<T: NetworkTest + 'static>(mut self, network_test: T) -> Self {
        self.network_tests.push(Box::new(network_test));
        self
    }

    pub fn with_network_tests(mut self, network_tests: Vec<Box<dyn NetworkTest>>) -> Self {
        self.network_tests = network_tests;
        self
    }

    pub fn with_initial_validator_count(mut self, initial_validator_count: NonZeroUsize) -> Self {
        self.initial_validator_count = initial_validator_count;
        self
    }

    pub fn with_initial_fullnode_count(mut self, initial_fullnode_count: usize) -> Self {
        self.initial_fullnode_count = initial_fullnode_count;
        self
    }

    pub fn with_genesis_helm_config_fn(mut self, genesis_helm_config_fn: GenesisConfigFn) -> Self {
        self.genesis_helm_config_fn = Some(genesis_helm_config_fn);
        self
    }

    pub fn with_validator_override_node_config_fn(mut self, f: OverrideNodeConfigFn) -> Self {
        self.validator_override_node_config_fn = Some(f);
        self
    }

    pub fn with_fullnode_override_node_config_fn(mut self, f: OverrideNodeConfigFn) -> Self {
        self.fullnode_override_node_config_fn = Some(f);
        self
    }

    pub fn with_multi_region_config(mut self) -> Self {
        self.multi_region_config = true;
        self
    }

    pub fn with_validator_resource_override(
        mut self,
        resource_override: NodeResourceOverride,
    ) -> Self {
        self.validator_resource_override = resource_override;
        self
    }

    pub fn with_fullnode_resource_override(
        mut self,
        resource_override: NodeResourceOverride,
    ) -> Self {
        self.fullnode_resource_override = resource_override;
        self
    }

    fn override_node_config_from_fn(config_fn: OverrideNodeConfigFn) -> OverrideNodeConfig {
        let mut override_config = NodeConfig::default();
        let mut base_config = NodeConfig::default();
        config_fn(&mut override_config, &mut base_config);
        OverrideNodeConfig::new(override_config, base_config)
    }

    /// Builds a function that can be used to override the default helm values for the validator and fullnode.
    /// If a configuration is intended to be set for all nodes, set the value in the default helm values file:
    /// testsuite/forge/src/backend/k8s/helm-values/aptos-node-default-values.yaml
    pub fn build_node_helm_config_fn(&self, retain_debug_logs: bool) -> Option<NodeConfigFn> {
        let validator_override_node_config = self
            .validator_override_node_config_fn
            .clone()
            .map(|config_fn| Self::override_node_config_from_fn(config_fn));
        let fullnode_override_node_config = self
            .fullnode_override_node_config_fn
            .clone()
            .map(|config_fn| Self::override_node_config_from_fn(config_fn));
        let multi_region_config = self.multi_region_config;
        let existing_db_tag = self.existing_db_tag.clone();
        let validator_resource_override = self.validator_resource_override;
        let fullnode_resource_override = self.fullnode_resource_override;

        // Override specific helm values. See reference: terraform/helm/aptos-node/values.yaml
        Some(Arc::new(move |helm_values: &mut serde_yaml::Value| {
            if let Some(override_config) = &validator_override_node_config {
                helm_values["validator"]["config"] = override_config.get_yaml().unwrap();
            }
            if let Some(override_config) = &fullnode_override_node_config {
                helm_values["fullnode"]["config"] = override_config.get_yaml().unwrap();
            }
            if multi_region_config {
                helm_values["multicluster"]["enabled"] = true.into();
                // Create headless services for validators and fullnodes.
                // Note: chaos-mesh will not work with clusterIP services.
                helm_values["service"]["validator"]["internal"]["type"] = "ClusterIP".into();
                helm_values["service"]["validator"]["internal"]["headless"] = true.into();
                helm_values["service"]["fullnode"]["internal"]["type"] = "ClusterIP".into();
                helm_values["service"]["fullnode"]["internal"]["headless"] = true.into();
            }
            if let Some(existing_db_tag) = &existing_db_tag {
                helm_values["validator"]["storage"]["labels"]["tag"] =
                    existing_db_tag.clone().into();
                helm_values["fullnode"]["storage"]["labels"]["tag"] =
                    existing_db_tag.clone().into();
            }

            // validator resource overrides
            if let Some(cpu_cores) = validator_resource_override.cpu_cores {
                helm_values["validator"]["resources"]["requests"]["cpu"] = cpu_cores.into();
                helm_values["validator"]["resources"]["limits"]["cpu"] = cpu_cores.into();
            }
            if let Some(memory_gib) = validator_resource_override.memory_gib {
                helm_values["validator"]["resources"]["requests"]["memory"] =
                    format!("{}Gi", memory_gib).into();
                helm_values["validator"]["resources"]["limits"]["memory"] =
                    format!("{}Gi", memory_gib).into();
            }
            if let Some(storage_gib) = validator_resource_override.storage_gib {
                helm_values["validator"]["storage"]["size"] = format!("{}Gi", storage_gib).into();
            }
            // fullnode resource overrides
            if let Some(cpu_cores) = fullnode_resource_override.cpu_cores {
                helm_values["fullnode"]["resources"]["requests"]["cpu"] = cpu_cores.into();
                helm_values["fullnode"]["resources"]["limits"]["cpu"] = cpu_cores.into();
            }
            if let Some(memory_gib) = fullnode_resource_override.memory_gib {
                helm_values["fullnode"]["resources"]["requests"]["memory"] =
                    format!("{}Gi", memory_gib).into();
                helm_values["fullnode"]["resources"]["limits"]["memory"] =
                    format!("{}Gi", memory_gib).into();
            }
            if let Some(storage_gib) = fullnode_resource_override.storage_gib {
                helm_values["fullnode"]["storage"]["size"] = format!("{}Gi", storage_gib).into();
            }

            if retain_debug_logs {
                helm_values["validator"]["podAnnotations"]["aptos.dev/min-log-level-to-retain"] =
                    serde_yaml::Value::String("debug".to_owned());
                helm_values["fullnode"]["podAnnotations"]["aptos.dev/min-log-level-to-retain"] =
                    serde_yaml::Value::String("debug".to_owned());
                helm_values["validator"]["rust_log"] = "debug,hyper=off".into();
                helm_values["fullnode"]["rust_log"] = "debug,hyper=off".into();
            }
            helm_values["validator"]["config"]["storage"]["rocksdb_configs"]
                ["enable_storage_sharding"] = true.into();
            helm_values["fullnode"]["config"]["storage"]["rocksdb_configs"]
                ["enable_storage_sharding"] = true.into();
            helm_values["validator"]["config"]["indexer_db_config"]["enable_event"] = true.into();
            helm_values["fullnode"]["config"]["indexer_db_config"]["enable_event"] = true.into();

            // enable optqs
            helm_values["validator"]["config"]["consensus"]["quorum_store"]
                ["enable_opt_quorum_store"] = true.into();
        }))
    }

    pub fn with_initial_version(mut self, initial_version: InitialVersion) -> Self {
        self.initial_version = initial_version;
        self
    }

    pub fn with_genesis_module_bundle(mut self, bundle: ReleaseBundle) -> Self {
        self.genesis_config = Some(GenesisConfig::Bundle(bundle));
        self
    }

    pub fn with_genesis_modules_path(mut self, genesis_modules: String) -> Self {
        self.genesis_config = Some(GenesisConfig::Path(genesis_modules));
        self
    }

    pub fn with_emit_job(mut self, emit_job_request: EmitJobRequest) -> Self {
        self.emit_job_request = emit_job_request;
        self
    }

    pub fn get_emit_job(&self) -> &EmitJobRequest {
        &self.emit_job_request
    }

    pub fn with_success_criteria(mut self, success_criteria: SuccessCriteria) -> Self {
        self.success_criteria = success_criteria;
        self
    }

    pub fn get_success_criteria_mut(&mut self) -> &mut SuccessCriteria {
        &mut self.success_criteria
    }

    pub fn with_existing_db(mut self, tag: String) -> Self {
        self.existing_db_tag = Some(tag);
        self
    }

    pub fn number_of_tests(&self) -> usize {
        self.admin_tests.len() + self.network_tests.len() + self.aptos_tests.len()
    }

    pub fn all_tests(&self) -> Vec<Box<AnyTestRef<'_>>> {
        self.admin_tests
            .iter()
            .map(|t| Box::new(AnyTestRef::Admin(t.as_ref())))
            .chain(
                self.network_tests
                    .iter()
                    .map(|t| Box::new(AnyTestRef::Network(t.as_ref()))),
            )
            .chain(
                self.aptos_tests
                    .iter()
                    .map(|t| Box::new(AnyTestRef::Aptos(t.as_ref()))),
            )
            .collect()
    }
}

impl Default for ForgeConfig {
    fn default() -> Self {
        let forge_run_mode = ForgeRunnerMode::try_from_env().unwrap_or(ForgeRunnerMode::K8s);
        let success_criteria = if forge_run_mode == ForgeRunnerMode::Local {
            SuccessCriteria::new(600).add_no_restarts()
        } else {
            SuccessCriteria::new(3500)
                .add_no_restarts()
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // Check that we don't use more than 12 CPU cores for 30% of the time.
                    MetricsThreshold::new(12.0, 30),
                    // Check that we don't use more than 10 GB of memory for 30% of the time.
                    MetricsThreshold::new_gb(10.0, 30),
                ))
        };
        Self {
            suite_name: None,
            aptos_tests: vec![],
            admin_tests: vec![],
            network_tests: vec![],
            initial_validator_count: NonZeroUsize::new(1).unwrap(),
            initial_fullnode_count: 0,
            initial_version: InitialVersion::Oldest,
            genesis_config: None,
            genesis_helm_config_fn: None,
            validator_override_node_config_fn: None,
            fullnode_override_node_config_fn: None,
            multi_region_config: false,
            emit_job_request: EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
                mempool_backlog: 40000,
            }),
            success_criteria,
            existing_db_tag: None,
            validator_resource_override: NodeResourceOverride::default(),
            fullnode_resource_override: NodeResourceOverride::default(),
            retain_debug_logs: false,
        }
    }
}
