// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_single_node_test_config, network};
use aptos_config::config::{NodeConfig, WaypointConfig};
use aptos_event_notifications::EventSubscriptionService;
use aptos_infallible::RwLock;
use aptos_storage_interface::{DbReader, DbReaderWriter, DbWriter};
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, waypoint::Waypoint};
use rand::SeedableRng;
use std::{fs, sync::Arc};

/// A mock database implementing DbReader and DbWriter
pub struct MockDatabase;
impl DbReader for MockDatabase {}
impl DbWriter for MockDatabase {}

#[test]
#[should_panic(expected = "Validator networks must always have mutual_authentication enabled!")]
fn test_mutual_authentication_validators() {
    // Create a default node config for the validator
    let temp_path = TempPath::new();
    let mut node_config = NodeConfig::get_default_validator_config();
    node_config.set_data_dir(temp_path.path().to_path_buf());
    node_config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());

    // Disable mutual authentication for the config
    let validator_network = node_config.validator_network.as_mut().unwrap();
    validator_network.mutual_authentication = false;

    // Create an event subscription service
    let mut event_subscription_service =
        EventSubscriptionService::new(Arc::new(RwLock::new(DbReaderWriter::new(MockDatabase {}))));

    // Set up the networks and gather the application network handles. This should panic.
    let peers_and_metadata = network::create_peers_and_metadata(&node_config);
    let _ = network::setup_networks_and_get_interfaces(
        &node_config,
        ChainId::test(),
        peers_and_metadata,
        &mut event_subscription_service,
    );
}

#[cfg(feature = "check-vm-features")]
#[test]
fn test_aptos_vm_does_not_have_test_natives() {
    aptos_vm::natives::assert_no_test_natives(crate::utils::ERROR_MSG_BAD_FEATURE_FLAGS)
}

// This test confirms that the overriding behavior works as intended.
#[test]
fn test_create_single_node_test_config() {
    // Create a test config override and merge it with the default config.
    // This will get cleaned up by the tempdir when it goes out of scope.
    let test_dir = aptos_temppath::TempPath::new().as_ref().to_path_buf();
    fs::DirBuilder::new()
        .recursive(true)
        .create(&test_dir)
        .expect("Failed to create test_dir");
    let config_override_path = test_dir.join("override.yaml");
    let config_override: serde_yaml::Value = serde_yaml::from_str(
        r#"
        storage:
            enable_indexer: true
        indexer_grpc:
            enabled: true
            address: 0.0.0.0:50053
            processor_task_count: 10
            processor_batch_size: 100
            output_batch_size: 100
        api:
            address: 0.0.0.0:8081
        execution:
            genesis_waypoint:
                from_config: "0:6072b68a942aace147e0655c5704beaa255c84a7829baa4e72a500f1516584c4"
        "#,
    )
    .unwrap();
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_override_path)
        .expect("Couldn't open file");
    serde_yaml::to_writer(f, &config_override).unwrap();

    // merge it
    let default_node_config = NodeConfig::get_default_validator_config();
    let merged_config = create_single_node_test_config(
        &None,
        &Some(config_override_path),
        &test_dir,
        false,
        false,
        false,
        aptos_cached_packages::head_release_bundle(),
        rand::rngs::StdRng::from_entropy(),
    )
    .unwrap();

    // overridden configs
    assert!(merged_config.storage.enable_indexer);
    assert!(merged_config.indexer_grpc.enabled);
    // default config is unchanged
    assert_eq!(
        merged_config
            .state_sync
            .state_sync_driver
            .bootstrapping_mode,
        default_node_config
            .state_sync
            .state_sync_driver
            .bootstrapping_mode
    );
}

#[test]
fn test_verifier_cache_enabled_for_aptos_node() {
    use std::process::{Command, Stdio};
    // Run the shell command `cargo tree -p aptos-node -e features`
    let output = Command::new("cargo")
        .arg("tree")
        .arg("-p")
        .arg("aptos-node")
        .arg("-e")
        .arg("features")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to execute `cargo tree -p aptos-node -e features`");
    let output = String::from_utf8_lossy(&output.stdout);

    let feature = "disable_verifier_cache";
    assert!(
        !output.contains(feature),
        "Feature `{}` should not be enabled for aptos-node",
        feature
    );
}
