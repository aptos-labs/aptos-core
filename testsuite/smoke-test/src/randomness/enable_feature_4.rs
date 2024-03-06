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
    randomness::PerBlockRandomness,
};
use std::{sync::Arc, time::Duration};

/// Enable on-chain randomness in the following steps.
/// - Enable validator transactions in consensus config by default.
/// - Enable feature `FAST_RANDOMNESS` in epoch `e + 1`. Randomness should not be enabled.
/// - Enable feature `RECONFIGURE_WITH_DKG` in epoch `e + 1`. Randomness should be enabled.
#[tokio::test]
async fn enable_feature_4() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // start with dkg disabled.
            let mut features = Features::default();
            features.disable(FeatureFlag::RECONFIGURE_WITH_DKG);
            features.disable(FeatureFlag::FAST_RANDOMNESS);
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

    info!("Now in epoch 3. Enabling feature FAST_RANDOMNESS.");
    let enable_fast_path_script = r#"
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let dkg_feature_id: u64 = std::features::get_fast_randomness_feature();
        aptos_governance::toggle_features(&framework_signer, vector[dkg_feature_id], vector[]);
    }
}
"#;

    let txn_summary = cli
        .run_script(root_idx, enable_fast_path_script)
        .await
        .expect("Txn execution error.");
    debug!("enabling_fast_randomness_summary={:?}", txn_summary);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    info!("Now in epoch 4. Enabling feature RECONFIGURE_WITH_DKG.");
    let enable_dkg_script = r#"
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let dkg_feature_id: u64 = std::features::get_reconfigure_with_dkg_feature();
        aptos_governance::toggle_features(&framework_signer, vector[dkg_feature_id], vector[]);
    }
}
"#;

    let txn_summary = cli
        .run_script(root_idx, enable_dkg_script)
        .await
        .expect("Txn execution error.");
    debug!("enabling_dkg_summary={:?}", txn_summary);

    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() != 4
    );

    let randomness_seed = get_on_chain_resource::<PerBlockRandomness>(&client).await;
    assert!(randomness_seed.seed.is_none());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 5.");

    info!("Now in epoch 5. Both DKG and vtxn are enabled. There should be no randomness since DKG did not happen at the end of last epoch.");
    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() != 5
    );

    info!("Waiting for epoch 6.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            6,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Waited too long for epoch 6.");

    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 6 should be present");
    assert_eq!(6, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
}
