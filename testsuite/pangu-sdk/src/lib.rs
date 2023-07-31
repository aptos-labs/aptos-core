// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
#[allow(unused_imports)]
use std::fs::File;
#[allow(unused_imports)]
use std::io::{Error, ErrorKind, Read, Write};
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

//
// Related to get_testnets()
/////////////////////////////////////////
#[derive(Debug, Deserialize)]
pub struct TestnetHeaders {
    pub headers: Vec<String>,
    pub testnets: Vec<TestnetEntry>,
}

#[derive(Debug, Deserialize)]
pub struct TestnetEntry {
    pub name: String,
    pub status: String,
    pub age: String,
    pub nodes: String,
}
/////////////////////////////////////////

//
// Related to get_testnet()
/////////////////////////////////////////
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Pod {
    name: String,
    ready: i32,
    age: String,
    nodes: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PodList {
    headers: Vec<String>,
    pods: Option<Vec<Pod>>,
}
/////////////////////////////////////////

//
// Related to healthcheck()
/////////////////////////////////////////
#[allow(dead_code)]
#[derive(Debug)]
struct HealthcheckSummary {
    total: u16,
    passed: u16,
    failed: u16,
    unaccounted: u16,
}

impl HealthcheckSummary {
    fn from_stdout(stdout: &str) -> Option<Self> {
        let re =
            regex::Regex::new(r"\|\s+(\d+)\s+\|\s+(\d+)\s+\|\s+(\d+)\s+\|\s+(\d+)\s+\|").unwrap();
        if let Some(captures) = re.captures(stdout) {
            let total = captures[1].parse().ok()?;
            let passed = captures[2].parse().ok()?;
            let failed = captures[3].parse().ok()?;
            let unaccounted = captures[4].parse().ok()?;
            Some(HealthcheckSummary {
                total,
                passed,
                failed,
                unaccounted,
            })
        } else {
            None
        }
    }
}
/////////////////////////////////////////

//
// Related to serialize/deserialize PanguNodeBlueprint
#[derive(Serialize, Deserialize, Clone)]
struct PanguNodeBlueprint {
    validator_config_path: String,
    validator_image: String,
    validator_storage_class_name: String,
    vfn_config_path: String,
    vfn_image: String,
    vfn_storage_class_name: String,
    nodes_persistent_volume_claim_size: String,
    create_vfns: bool,
    stake_amount: u128,
    count: i16,
}

#[derive(Serialize, Deserialize)]
struct BlueprintCollection {
    blueprints: BTreeMap<String, PanguNodeBlueprint>,
}
/////////////////////////////////////////

struct PanguSDK;
impl PanguSDK {
    //
    // This is a light Rust wrapper around the Pangu CLI. It is not fully feature complete.

    #[allow(dead_code)]
    pub fn create_testnet(
        pangu_node_configs: Option<&BlueprintCollection>,
        pangu_node_configs_path: Option<&str>,
        num_of_validators: Option<i32>,
        layout_path: Option<&str>,
        workspace: Option<&str>,
        framework_path: Option<&str>,
        aptos_cli_path: Option<&str>,
        dry_run: Option<bool>,
        name: Option<&str>,
    ) {
        let pangu_dir = Self::pangu_directory();
        let temp_file: NamedTempFile;
        let pangu_node_configs_path_value: String =
            match (pangu_node_configs, pangu_node_configs_path) {
                (Some(bp), None) => {
                    temp_file = Self::create_pangu_node_config(bp)
                        .expect("Failed to create temporary file");
                    temp_file.path().to_string_lossy().to_string()
                },
                (None, Some(path)) => path.to_string(),
                _ => String::new(),
            };
        let mut python_command = Command::new("poetry");
        python_command
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("create");

        if !pangu_node_configs_path_value.is_empty() {
            python_command
                .arg("--pangu-node-configs-path")
                .arg(&pangu_node_configs_path_value);
        } else if let Some(num_of_validators_value) = num_of_validators {
            python_command
                .arg("--num-of-validators")
                .arg(num_of_validators_value.to_string());
        }

        if let Some(layout_path_value) = layout_path {
            python_command.arg("--layout-path").arg(layout_path_value);
        }

        if let Some(workspace_value) = workspace {
            python_command.arg("--workspace").arg(workspace_value);
        }

        if let Some(framework_path_value) = framework_path {
            python_command
                .arg("--framework-path")
                .arg(framework_path_value);
        }

        if let Some(aptos_cli_path_value) = aptos_cli_path {
            python_command
                .arg("--aptos-cli-path")
                .arg(aptos_cli_path_value);
        }

        if let Some(dry_run_value) = dry_run {
            python_command
                .arg("--dry-run")
                .arg(dry_run_value.to_string());
        }

        if let Some(name_value) = name {
            python_command.arg("--name").arg(name_value);
        }

        let command_output = python_command
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();

        if command_output.status.success() {
            println!("Testnet creation initiated successfully.");
        } else {
            eprintln!("Failed to create testnet: \n{} \n{}", stderr, stdout);
        }
    }

    #[allow(dead_code)]
    pub fn update_testnet(
        testnet_name: &str,
        pangu_node_configs: Option<&BlueprintCollection>,
        pangu_node_configs_path: Option<&str>,
    ) {
        let pangu_dir = Self::pangu_directory();
        let temp_file;
        let pangu_node_configs_path_value = match (pangu_node_configs, pangu_node_configs_path) {
            (Some(bp), None) => {
                temp_file =
                    Self::create_pangu_node_config(bp).expect("Failed to create temporary file");
                temp_file.path().to_string_lossy().to_string()
            },
            (None, Some(path)) => path.to_string(),
            _ => panic!("You must provide either a blueprint or a file path."),
        };
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("update")
            .arg(testnet_name)
            .arg(pangu_node_configs_path_value)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();

        if command_output.status.success() {
            println!("Initiated to update testnet \"{}\".", testnet_name);
        } else {
            eprintln!(
                "Failed to update testnet \"{}\": \n{} \n {}",
                testnet_name, stderr, stdout
            );
        }
    }

    #[allow(dead_code)]
    pub fn add_pfn(
        testnet_name: &str,
        pfn_name: &str,
        pfn_config_path: &str,
        image: Option<&str>,
        workspace: Option<&str>,
        storage_class_name: Option<&str>,
        storage_size: Option<&str>,
    ) -> Result<String, String> {
        let pangu_dir = Self::pangu_directory();
        let mut python_command = Command::new("poetry");
        python_command
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("node")
            .arg("add-pfn")
            .arg(testnet_name)
            .arg(pfn_name)
            .arg(pfn_config_path);

        if let Some(image_value) = image {
            python_command.arg("--image").arg(image_value);
        }

        if let Some(workspace_value) = workspace {
            python_command.arg("--workspace").arg(workspace_value);
        }

        if let Some(storage_class_name_value) = storage_class_name {
            python_command
                .arg("--storage-class-name")
                .arg(storage_class_name_value);
        }

        if let Some(storage_size_value) = storage_size {
            python_command.arg("--storage-size").arg(storage_size_value);
        }

        let command_output = python_command
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();

        if command_output.status.success() {
            Ok(format!(
                "Initiated to add pfn \"{}\"-pfn in testnet \"{}\".",
                pfn_name, testnet_name
            ))
        } else {
            Err(format!(
                "Failed to add pfn \"{}\"-pfn in testnet \"{}\": \n{} \n {}",
                pfn_name, testnet_name, stderr, stdout
            ))
        }
    }

    #[allow(dead_code)]
    pub fn wipe_node(testnet_name: &str, node_name: &str) -> Result<String, String> {
        Self::node_action_helper("wipe", testnet_name, node_name)
    }

    #[allow(dead_code)]
    pub fn restart_node(testnet_name: &str, node_name: &str) -> Result<String, String> {
        Self::node_action_helper("restart", testnet_name, node_name)
    }

    #[allow(dead_code)]
    pub fn stop_node(testnet_name: &str, node_name: &str) -> Result<String, String> {
        Self::node_action_helper("stop", testnet_name, node_name)
    }

    #[allow(dead_code)]
    pub fn start_node(testnet_name: &str, node_name: &str) -> Result<String, String> {
        Self::node_action_helper("start", testnet_name, node_name)
    }

    #[allow(dead_code)]
    pub fn restart_nodes_in_testnet(testnet_name: &str) -> Result<String, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("restart")
            .arg(testnet_name)
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();

        if command_output.status.success() {
            Ok(format!("Restarting nodes in testnet \"{}\".", testnet_name))
        } else {
            Err(format!(
                "Failed to restart testnet \"{}\": \n{}",
                testnet_name, stderr
            ))
        }
    }

    #[allow(dead_code)]
    pub fn healthcheck_testnet(
        testnet_name: &str,
        endpoint_name: &str,
    ) -> Result<HealthcheckSummary, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("healthcheck")
            .arg(testnet_name)
            .arg("--endpoint-name")
            .arg(endpoint_name)
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();

        if command_output.status.success() {
            if let Some(summary) = HealthcheckSummary::from_stdout(&stdout) {
                Ok(summary)
            } else {
                Err("Failed to parse healthcheck summary from stdout.".to_string())
            }
        } else {
            Err(format!(
                "Failed to healthcheck testnet \"{}\": \n{}",
                testnet_name, stderr
            ))
        }
    }

    #[allow(dead_code)]
    pub fn delete_testnet(testnet_name: &str, wait_deletion: bool) -> Result<String, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("delete")
            .arg(testnet_name)
            .arg("--wait_deletion")
            .arg(wait_deletion.to_string())
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();

        if command_output.status.success() && !wait_deletion {
            Ok(format!(
                "Deletion of the testnet \"{}\" has been initiated.",
                testnet_name
            ))
        } else if command_output.status.success() {
            Ok(format!(
                "Deletion of the testnet \"{}\" is successful.",
                testnet_name
            ))
        } else {
            Err(format!(
                "Failed to delete testnet \"{}\": \n{}",
                testnet_name, stderr
            ))
        }
    }

    #[allow(dead_code)]
    pub fn get_testnet(testnet_name: &str) -> Result<PodList, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("get")
            .arg(testnet_name)
            .arg("-o")
            .arg("json")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();

        if command_output.status.success() {
            serde_json::from_str(&stdout)
                .map_err(|err| {
                    eprintln!("Failed to deserialize JSON: {}", err);
                    format!("Failed to deserialize JSON: {}", err)
                })
                .map_err(|err| {
                    eprintln!(
                        "Failed to get testnet {}: \n{}, {}",
                        testnet_name, stderr, err
                    );
                    format!("Failed to get testnet {}: \n{}", testnet_name, stderr)
                })
        } else {
            Err(format!(
                "Failed to get testnet {}: \n{}",
                testnet_name, stderr
            ))
        }
    }

    #[allow(dead_code)]
    pub fn get_testnets() -> Result<TestnetHeaders, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("testnet")
            .arg("get")
            .arg("-o")
            .arg("json")
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();

        if command_output.status.success() {
            let testnet_headers: TestnetHeaders = serde_json::from_str(&stdout).map_err(|err| {
                eprintln!("Failed to deserialize JSON: {}", err);
                format!("Failed to deserialize JSON: {}", err)
            })?;
            Ok(testnet_headers)
        } else {
            Err(format!("Failed to get testnets: {}", stderr))
        }
    }

    fn node_action_helper(
        action: &str,
        testnet_name: &str,
        node_name: &str,
    ) -> Result<String, String> {
        let pangu_dir = Self::pangu_directory();
        let command_output = Command::new("poetry")
            .arg("-C")
            .arg(&pangu_dir)
            .arg("run")
            .arg("python")
            .arg(format!("{}/pangu.py", pangu_dir))
            .arg("node")
            .arg(action)
            .arg(testnet_name)
            .arg(node_name)
            .stderr(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let stderr = String::from_utf8_lossy(&command_output.stderr).to_string();
        let stdout = String::from_utf8_lossy(&command_output.stdout).to_string();

        if command_output.status.success() {
            Ok(format!(
                "Initiated to {} node \"{}\" in testnet \"{}\".",
                action, node_name, testnet_name
            ))
        } else {
            Err(format!(
                "Failed to {} node \"{}\" in testnet \"{}\": \n{}, \n{}",
                action, node_name, testnet_name, stderr, stdout
            ))
        }
    }

    fn create_pangu_node_config(
        blueprint: &BlueprintCollection,
    ) -> Result<NamedTempFile, Box<dyn std::error::Error>> {
        let yaml_data = serde_yaml::to_string(blueprint)?;
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(yaml_data.as_bytes())?;
        Ok(temp_file)
    }

    fn pangu_directory() -> String {
        if let Ok(pangu_dir) = env::var("PANGU_DIR") {
            pangu_dir
        } else {
            panic!("Failed to retrieve PANGU_DIR environment variable.");
        }
    }
}

#[cfg(test)]
mod tests {

    #[cfg(feature = "integration-tests")]
    fn test_create_testnet() {
        env::set_var(
            "PANGU_DIR",
            "/Users/olsenbudanur/Desktop/aptos-repos/aptos-core/testsuite",
        );
        PanguSDK::create_testnet(
            None,
            None,
            Some(3),
            None,
            None,
            None,
            None,
            None,
            Some("olsen2"),
        );
    }

    #[cfg(feature = "integration-tests")]
    fn test_update_testnet() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        PanguSDK::update_testnet(
            "pangu-olsen",
            None,
            Some("/Users/olsenbudanur/Desktop/project/testnet-deployment/pangu_lib/template_testnet_files/pangu_node_config.yaml"),
        )
    }

    #[cfg(feature = "integration-tests")]
    fn test_add_pfn() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::add_pfn(
            "pangu-olsen",
            "pfn-olsen",
            "/Users/olsenbudanur/Desktop/aptos-repos/aptos-core/testsuite/pangu_lib/template_testnet_files/pfn.yaml",
            None,
            None,
            None,
            None,
        ) {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_create_pangu_node_config() {
        let blueprint1 = PanguNodeBlueprint {
            validator_config_path: "validator_config_path".to_string(),
            validator_image: "validator_image".to_string(),
            validator_storage_class_name: "validator_storage_class_name".to_string(),
            vfn_config_path: "vfn_config_path".to_string(),
            vfn_image: "vfn_image".to_string(),
            vfn_storage_class_name: "vfn_storage_class_name".to_string(),
            nodes_persistent_volume_claim_size: "nodes_persistent_volume_claim_size".to_string(),
            create_vfns: true,
            stake_amount: 100000000000000,
            count: 200,
        };

        let blueprint2 = PanguNodeBlueprint {
            validator_config_path: "asd".to_string(),
            validator_image: "asd".to_string(),
            validator_storage_class_name: "asd".to_string(),
            vfn_config_path: "s".to_string(),
            vfn_image: "d".to_string(),
            vfn_storage_class_name: "d".to_string(),
            nodes_persistent_volume_claim_size: "d".to_string(),
            create_vfns: false,
            stake_amount: 100000000000000,
            count: 100,
        };

        let mut blueprint_collection = BlueprintCollection {
            blueprints: BTreeMap::new(),
        };

        blueprint_collection
            .blueprints
            .insert("nodebp".to_string(), blueprint1);
        blueprint_collection
            .blueprints
            .insert("nodebp2".to_string(), blueprint2);

        if let Ok(temp_file) = PanguSDK::create_pangu_node_config(&blueprint_collection) {
            let file_path = temp_file.path().to_string_lossy().to_string();
            println!("Temporary YAML file created: {:?}", file_path);
            let mut file = std::fs::File::open(&file_path).expect("Error opening temporary file.");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Error reading from temporary file.");
            println!("Data written to temporary file:");
            println!("{}", contents);
        } else {
            eprintln!("Error writing to the temporary file.");
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_wipe_node() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::wipe_node("pangu-olsen", "nodebp-node-1-validator") {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_restart_node() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::restart_node("pangu-olsen", "nodebp-node-1-validator") {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_stop_node() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::stop_node("pangu-olsen", "nodebp-node-1-validator") {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_start_node() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::start_node("pangu-olsen", "nodebp-node-1-validator") {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_restart_nodes_in_testnet() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::restart_nodes_in_testnet("pangu-olsen") {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }

    // #[cfg(feature = "integration-tests")]
    #[test]
    fn test_healthcheck_testnet() {
        // let pangu_dir = env::var("PANGU_DIR");
        // assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");
        env::set_var(
            "PANGU_DIR",
            "/Users/olsenbudanur/Desktop/aptos-repos/aptos-core/testsuite",
        );

        match PanguSDK::healthcheck_testnet("pangu-o", "ledger_info") {
            Ok(summary) => {
                println!("{:#?}", summary);
            },
            Err(err) => {
                eprintln!("Failed to healthcheck testnets: {}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_get_testnets() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::get_testnets() {
            Ok(testnet_headers) => {
                println!("{:#?}", testnet_headers);
            },
            Err(err) => {
                eprintln!("Failed to get testnets: {}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_get_testnet() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::get_testnet("pangu-olsen") {
            Ok(testnet) => {
                println!("{:#?}", testnet);
            },
            Err(err) => {
                eprintln!("Failed to get testnet: {}", err);
            },
        }
    }

    #[cfg(feature = "integration-tests")]
    fn test_delete_testnet() {
        let pangu_dir = env::var("PANGU_DIR");
        assert!(pangu_dir.is_ok(), "PANGU_DIR environment variable is not set");

        match PanguSDK::delete_testnet("pangu-olsen", true) {
            Ok(message) => {
                println!("{}", message);
            },
            Err(err) => {
                eprintln!("{}", err);
            },
        }
    }
}
