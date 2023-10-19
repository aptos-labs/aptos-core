// Copyright © Aptos Foundation

use crate::{dkg, dkg::decrypt_key_map, smoke_test_environment::SwarmBuilder};
use aptos::test::CliTestFramework;
use aptos_forge::{Node, Swarm, SwarmExt};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use crate::dkg::{get_latest_dkg_state, verify_dkg_transcript};

#[tokio::test]
async fn dkg_feature_flag_flips() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
        }))
        .build_with_cli(0)
        .await;

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm
        .validators()
        .skip(1)
        .next()
        .unwrap()
        .rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm.wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 3));

    println!("DKG should be disabled since the beginning and in epoch 3.");
    assert(get_latest_dkg_state(&client).await.last_complete.is_none());

    println!("Enabling the feature.");
    let enable_dkg_script = format!(r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;
    use
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let dkg_feature_id: u64 = 38; // supposed to be `std::features::get_reconfigure_with_dkg_feature()` but not sure if it will work.
        aptos_governance::toggle_features(&framework_signer, vector[dkg_feature_id], vector[]);
    }}
}}
"#, );

    cli.run_script(0, &enable_dkg_script);

    println!("Wait until epoch 5. (Epoch 4 will have no DKG.)");
    swarm.wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(epoch_duration_secs * 2));

    let dkg_session_1 = get_latest_dkg_state(&client).await.last_complete.expect("After epoch 5, there should be DKG results on chain.");
    assert!(verify_dkg_transcript(&dkg_session_1, &decrypt_key_map));

    println!("Disabling the feature.");

    let disable_dkg_script = format!(r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;
    use
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        let dkg_feature_id: u64 = 38; // supposed to be `std::features::get_reconfigure_with_dkg_feature()` but not sure if it will work.
        aptos_governance::toggle_features(&framework_signer, vector[], vector[dkg_feature_id]);
    }}
}}
"#, );

    cli.run_script(0, &disable_dkg_script);

    println!("Wait until epoch 7. (Epoch 6 should still have DKG results.)");
    swarm.wait_for_all_nodes_to_catchup_to_epoch(7, Duration::from_secs(epoch_duration_secs * 2));

    println!("DKG should be disabled again at this point.");
    assert(get_latest_dkg_state(&client).await.last_complete.is_none());
}
