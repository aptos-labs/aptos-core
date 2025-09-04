// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{decrypt_key_map, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
    utils::{get_current_consensus_config, get_on_chain_resource},
};
use velor_forge::{Node, Swarm, SwarmExt};
use velor_logger::{debug, info};
use velor_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Enable on-chain randomness by enabling validator transactions and randomness main logic.
#[tokio::test]
async fn enable_feature_2() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // start with vtxn disabled and randomness off.
            conf.consensus_config.disable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_disabled());
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = velor_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    info!("Now in epoch 3. Enabling all the dependencies at the same time.");
    let mut config = get_current_consensus_config(&client).await;
    config.enable_validator_txns();
    let config_bytes = bcs::to_bytes(&config).unwrap();
    let script = format!(
        r#"
script {{
    use velor_framework::velor_governance;
    use velor_framework::consensus_config;
    use velor_framework::randomness_config;
    use velor_std::fixed_point64;

    fun main(core_resources: &signer) {{
        let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @0x1);
        let consensus_config_bytes = vector{:?};
        consensus_config::set_for_next_epoch(&framework_signer, consensus_config_bytes);
        let randomness_config = randomness_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        randomness_config::set_for_next_epoch(&framework_signer, randomness_config);
        velor_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        config_bytes
    );

    debug!("script={}", script);
    let txn_summary = cli
        .run_script(root_idx, script.as_str())
        .await
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    info!("Now in epoch 4. Both DKG and vtxn are enabled. There should be no randomness since DKG did not happen at the end of last epoch.");
    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() != 4
    );

    info!("Waiting for epoch 5.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            5,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Waited too long for epoch 5.");

    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 5 should be present");
    assert_eq!(5, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
}
