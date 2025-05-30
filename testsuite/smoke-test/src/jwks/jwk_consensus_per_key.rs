// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::{
        dummy_provider::{
            request_handler::{EquivocatingServer, StaticContentServer},
            DummyHttpServer,
        },
        get_patched_jwks, update_jwk_consensus_config,
    },
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    jwks::{jwk::JWK, rsa::RSA_JWK, secure_test_rsa_jwk, AllProvidersJWKs, ProviderJWKs},
    keyless::test_utils::get_sample_iss,
    on_chain_config::{
        FeatureFlag, Features, JWKConsensusConfigV1, OIDCProvider, OnChainJWKConsensusConfig,
    },
};
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;

/// Validators should be able to reach consensus on key-level diffs
/// even if providers are equivocating on the full key list.
#[tokio::test]
async fn jwk_consensus_per_key() {
    let epoch_duration_secs = 30;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            let mut features = Features::default();
            features.enable(FeatureFlag::JWK_CONSENSUS_PER_KEY_MODE);
            conf.initial_features_override = Some(features);
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
    assert_eq!(1, patched_jwks.jwks.entries.len());

    info!("Adding providers https://alice.io and https://bob.dev");
    let (alice_config_server, alice_jwks_server, bob_config_server, bob_jwks_server) = tokio::join!(
        DummyHttpServer::spawn(),
        DummyHttpServer::spawn(),
        DummyHttpServer::spawn(),
        DummyHttpServer::spawn()
    );
    let alice_issuer_id = "https://alice.io";
    let bob_issuer_id = "https://bob.dev";
    alice_config_server.update_request_handler(Some(Arc::new(StaticContentServer::new_str(
        format!(
            r#"{{"issuer": "{}", "jwks_uri": "{}"}}"#,
            alice_issuer_id,
            alice_jwks_server.url()
        )
        .as_str(),
    ))));
    bob_config_server.update_request_handler(Some(Arc::new(StaticContentServer::new_str(
        format!(
            r#"{{"issuer": "{}", "jwks_uri": "{}"}}"#,
            bob_issuer_id,
            bob_jwks_server.url()
        )
        .as_str(),
    ))));

    // https://alice.io initially gives 0 keys.
    alice_jwks_server.update_request_handler(Some(Arc::new(StaticContentServer::new(
        r#"{"keys": []}"#.as_bytes().to_vec(),
    ))));

    // https://bob.dev initially gives 2 keys.
    let bob_jwk_0 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"b0","e":"AQAB","n":"990"}"#;
    let bob_jwk_1 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"b1","e":"AQAB","n":"991"}"#;
    bob_jwks_server.update_request_handler(Some(Arc::new(StaticContentServer::new(
        format!(r#"{{"keys": [{}, {}]}}"#, bob_jwk_0, bob_jwk_1)
            .as_str()
            .as_bytes()
            .to_vec(),
    ))));

    let config = OnChainJWKConsensusConfig::V1(JWKConsensusConfigV1 {
        oidc_providers: vec![
            OIDCProvider {
                name: alice_issuer_id.to_string(),
                config_url: alice_config_server.url(),
            },
            OIDCProvider {
                name: bob_issuer_id.to_string(),
                config_url: bob_config_server.url(),
            },
        ],
    });

    let txn_summary = update_jwk_consensus_config(cli, root_idx, &config).await;
    debug!("txn_summary={:?}", txn_summary);

    info!("Wait for 30 secs, and `b0, b1` should be on chain.");
    sleep(Duration::from_secs(30)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    assert_eq!(
        AllProvidersJWKs {
            entries: vec![
                ProviderJWKs {
                    issuer: bob_issuer_id.as_bytes().to_vec(),
                    version: 2,
                    jwks: vec![
                        JWK::RSA(RSA_JWK::new_256_aqab("b0", "990")).into(),
                        JWK::RSA(RSA_JWK::new_256_aqab("b1", "991")).into(),
                    ],
                },
                ProviderJWKs {
                    issuer: get_sample_iss().into_bytes(),
                    version: 0,
                    jwks: vec![secure_test_rsa_jwk().into()],
                },
            ]
        }
        .indexed()
        .unwrap(),
        patched_jwks.jwks.indexed().unwrap()
    );

    // https://alice.io exposes 4 new keys, but equivocates.
    let alice_jwk_0 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"a0","e":"AQAB","n":"999"}"#;
    let alice_jwk_1 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"a1","e":"AQAB","n":"998"}"#;
    let alice_jwk_2 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"a2","e":"AQAB","n":"997"}"#;
    let alice_jwk_3 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"a3","e":"AQAB","n":"996"}"#;
    alice_jwks_server.update_request_handler(Some(Arc::new(EquivocatingServer::new(
        format!(
            r#"{{"keys": [{},{},{}]}}"#,
            alice_jwk_0, alice_jwk_1, alice_jwk_2
        )
        .as_str()
        .as_bytes()
        .to_vec(), // Content A
        format!(
            r#"{{"keys": [{},{},{}]}}"#,
            alice_jwk_1, alice_jwk_2, alice_jwk_3
        )
        .as_str()
        .as_bytes()
        .to_vec(), // Content B
        2, // The first 2 clients get Content A, others get Content B.
    ))));

    // https://bob.dev deletes `b0`, updates `b1`, add `b2`.
    let bob_jwk_1_edited =
        r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"b1","e":"AQAB","n":"991ex"}"#;
    let bob_jwk_2 = r#"{"alg":"RS256","use":"sig","kty":"RSA","kid":"b2","e":"AQAB","n":"992"}"#;
    bob_jwks_server.update_request_handler(Some(Arc::new(StaticContentServer::new(
        format!(r#"{{"keys": [{},{}]}}"#, bob_jwk_1_edited, bob_jwk_2)
            .as_str()
            .as_bytes()
            .to_vec(),
    ))));

    info!("Wait for 30 secs and `a1, a2, b1 (new ver.), b2` should be on chain.");
    sleep(Duration::from_secs(30)).await;
    let patched_jwks = get_patched_jwks(&client).await;
    assert_eq!(
        AllProvidersJWKs {
            entries: vec![
                ProviderJWKs {
                    issuer: alice_issuer_id.as_bytes().to_vec(),
                    version: 2, // In per-key mode, we can only consensus one key at a time, and need 2 txns here.
                    jwks: vec![
                        JWK::RSA(RSA_JWK::new_256_aqab("a1", "998")).into(),
                        JWK::RSA(RSA_JWK::new_256_aqab("a2", "997")).into(),
                    ],
                },
                ProviderJWKs {
                    issuer: bob_issuer_id.as_bytes().to_vec(),
                    version: 5, // 3 changes since version 2: 1 delete and 2 upserts.
                    jwks: vec![
                        JWK::RSA(RSA_JWK::new_256_aqab("b1", "991ex")).into(),
                        JWK::RSA(RSA_JWK::new_256_aqab("b2", "992")).into(),
                    ],
                },
                ProviderJWKs {
                    issuer: get_sample_iss().into_bytes(),
                    version: 0,
                    jwks: vec![secure_test_rsa_jwk().into()],
                },
            ]
        }
        .indexed()
        .unwrap(),
        patched_jwks.jwks.indexed().unwrap()
    );

    info!("Tear down.");
    tokio::join!(
        alice_jwks_server.shutdown(),
        alice_config_server.shutdown(),
        bob_jwks_server.shutdown(),
        bob_config_server.shutdown()
    );
}
