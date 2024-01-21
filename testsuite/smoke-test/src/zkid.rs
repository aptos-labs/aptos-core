// Copyright © Aptos Foundation

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, encoding_type::EncodingType, SigningKey};
use aptos_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::{
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        rsa::RSA_JWK,
        AllProvidersJWKs, PatchedJWKs, ProviderJWKs,
    },
    transaction::{
        authenticator::{AnyPublicKey, EphemeralPublicKey, EphemeralSignature},
        SignedTransaction,
    },
    zkid::{IdCommitment, OpenIdSig, Pepper, ZkIdPublicKey, ZkIdSignature, ZkpOrOpenIdSig},
};
use move_core_types::account_address::AccountAddress;
use std::time::Duration;

async fn get_latest_jwkset(rest_client: &Client) -> PatchedJWKs {
    let maybe_response = rest_client
        .get_account_resource_bcs::<PatchedJWKs>(AccountAddress::ONE, "0x1::jwks::PatchedJWKs")
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}

async fn test_setup(swarm: &mut LocalSwarm, cli: &mut CliTestFramework) {
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    let iss = "https://accounts.google.com";
    let jwk = RSA_JWK {
        kid:"test_jwk".to_owned(),
        kty:"RSA".to_owned(),
        alg:"RS256".to_owned(),
        e:"AQAB".to_owned(),
        n:"6S7asUuzq5Q_3U9rbs-PkDVIdjgmtgWreG5qWPsC9xXZKiMV1AiV9LXyqQsAYpCqEDM3XbfmZqGb48yLhb_XqZaKgSYaC_h2DjM7lgrIQAp9902Rr8fUmLN2ivr5tnLxUUOnMOc2SQtr9dgzTONYW5Zu3PwyvAWk5D6ueIUhLtYzpcB-etoNdL3Ir2746KIy_VUsDwAM7dhrqSK8U2xFCGlau4ikOTtvzDownAMHMrfE7q1B6WZQDAQlBmxRQsyKln5DIsKv6xauNsHRgBAKctUxZG8M4QJIx3S6Aughd3RZC4Ca5Ae9fd8L8mlNYBCrQhOZ7dS0f4at4arlLcajtw".to_owned(),
    };

    info!("Insert a JWK.");
    let jwk_patch_script = format!(
        r#"
script {{
use aptos_framework::jwks;
use aptos_framework::aptos_governance;
use std::string::utf8;
fun main(core_resources: &signer) {{
    let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
    let google_jwk_0 = jwks::new_rsa_jwk(
        utf8(b"{}"),
        utf8(b"RS256"),
        utf8(b"AQAB"),
        utf8(b"{}")
    );
    let patches = vector[
        jwks::new_patch_remove_all(),
        jwks::new_patch_upsert_jwk(b"{}", google_jwk_0),
    ];
    jwks::set_patches(&framework_signer, patches);
}}
}}
"#,
        jwk.kid, jwk.n, iss
    );

    let txn_summary = cli.run_script(root_idx, &jwk_patch_script).await.unwrap();
    debug!("txn_summary={:?}", txn_summary);

    info!("Use resource API to check the patch result.");
    let patched_jwks = get_latest_jwkset(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);

    let expected_providers_jwks = AllProvidersJWKs {
        entries: vec![ProviderJWKs {
            issuer: b"https://accounts.google.com".to_vec(),
            version: 0,
            jwks: vec![JWKMoveStruct::from(JWK::RSA(jwk))],
        }],
    };
    assert_eq!(expected_providers_jwks, patched_jwks.jwks);

    let mut info = swarm.aptos_public_info();

    // Increment sequence number since we patched a JWK
    info.root_account().increment_sequence_number();
}

#[tokio::test]
async fn test_openid_signature_transaction_submission() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    test_setup(&mut swarm, &mut cli).await;

    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage("test_client_id", "sub", "test_account", &pepper).unwrap();
    let sender_zkid_public_key = ZkIdPublicKey {
        iss: "https://accounts.google.com".to_owned(),
        idc,
    };
    let sender_any_public_key = AnyPublicKey::zkid(sender_zkid_public_key.clone());
    let account_address = info
        .create_user_account_with_any_key(&sender_any_public_key)
        .await
        .unwrap();
    info.mint(account_address, 10_000_000_000).await.unwrap();

    let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
        .decode_key(
            "zkid test ephemeral private key",
            "0x1111111111111111111111111111111111111111111111111111111111111111"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
    let ephemeral_account: aptos_sdk::types::LocalAccount = LocalAccount::new(
        account_address,
        AccountKey::from_private_key(ephemeral_private_key),
        0,
    );
    let ephemeral_public_key = EphemeralPublicKey::ed25519(ephemeral_account.public_key().clone());

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100))
        .sender(account_address)
        .sequence_number(1)
        .build();

    let sender_sig = ephemeral_account.private_key().sign(&raw_txn).unwrap();
    let ephemeral_signature = EphemeralSignature::ed25519(sender_sig);

    let epk_blinder: [u8; 31] = [0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiJFVVRhSE9HdDcwRTNxbk9QMUJibnUzbE03QjR5TTdzaHZTb1NvdXF1VVJ3IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "CEgO4S7hRgASaINsGST5Ygtl_CY-mUn2GaQ6d7q9q1eGz1MjW0o0yusJQDU6Hi1nDfXlNSvCF2SgD9ayG3uDGC5-18H0AWo2QgyZ2rC_OUa36RCTmhdo-i_H8xmwPxa3yHZZsGC-gJy_vVX-rfMLIh-JgdIFFIzGVPN75MwXLP3bYUaB9Lw52g50rf_006Qg5ubkZ70I13vGUTVbRVWanQIN69naFqHreLCjVsGsEBVBoUtexZw6Ulr8s0VajBpcTUqlMvbvqMfQ33NXaBQYvu3YZivpkus8rcG_eAMrFbYFY9AZF7AaW2HUaYo5QjzMQDsIA1lpnAcOW3GzWvb0vw".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 2000000000,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit openid transaction");
    info.client()
        .submit_without_serializing_response(&signed_txn)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_openid_signature_transaction_submission_fails_jwt_verification() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage("test_client_id", "sub", "test_account", &pepper).unwrap();
    let sender_zkid_public_key = ZkIdPublicKey {
        iss: "https://accounts.google.com".to_owned(),
        idc,
    };
    let sender_any_public_key = AnyPublicKey::zkid(sender_zkid_public_key.clone());
    let account_address = info
        .create_user_account_with_any_key(&sender_any_public_key)
        .await
        .unwrap();
    info.mint(account_address, 10_000_000_000).await.unwrap();

    let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
        .decode_key(
            "zkid test ephemeral private key",
            "0x1111111111111111111111111111111111111111111111111111111111111111"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
    let ephemeral_account: aptos_sdk::types::LocalAccount = LocalAccount::new(
        account_address,
        AccountKey::from_private_key(ephemeral_private_key),
        0,
    );
    let ephemeral_public_key = EphemeralPublicKey::ed25519(ephemeral_account.public_key().clone());

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100))
        .sender(account_address)
        .sequence_number(1)
        .build();

    let sender_sig = ephemeral_account.private_key().sign(&raw_txn).unwrap();
    let ephemeral_signature = EphemeralSignature::ed25519(sender_sig);

    let epk_blinder: [u8; 31] = [0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiJFVVRhSE9HdDcwRTNxbk9QMUJibnUzbE03QjR5TTdzaHZTb1NvdXF1VVJ3IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "bad_signature".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 2000000000,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit openid transaction");
    let _err = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_openid_signature_transaction_submission_epk_expired() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage("test_client_id", "sub", "test_account", &pepper).unwrap();
    let sender_zkid_public_key = ZkIdPublicKey {
        iss: "https://accounts.google.com".to_owned(),
        idc,
    };
    let sender_any_public_key = AnyPublicKey::zkid(sender_zkid_public_key.clone());
    let account_address = info
        .create_user_account_with_any_key(&sender_any_public_key)
        .await
        .unwrap();
    info.mint(account_address, 10_000_000_000).await.unwrap();

    let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
        .decode_key(
            "zkid test ephemeral private key",
            "0x1111111111111111111111111111111111111111111111111111111111111111"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
    let ephemeral_account: aptos_sdk::types::LocalAccount = LocalAccount::new(
        account_address,
        AccountKey::from_private_key(ephemeral_private_key),
        0,
    );
    let ephemeral_public_key = EphemeralPublicKey::ed25519(ephemeral_account.public_key().clone());

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100))
        .sender(account_address)
        .sequence_number(1)
        .build();

    let sender_sig = ephemeral_account.private_key().sign(&raw_txn).unwrap();
    let ephemeral_signature = EphemeralSignature::ed25519(sender_sig);

    let epk_blinder: [u8; 31] = [0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiJIVEtvTDVGTDFOb0N1Vm1faHF1UWk2ZzAxckxPNjVhT2hQck5BVWxETVNNIiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "yX7vGd87u3O78GyBU7IuKnimM69yusEURgN4bXsXhJsujWTGQfvwVrXemO_gmWkykw2Awx-Vr8sNFD7vbNdbkLIdRAxoYow0hMNNvpcvAKriOiRX3ObGEJjpJNbiexQt6hJLh5sSfOW0wCmD_82KsOrNqDvegj1y-d_uemgrX9-I52tLemO76bplJQdFx5X-q2pC8y5HV4VsSgsigxpPfZ7lIwSB5db6vubTgPIYvzXnAajZkpAR-uMRFo1RoOtukeQjGBVxt104DIBh0sLW_9EH2f9j_7L6YWBtilpLSWBea2qDJ1dGPG_BvpBqVm5hcVy8qHRnX6fJXKMXnXvTKQ".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 1704909236,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit openid transaction");
    let _err = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await
        .unwrap_err();
}
