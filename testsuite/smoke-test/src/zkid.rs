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
    bn254_circom::{G1Bytes, G2Bytes, Groth16VerificationKey},
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
        Configuration, Groth16Zkp, IdCommitment, OpenIdSig, Pepper, SignedGroth16Zkp,
        ZkIdPublicKey, ZkIdSignature, ZkpOrOpenIdSig,
    },
};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use std::time::Duration;

// TODO(zkid): test the override aud_val path
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
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxMzIwMTc1NTc0Njg5NjI2Mjk1MjE1NjI0NDQ5OTc3ODc4Njk5NzE5Njc3NzE0MzIzOTg5Njk3NzczODY0NTIzOTkwMzIyNzI4MjE2IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcyNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "W4-yUKHhM7HYYhELuP9vfRH1D2IgcSSxz397SMz4u04WfLW3mBrmsaZ0QBgUwy33I7ZA6UoffnuUN8M8koXjfFMv0AfTgkQNJCg0X7cPCIn0WplONF6i4ACWUZjX_fSg36y5cRLDBv4pMOOMEI_eGyMt2tOoNZ2Fik1k-AXsyVNV-mqBtzblhdiGpy0bBgvcrMvJiBfe-AJazv-W3Ik5M0OeZB12YbQDHQSMTjhPEnADn6gmgsERBKbaGO8ieKW0v2Ukb3yqIy7PtdM44wJ0E_u2_tyqffmm6VoH6zaiFHgvEqfT7IM1w8_8k7nk2M9rT__o2A0cGWsYzhw3Mxs1Xw".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
        idc_aud_val: None,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 1727812836,
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
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxMzIwMTc1NTc0Njg5NjI2Mjk1MjE1NjI0NDQ5OTc3ODc4Njk5NzE5Njc3NzE0MzIzOTg5Njk3NzczODY0NTIzOTkwMzIyNzI4MjE2IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcyNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "bad_signature".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
        idc_aud_val: None,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 1727812836,
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
    let jwt_payload = "eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhdWQiOiJ0ZXN0X2NsaWVudF9pZCIsInN1YiI6InRlc3RfYWNjb3VudCIsImVtYWlsIjoidGVzdEBnbWFpbC5jb20iLCJlbWFpbF92ZXJpZmllZCI6dHJ1ZSwibm9uY2UiOiIxMzIwMTc1NTc0Njg5NjI2Mjk1MjE1NjI0NDQ5OTc3ODc4Njk5NzE5Njc3NzE0MzIzOTg5Njk3NzczODY0NTIzOTkwMzIyNzI4MjE2IiwibmJmIjoxNzAyODA4OTM2LCJpYXQiOjE3MDQ5MDkyMzYsImV4cCI6MTcyNzgxMjgzNiwianRpIjoiZjEwYWZiZjBlN2JiOTcyZWI4ZmE2M2YwMjQ5YjBhMzRhMjMxZmM0MCJ9".to_string();
    let jwt_sig = "W4-yUKHhM7HYYhELuP9vfRH1D2IgcSSxz397SMz4u04WfLW3mBrmsaZ0QBgUwy33I7ZA6UoffnuUN8M8koXjfFMv0AfTgkQNJCg0X7cPCIn0WplONF6i4ACWUZjX_fSg36y5cRLDBv4pMOOMEI_eGyMt2tOoNZ2Fik1k-AXsyVNV-mqBtzblhdiGpy0bBgvcrMvJiBfe-AJazv-W3Ik5M0OeZB12YbQDHQSMTjhPEnADn6gmgsERBKbaGO8ieKW0v2Ukb3yqIy7PtdM44wJ0E_u2_tyqffmm6VoH6zaiFHgvEqfT7IM1w8_8k7nk2M9rT__o2A0cGWsYzhw3Mxs1Xw".to_string();

    let openid_signature = OpenIdSig {
        jwt_sig,
        jwt_payload,
        uid_key: "sub".to_string(),
        epk_blinder,
        pepper,
        idc_aud_val: None,
    };

    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_signature),
        jwt_header,
        exp_timestamp_secs: 1, // Expired timestamp
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
        "20534193224874816823038374805971256353897254359389549519579800571198905682623",
        "3128047629776327625062258700337193014005673411952335683536865294076478098678",
    )
    .unwrap();
    let b = G2Bytes::new_unchecked(
        [
            "11831059544281359959902363827760224027191828999098259913907764686593049260801",
            "14933419822301565783764657928814181728459886670248956535955133596731082875810",
        ],
        [
            "16616167200367085072660100259194052934821478809307596510515652443339946625933",
            "1103855954970567341442645156173756328940907403537523212700521414512165362008",
        ],
    )
    .unwrap();
    let c = G1Bytes::new_unchecked(
        "296457556259014920933232985275282694032456344171046224944953719399946325676",
        "10314488872240559867545387237625153841351761679810222583912967187658678987385",
    )
    .unwrap();
    let proof = Groth16Zkp::new(a, b, c);

    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();

    let proof_sig = ephemeral_account.private_key().sign(&proof).unwrap();
    let ephem_proof_sig = EphemeralSignature::ed25519(proof_sig);

    // TODO(zkid): Refactor tests to be modular and add test for bad training wheels signature (commented out below).
    //let bad_sk = Ed25519PrivateKey::generate(&mut thread_rng());
    let config = Configuration::new_for_devnet_and_testing();
    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::Groth16Zkp(SignedGroth16Zkp {
            proof: proof.clone(),
            non_malleability_signature: ephem_proof_sig,
            extra_field: "\"family_name\":\"Straka\",".to_string(),
            exp_horizon_secs: config.max_exp_horizon_secs,
            override_aud_val: None,
            training_wheels_signature: Some(EphemeralSignature::ed25519(
                tw_sk.sign(&proof).unwrap(),
            )),
        }),
        jwt_header,
        exp_timestamp_secs: 1900255944,
        ephemeral_pubkey: ephemeral_public_key,
        ephemeral_signature,
    };

    let signed_txn = SignedTransaction::new_zkid(raw_txn, sender_zkid_public_key, zk_sig);

    info!("Submit zero knowledge transaction");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with Groth16 TXN verification: {:?}", e)
    }
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
        "20534193224874816823038374805971256353897254359389549519579800571198905682623",
        "3128047629776327625062258700337193014005673411952335683536865294076478098678",
    )
    .unwrap();
    let b = G2Bytes::new_unchecked(
        [
            "11831059544281359959902363827760224027191828999098259913907764686593049260801",
            "14933419822301565783764657928814181728459886670248956535955133596731082875810",
        ],
        [
            "16616167200367085072660100259194052934821478809307596510515652443339946625933",
            "1103855954970567341442645156173756328940907403537523212700521414512165362008",
        ],
    )
    .unwrap();
    let c = G1Bytes::new_unchecked(
        "296457556259014920933232985275282694032456344171046224944953719399946325676",
        "10314488872240559867545387237625153841351761679810222583912967187658678987385",
    )
    .unwrap();
    let proof = Groth16Zkp::new(a, b, c);

    let jwt_header = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0".to_string();

    let config = Configuration::new_for_devnet_and_testing();
    let zk_sig = ZkIdSignature {
        sig: ZkpOrOpenIdSig::Groth16Zkp(SignedGroth16Zkp {
            proof: proof.clone(),
            non_malleability_signature: ephemeral_signature.clone(), // Wrong signature
            extra_field: "\"family_name\":\"Straka\",".to_string(),
            exp_horizon_secs: config.max_exp_horizon_secs,
            override_aud_val: None,
            training_wheels_signature: Some(EphemeralSignature::ed25519(
                tw_sk.sign(&proof).unwrap(),
            )),
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

    let maybe_response = client
        .get_account_resource_bcs::<Groth16VerificationKey>(
            AccountAddress::ONE,
            "0x1::zkid::Groth16VerificationKey",
        )
        .await;
    let vk = maybe_response.unwrap().into_inner();
    println!("Groth16 VK: {:?}", vk);

    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(AccountAddress::ONE, "0x1::zkid::Configuration")
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("zkID configuration: {:?}", config);

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
