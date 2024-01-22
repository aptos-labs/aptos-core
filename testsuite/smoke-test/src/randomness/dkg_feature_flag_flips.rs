// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, get_on_chain_resource, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{Node, Swarm, SwarmExt};
use std::{sync::Arc, time::Duration};
use aptos_types::dkg::DKGState;

/// A quick overview of what this test does.
///             Has randomness?     What else happened?
/// Epoch 1     Yes                 -
/// Epoch 2     Yes                 -
/// Epoch 3     Yes                 Executed a txn to disable DKG.
/// Epoch 4     Yes                 -
/// Epoch 5     No                  Executed a txn to enable DKG.
/// Epoch 6     No                  -
/// Epoch 7     Yes                 -
#[tokio::test]
async fn dkg_feature_flag_flips() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
        }))
        .build_with_cli(0)
        .await;

    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 5.");

    let dkg_session_3 = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("After epoch 3, there should be DKG results on chain.");
    assert_eq!(3, dkg_session_3.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session_3, &decrypt_key_map).is_ok());

    println!("Disabling the feature.");
    let disable_dkg_script = r#"
script {
    use aptos_framework::aptos_governance;
    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let dkg_feature_id: u64 = std::features::get_reconfigure_with_dkg_feature();
        aptos_governance::toggle_features(&framework_signer, vector[], vector[dkg_feature_id]);
    }
}
"#;

    let txn_summary = cli
        .run_script(root_idx, disable_dkg_script)
        .await
        .expect("Disabling script execution error.");
    println!("disabling_txn_summary={:?}", txn_summary);

    println!("Wait until epoch 5. (Epoch 4 will still have DKG.)");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            5,
            Duration::from_secs((epoch_duration_secs + estimated_dkg_latency_secs) * 2),
        )
        .await
        .expect("Waited too long for epoch 5.");
    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() < 5
    );

    println!("Re-enabling the feature.");

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
        .expect("Enabling script execution error.");
    println!("enabling_txn_summary={:?}", txn_summary);

    println!("Wait until epoch 7. (Epoch 6 should still have no DKG results.)");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            7,
            Duration::from_secs((epoch_duration_secs + estimated_dkg_latency_secs) * 2),
        )
        .await
        .expect("Waited too long for epoch 7");
    let dkg_session_7 = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("Starting epoch 7, there should be DKG results again.");
    assert_eq!(7, dkg_session_7.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session_7, &decrypt_key_map).is_ok());
}
