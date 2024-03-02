// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{create_single_node_test_config, load_remote_config, network};
use aptos_config::{
    config::{NetworkConfig, NodeConfig, WaypointConfig},
    network_id::NetworkId,
};
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
        .create(true)
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
        aptos_cached_packages::head_release_bundle(),
        rand::rngs::StdRng::from_entropy(),
    )
    .unwrap();

    // overriden configs
    assert!(merged_config.storage.enable_indexer);
    assert!(merged_config.indexer_grpc.enabled);
    // default config is unchanged
    assert_eq!(
        merged_config
            .state_sync
            .state_sync_driver
            .continuous_syncing_mode,
        default_node_config
            .state_sync
            .state_sync_driver
            .continuous_syncing_mode
    );
}

#[test]
fn test_load_remote_config() {
    use claims::assert_ok_eq;

    aptos_logger::Logger::new()
        .level(aptos_logger::Level::Debug)
        .init();

    let mut initial_config = NodeConfig::default();
    initial_config.base.waypoint = WaypointConfig::FromFile("test".into());
    initial_config.validator_network = Some(NetworkConfig::network_with_id(NetworkId::Validator));
    let test_dir = aptos_temppath::TempPath::new().as_ref().to_path_buf();
    fs::DirBuilder::new()
        .recursive(true)
        .create(&test_dir.join("remote_configs"))
        .expect("Must be able to create temp directory");
    initial_config.set_data_dir(test_dir.clone());
    // assert_ok_eq!(load_remote_config(&initial_config), None);

    let config_override: serde_yaml::Value = serde_yaml::from_str(
        r#"
        storage:
            enable_indexer: true
        indexer_grpc:
            output_batch_size: 100
        dag_consensus:
            node_payload_config:
                max_sending_txns_per_round: 10
        "#,
    )
    .unwrap();
    let test_file = fs::File::create(
        test_dir
            .join("remote_configs")
            .join("remote_config_v1.yaml"),
    )
    .unwrap();
    serde_yaml::to_writer(test_file, &config_override).unwrap();

    let mut merged_configs = initial_config.clone();
    merged_configs.storage.enable_indexer = true;
    merged_configs.indexer_grpc.output_batch_size = 100;
    merged_configs
        .dag_consensus
        .node_payload_config
        .max_sending_txns_per_round = 10;

    // assert_eq!(
    //     serde_yaml::to_value(load_remote_config(&initial_config).unwrap().unwrap()).unwrap(),
    //     serde_yaml::to_value(merged_configs).unwrap()
    // );

    for i in 0..100 {
        let config_override: serde_yaml::Value = serde_yaml::from_str(&format!(
            "
            storage:
                enable_indexer: true
            indexer_grpc:
                output_batch_size: 100
            dag_consensus:
                node_payload_config:
                    max_sending_txns_per_round: {}
            ",
            i
        ))
        .unwrap();
        let test_file = fs::File::create(
            test_dir
                .join("remote_configs")
                .join(format!("remote_config_v{}.yaml", i)),
        )
        .unwrap();
        serde_yaml::to_writer(test_file, &config_override).unwrap();
    }

    // let merged_configs = load_remote_config(&initial_config).unwrap().unwrap();
    // assert_eq!(
    //     merged_configs
    //         .dag_consensus
    //         .node_payload_config
    //         .max_sending_txns_per_round,
    //     99
    // );
}
