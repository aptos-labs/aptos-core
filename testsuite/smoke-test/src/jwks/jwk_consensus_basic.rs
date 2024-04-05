// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::{
        dummy_provider::{
            request_handler::{EquivocatingServer, StaticContentServer},
            DummyProvider,
        },
        get_patched_jwks, update_jwk_consensus_config,
    },
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    jwks::{jwk::{JWKMoveStruct, JWK}, rsa::RSA_JWK, unsupported::UnsupportedJWK, AllProvidersJWKs, ProviderJWKs}, keyless::test_utils::get_sample_iss, on_chain_config::{JWKConsensusConfigV1, OIDCProvider, OnChainJWKConsensusConfig}
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// The validators should agree on the JWK after provider set is changed/JWK is rotated.
#[tokio::test]
async fn jwk_consensus_basic() {
    let epoch_duration_secs = 30;

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
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Initially the provider set is empty. The JWK map should have the secure test jwk added via a patch at genesis.");

    sleep(Duration::from_secs(10)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);
    assert!(patched_jwks.jwks.entries.len() == 1);

    info!("Adding some providers.");
    let (provider_alice, provider_bob) =
        tokio::join!(DummyProvider::spawn(), DummyProvider::spawn());

    provider_alice.update_request_handler(Some(Arc::new(StaticContentServer::new_str(
        r#"
{
    "keys": [
        {"kid":"kid1", "kty":"RSA", "e":"AQAB", "n":"n1", "alg":"RS384", "use":"sig"},
        {"n":"n0", "kty":"RSA", "use":"sig", "alg":"RS256", "e":"AQAB", "kid":"kid0"}
    ]
}
"#,
    ))));
    provider_bob.update_request_handler(Some(Arc::new(StaticContentServer::new(
        r#"{"keys": ["BOB_JWK_V0"]}"#.as_bytes().to_vec(),
    ))));
    let config = OnChainJWKConsensusConfig::V1(JWKConsensusConfigV1 {
        oidc_providers: vec![
            OIDCProvider {
                name: "https://alice.io".to_string(),
                config_url: provider_alice.open_id_config_url(),
            },
            OIDCProvider {
                name: "https://bob.dev".to_string(),
                config_url: provider_bob.open_id_config_url(),
            },
        ],
    });

    let txn_summary = update_jwk_consensus_config(cli, root_idx, &config).await;
    debug!("txn_summary={:?}", txn_summary);

    info!("Waiting for an on-chain update. 10 sec should be enough.");
    sleep(Duration::from_secs(10)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);
    assert_eq!(
        AllProvidersJWKs {
            entries: vec![
                ProviderJWKs {
                    issuer: b"https://alice.io".to_vec(),
                    version: 1,
                    jwks: vec![
                        JWK::RSA(RSA_JWK::new_256_aqab("kid0", "n0")).into(),
                        JWK::RSA(RSA_JWK::new_from_strs("kid1", "RSA", "RS384", "AQAB", "n1"))
                            .into(),
                    ],
                },
                ProviderJWKs {
                    issuer: b"https://bob.dev".to_vec(),
                    version: 1,
                    jwks: vec![JWK::Unsupported(UnsupportedJWK::new_with_payload(
                        "\"BOB_JWK_V0\""
                    ))
                    .into()],
                },
                ProviderJWKs {
                    issuer: get_sample_iss().into_bytes(),
                    version: 0,
                    jwks: vec![JWKMoveStruct::from(RSA_JWK::secure_test_jwk())],
                },
            ]
        },
        patched_jwks.jwks
    );

    info!("Rotating Alice keys. Also making https://alice.io gently equivocate.");
    provider_alice.update_request_handler(Some(Arc::new(EquivocatingServer::new(
        r#"{"keys": ["ALICE_JWK_V1A"]}"#.as_bytes().to_vec(),
        r#"{"keys": ["ALICE_JWK_V1B"]}"#.as_bytes().to_vec(),
        1,
    ))));

    info!("Waiting for an on-chain update. 30 sec should be enough.");
    sleep(Duration::from_secs(30)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);
    assert_eq!(
        AllProvidersJWKs {
            entries: vec![
                ProviderJWKs {
                    issuer: b"https://alice.io".to_vec(),
                    version: 2,
                    jwks: vec![JWK::Unsupported(UnsupportedJWK::new_with_payload(
                        "\"ALICE_JWK_V1B\""
                    ))
                    .into()],
                },
                ProviderJWKs {
                    issuer: b"https://bob.dev".to_vec(),
                    version: 1,
                    jwks: vec![JWK::Unsupported(UnsupportedJWK::new_with_payload(
                        "\"BOB_JWK_V0\""
                    ))
                    .into()],
                },
                ProviderJWKs {
                    issuer: get_sample_iss().into_bytes(),
                    version: 0,
                    jwks: vec![JWKMoveStruct::from(RSA_JWK::secure_test_jwk())],
                },
            ]
        },
        patched_jwks.jwks
    );

    info!("Tear down.");
    provider_alice.shutdown().await;
}
