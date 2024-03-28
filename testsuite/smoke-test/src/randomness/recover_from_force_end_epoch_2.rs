// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, get_on_chain_resource, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    dkg::DKGState,
    on_chain_config::{FeatureFlag, Features},
};
use std::{sync::Arc, time::Duration};

/// If `version::set_version()` is accidentally called in epoch `e`,
/// Randomness should recover by an async reconfiguration, starting epoch `e+2`.
#[tokio::test]
async fn recover_from_force_end_epoch_2() {
    let epoch_duration_secs = 20;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // Ensure vtxn is enabled.
            conf.consensus_config.enable_validator_txns();

            // Ensure randomness flag is set.
            let mut features = Features::default();
            features.enable(FeatureFlag::RECONFIGURE_WITH_DKG);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    info!("Now in epoch 3. Force-end the epoch");
    let force_end_epoch_script = r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::version;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        version::set_version(&framework_signer, 999);
    }
}
"#;
    let txn_summary = cli
        .run_script(root_idx, force_end_epoch_script)
        .await
        .expect("Txn execution error.");
    debug!("force_end_epoch_summary={:?}", txn_summary);

    info!("There should be no randomness in epoch 4.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    let trigger_async_reconfig_script = r#"
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;
    let txn_summary = cli
        .run_script(root_idx, trigger_async_reconfig_script)
        .await
        .expect("Txn execution error.");
    debug!("trigger_async_reconfig_summary={:?}", txn_summary);

    info!("Randomness should come back in epoch 5.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 5.");
    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 5 should be present");
    assert_eq!(5, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
}
