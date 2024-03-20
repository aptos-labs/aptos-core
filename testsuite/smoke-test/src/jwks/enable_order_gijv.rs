// Copyright © Aptos Foundation

use crate::{
    jwks::{
        dummy_provider::{
            request_handler::{EquivocatingServer, StaticContentServer},
            DummyProvider,
        },
        get_patched_jwks, put_provider_on_chain,
    },
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::jwks::{
    jwk::JWK, rsa::RSA_JWK, unsupported::UnsupportedJWK, AllProvidersJWKs, OIDCProvider,
    ProviderJWKs,
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use crate::jwks::{add_provider_google, enable_feature_flag, enable_vtxn, initialize_jwk_module};

#[tokio::test]
async fn enable_order_gijv() {
    let epoch_duration_secs = 20;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
        }))
        .build_with_cli(0)
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 3 taking too long to arrive!");

    let txn_result = add_provider_google(&cli, root_idx).await;
    println!("provider_result={:?}", txn_result);

    let txn_result = initialize_jwk_module(&cli, root_idx).await;
    println!("init_result={:?}", txn_result);

    let txn_result = enable_feature_flag(&cli, root_idx).await;
    println!("flag_result={:?}", txn_result);

    let txn_result = enable_vtxn(&client, &cli, root_idx).await;
    println!("vtxn_result={:?}", txn_result);

    tokio::time::sleep(Duration::from_secs(20)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    assert_eq!(2, patched_jwks.jwks.entries[0].jwks.len());
}
