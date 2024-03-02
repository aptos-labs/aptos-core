// Copyright © Aptos Foundation

use crate::{randomness::get_on_chain_resource, smoke_test_environment::SwarmBuilder};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    dkg::DKGState,
    on_chain_config::{ConfigurationResource, FeatureFlag, Features},
};
use std::{sync::Arc, time::Duration};

/// The presence of `FailureInjectionBlockDKG` should block DKG.
#[tokio::test]
async fn failure_indicator_block_dkg() {
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

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    info!("Now in epoch 3. Set flag to block randomness.");
    let script = r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::dkg;
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        dkg::block_dkg(&framework_signer);
    }}
}}
"#;
    let txn_summary = cli
        .run_script(root_idx, script)
        .await
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    info!("Chain should be stuck at the end of epoch 3.");
    tokio::time::sleep(Duration::from_secs(60)).await;
    let config_resource = get_on_chain_resource::<ConfigurationResource>(&client).await;
    assert_eq!(3, config_resource.epoch());
    let dkg_state = get_on_chain_resource::<DKGState>(&client).await;
    assert_eq!(
        3,
        dkg_state
            .in_progress
            .as_ref()
            .unwrap()
            .metadata
            .dealer_epoch
    );
}
