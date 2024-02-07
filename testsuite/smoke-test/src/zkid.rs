// Copyright © Aptos Foundation

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
    SigningKey, Uniform,
};
use aptos_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::{
    bn254_circom::{G1Bytes, G2Bytes},
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        rsa::RSA_JWK,
        AllProvidersJWKs, PatchedJWKs, ProviderJWKs,
    },
    transaction::{
        authenticator::{AnyPublicKey, EphemeralPublicKey, EphemeralSignature},
        SignedTransaction,
    },
    zkid::{
        Groth16Zkp, IdCommitment, OpenIdSig, Pepper, SignedGroth16Zkp, ZkIdPublicKey,
        ZkIdSignature, ZkpOrOpenIdSig,
    },
};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use std::time::Duration;

// TODO(zkid): These tests are not modular and they lack instructions for how to regenerate the proofs.

#[tokio::test]
async fn test_zkid_oidc_signature_transaction_submission() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let _ = test_setup(&mut swarm, &mut cli).await;

    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage(&pepper, "test_client_id", "sub", "test_account").unwrap();
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

    let epk_blinder = vec![0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxYlFsNF9YYzUtSXBDcFViS19BZVhwZ2Q2R1o0MGxVVjN1YjN5b19FTHhrIiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "oBdOiIUc-ioG2-sHV1hWDLjgk4NrVf3z6V-HmgbOrVAz3PV1CwdfyTXsmVaCqLzOHzcbFB6ZRDxShs3aR7PsqdlhI0Dh8WrfU8kBkyk1FAmx2nST4SoSJROXsnusaOpNFpgSl96Rq3SXgr-yPBE9dEwTfD00vq2gH_fH1JAIeJJhc6WicMcsEZ7iONT1RZOid_9FlDrg1GxlGtNmpn4nEAmIxqnT0JrCESiRvzmuuXUibwx9xvHgIxhyVuAA9amlzaD1DL6jEc5B_0YnGKN7DO_l2Hkj9MbQZvU0beR-Lfcz8jxCjojODTYmWgbtu5E7YWIyC6dsjiBnTxc-svCsmQ".to_string();

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
        exp_timestamp_secs: 1707812836,
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
async fn test_zkid_oidc_signature_transaction_submission_fails_jwt_verification() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let _ = test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage(&pepper, "test_client_id", "sub", "test_account").unwrap();
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

    let epk_blinder = vec![0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxYlFsNF9YYzUtSXBDcFViS19BZVhwZ2Q2R1o0MGxVVjN1YjN5b19FTHhrIiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
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
        exp_timestamp_secs: 1707812836,
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
async fn test_zkid_oidc_signature_transaction_submission_epk_expired() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let _ = test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::new([0u8; 31]);
    let idc =
        IdCommitment::new_from_preimage(&pepper, "test_client_id", "sub", "test_account").unwrap();
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

    let epk_blinder = vec![0u8; 31];
    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxYlFsNF9YYzUtSXBDcFViS19BZVhwZ2Q2R1o0MGxVVjN1YjN5b19FTHhrIiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcwNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "oBdOiIUc-ioG2-sHV1hWDLjgk4NrVf3z6V-HmgbOrVAz3PV1CwdfyTXsmVaCqLzOHzcbFB6ZRDxShs3aR7PsqdlhI0Dh8WrfU8kBkyk1FAmx2nST4SoSJROXsnusaOpNFpgSl96Rq3SXgr-yPBE9dEwTfD00vq2gH_fH1JAIeJJhc6WicMcsEZ7iONT1RZOid_9FlDrg1GxlGtNmpn4nEAmIxqnT0JrCESiRvzmuuXUibwx9xvHgIxhyVuAA9amlzaD1DL6jEc5B_0YnGKN7DO_l2Hkj9MbQZvU0beR-Lfcz8jxCjojODTYmWgbtu5E7YWIyC6dsjiBnTxc-svCsmQ".to_string();

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

#[ignore]
#[tokio::test]
// TODO(zkid): heliuchuan, please regenerate Groth16 proof
async fn test_zkid_groth16_verifies() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let tw_sk = test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::from_number(76);
    let idc = IdCommitment::new_from_preimage(
        &pepper,
        "407408718192.apps.googleusercontent.com",
        "sub",
        "113990307082899718775",
    )
    .unwrap();
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
            "0x76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
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

    let a = G1Bytes::new_unchecked(
        "19843734071102143602441202443608981862760142725808945198375332557568733182487",
        "7490772921219489322991985736547330118240504032652964776703563444800470517507",
    )
    .unwrap();
    let b = G2Bytes::new_unchecked(
        [
            "799096037534263564394323941982781608031806843599379318443427814019873224162",
            "14026173330568980628011709588549732085308934280497623796136346291913189596064",
        ],
        [
            "18512483370445888670421748202641195280704367913960380279153644128302403162953",
            "11254131899335650800706930224907562847943361881351835752623166468667575239687",
        ],
    )
    .unwrap();
    let c = G1Bytes::new_unchecked(
        "161411929919357135819312594620804205291494587085213166645876168613542945746",
        "20470377953299181976881540108292343474195200393467944112548990712451344598537",
    )
    .unwrap();
    let proof = Groth16Zkp::new(a, b, c);

    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();

    let proof_sig = ephemeral_account.private_key().sign(&proof).unwrap();
    let ephem_proof_sig = EphemeralSignature::ed25519(proof_sig);

    // TODO(zkid): Refactor tests to be modular and add test for bad training wheels signature (commented out below).
    //let bad_sk = Ed25519PrivateKey::generate(&mut thread_rng());
    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::Groth16Zkp(SignedGroth16Zkp {
            proof: proof.clone(),
            non_malleability_signature: ephem_proof_sig,
            training_wheels_signature: EphemeralSignature::ed25519(tw_sk.sign(&proof).unwrap()),
            //training_wheels_signature: EphemeralSignature::ed25519(bad_sk.sign(&proof).unwrap()),
            extra_field: "\"family_name\":\"Straka\",".to_string(),
            override_aud_val: None,
        }),
        jwt_header,
        exp_timestamp_secs: 1900255944,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit zero knowledge transaction");
    info.client()
        .submit_without_serializing_response(&signed_txn)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_zkid_groth16_signature_transaction_submission_proof_signature_check_fails() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let tw_sk = test_setup(&mut swarm, &mut cli).await;
    let mut info = swarm.aptos_public_info();

    let pepper = Pepper::from_number(76);
    let idc = IdCommitment::new_from_preimage(
        &pepper,
        "407408718192.apps.googleusercontent.com",
        "sub",
        "113990307082899718775",
    )
    .unwrap();
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
            "0x76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
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

    let a = G1Bytes::new_unchecked(
        "19843734071102143602441202443608981862760142725808945198375332557568733182487",
        "7490772921219489322991985736547330118240504032652964776703563444800470517507",
    )
    .unwrap();
    let b = G2Bytes::new_unchecked(
        [
            "799096037534263564394323941982781608031806843599379318443427814019873224162",
            "14026173330568980628011709588549732085308934280497623796136346291913189596064",
        ],
        [
            "18512483370445888670421748202641195280704367913960380279153644128302403162953",
            "11254131899335650800706930224907562847943361881351835752623166468667575239687",
        ],
    )
    .unwrap();
    let c = G1Bytes::new_unchecked(
        "161411929919357135819312594620804205291494587085213166645876168613542945746",
        "20470377953299181976881540108292343474195200393467944112548990712451344598537",
    )
    .unwrap();
    let proof = Groth16Zkp::new(a, b, c);

    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::Groth16Zkp(SignedGroth16Zkp {
            proof: proof.clone(),
            non_malleability_signature: ephemeral_signature.clone(), // Wrong signature
            training_wheels_signature: EphemeralSignature::ed25519(tw_sk.sign(&proof).unwrap()),
            extra_field: "\"family_name\":\"Straka\",".to_string(),
            override_aud_val: None,
        }),
        jwt_header,
        exp_timestamp_secs: 1900255944,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit zero knowledge transaction");
    info.client()
        .submit_without_serializing_response(&signed_txn)
        .await
        .unwrap_err();
}

async fn test_setup(swarm: &mut LocalSwarm, cli: &mut CliTestFramework) -> Ed25519PrivateKey {
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

    let training_wheels_sk = Ed25519PrivateKey::generate(&mut thread_rng());
    let training_wheels_pk = Ed25519PublicKey::from(&training_wheels_sk);

    info!("Insert a JWK.");
    let jwk_patch_script = format!(
        r#"
script {{
use aptos_framework::jwks;
use aptos_framework::zkid;
use aptos_framework::aptos_governance;
use std::string::utf8;
use std::option;
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

    zkid::update_training_wheels(&framework_signer, option::some(x"{}"));
}}
}}
"#,
        jwk.kid,
        jwk.n,
        iss,
        hex::encode(training_wheels_pk.to_bytes())
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

    training_wheels_sk
}

async fn get_latest_jwkset(rest_client: &Client) -> PatchedJWKs {
    let maybe_response = rest_client
        .get_account_resource_bcs::<PatchedJWKs>(AccountAddress::ONE, "0x1::jwks::PatchedJWKs")
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}
