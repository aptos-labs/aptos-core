use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, process::Command};

/// Config for the old ForgeConfigs in S3
#[derive(Debug, Deserialize)]
pub struct ForgeConfig {
    enabled_clusters: Vec<String>,
    all_clusters: Vec<String>,
    test_suites: HashMap<String, Value>,
    default_helm_values: Value,
}

impl ForgeConfig {
    /// Reads the ForgeConfig from a file
    pub fn read_from_file(path: &str) -> ForgeConfig {
        let json = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// Reads the ForgeConfig from S3
    pub fn read_from_s3(path: &str) -> ForgeConfig {
        let bucket = "forge-wrapper-config";
        let key = "forge-wrapper-config.json";
        let status = Command::new("aws")
            .args(&["s3", "cp", format!("s3://{}/{}", bucket, key).as_str(), "-"])
            .output()
            .expect("failed to execute process");
        let json = String::from_utf8_lossy(&status.stdout);
        serde_json::from_str(&json).unwrap()
    }

    /// Reads the ForgeConfig from GCS
    pub fn read_from_gcs(path: &str) -> ForgeConfig {
        todo!("Implement reading from GCS")
    }
}

/// Configures which tests are enabled for a given test suite
#[derive(Debug, Deserialize)]
pub struct TestSuite {
    name: String,
    all_tests: HashMap<String, TestConfig>,
    enabled_tests: HashMap<String, TestConfig>,
}

/// A single test
#[derive(Debug, Deserialize)]
pub struct TestConfig {
    name: String,
}
