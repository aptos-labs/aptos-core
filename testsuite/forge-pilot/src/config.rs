use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

/// Config for the old ForgeConfigs in S3
#[derive(Debug, Deserialize)]
pub struct ForgeConfig {
    enabled_clusters: Vec<String>,
    all_clusters: Vec<String>,
    test_suites: HashMap<String, Value>,
    default_helm_values: Value,
}

#[derive(Debug, Deserialize)]
pub struct TestSuite {
    name: String,
    all_tests: HashMap<String, TestConfig>,
    enabled_tests: HashMap<String, TestConfig>,
}

#[derive(Debug, Deserialize)]
pub struct TestConfig {
    name: String,
}
