// Copyright © Aptos Foundation

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    SigningKey, Uniform,
};
use aptos_forge::{AptosPublicInfo, LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_types::{
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        AllProvidersJWKs, PatchedJWKs, ProviderJWKs,
    },
    oidb::{
        test_utils::{
            get_sample_esk, get_sample_iss, get_sample_jwk, get_sample_oidb_groth16_sig_and_pk,
            get_sample_oidb_openid_sig_and_pk,
        },
        Configuration, Groth16VerificationKey, OidbPublicKey, OidbSignature, ZkpOrOpenIdSig,
    },
    transaction::{
        authenticator::{AnyPublicKey, EphemeralSignature},
        SignedTransaction,
    },
};
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use std::time::Duration;

// TODO(oidb): Test the override aud_val path

#[tokio::test]
async fn test_oidb_oidc_txn_verifies() {
    let (_, _, mut swarm, signed_txn) =
        get_oidb_transaction(get_sample_oidb_openid_sig_and_pk).await;

    info!("Submit OpenID transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with OpenID TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_oidb_oidc_txn_with_bad_jwt_sig() {
    let (tw_sk, mut swarm) = setup_local_net().await;
    let (mut oidb_sig, oidb_pk) = get_sample_oidb_openid_sig_and_pk();

    match &mut oidb_sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(_) => panic!("Internal inconsistency"),
        ZkpOrOpenIdSig::OpenIdSig(openid_sig) => {
            openid_sig.jwt_sig_b64 = "bad signature".to_string() // Mauling the signature
        },
    }

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_oidb_transaction(&mut info, oidb_sig, oidb_pk, &tw_sk).await;

    info!("Submit OpenID transaction with bad JWT signature");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!("OpenID TXN with bad JWT signature should have failed verification")
    }
}

#[tokio::test]
async fn test_oidb_oidc_txn_with_expired_epk() {
    let (tw_sk, mut swarm) = setup_local_net().await;
    let (mut oidb_sig, oidb_pk) = get_sample_oidb_openid_sig_and_pk();

    oidb_sig.exp_timestamp_secs = 1; // This should fail the verification since the expiration date is way in the past

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_oidb_transaction(&mut info, oidb_sig, oidb_pk, &tw_sk).await;

    info!("Submit OpenID transaction with expired EPK");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!("OpenID TXN with expired EPK should have failed verification")
    }
}

#[tokio::test]
async fn test_oidb_groth16_verifies() {
    let (_, _, mut swarm, signed_txn) =
        get_oidb_transaction(get_sample_oidb_groth16_sig_and_pk).await;

    info!("Submit OIDB Groth16 transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with OIDB Groth16 TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_oidb_groth16_with_mauled_proof() {
    let (tw_sk, mut swarm) = setup_local_net().await;
    let (mut oidb_sig, oidb_pk) = get_sample_oidb_groth16_sig_and_pk();

    match &mut oidb_sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(proof) => {
            proof.non_malleability_signature =
                EphemeralSignature::ed25519(tw_sk.sign(&proof.proof).unwrap()); // bad signature using the TW SK rather than the ESK
        },
        ZkpOrOpenIdSig::OpenIdSig(_) => panic!("Internal inconsistency"),
    }

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_oidb_transaction(&mut info, oidb_sig, oidb_pk, &tw_sk).await;

    info!("Submit OIDB Groth16 transaction");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!("OIDB Groth16 TXN with mauled proof should have failed verification")
    }
}

#[tokio::test]
async fn test_oidb_groth16_with_bad_tw_signature() {
    let (_tw_sk, mut swarm) = setup_local_net().await;
    let (oidb_sig, oidb_pk) = get_sample_oidb_groth16_sig_and_pk();

    let mut info = swarm.aptos_public_info();

    // using the sample ESK rather than the TW SK to get a bad training wheels signature
    let signed_txn = sign_oidb_transaction(&mut info, oidb_sig, oidb_pk, &get_sample_esk()).await;

    info!("Submit OIDB Groth16 transaction");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!(
            "OIDB Groth16 TXN with bad training wheels signature should have failed verification"
        )
    }
}

async fn sign_oidb_transaction<'a>(
    info: &mut AptosPublicInfo<'a>,
    mut oidb_sig: OidbSignature,
    oidb_pk: OidbPublicKey,
    tw_sk: &Ed25519PrivateKey,
) -> SignedTransaction {
    let oidb_addr = info
        .create_user_account_with_any_key(&AnyPublicKey::oidb(oidb_pk.clone()))
        .await
        .unwrap();
    info.mint(oidb_addr, 10_000_000_000).await.unwrap();

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100))
        .sender(oidb_addr)
        .sequence_number(1)
        .build();

    let esk = get_sample_esk();
    oidb_sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&raw_txn).unwrap());

    // Compute the training wheels signature if not present
    match &mut oidb_sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(proof) => {
            proof.training_wheels_signature = Some(EphemeralSignature::ed25519(
                tw_sk.sign(&proof.proof).unwrap(),
            ));
        },
        ZkpOrOpenIdSig::OpenIdSig(_) => {},
    }

    SignedTransaction::new_oidb(raw_txn, oidb_pk, oidb_sig)
}

async fn get_oidb_transaction(
    get_pk_and_sig_func: fn() -> (OidbSignature, OidbPublicKey),
) -> (OidbSignature, OidbPublicKey, LocalSwarm, SignedTransaction) {
    let (tw_sk, mut swarm) = setup_local_net().await;

    let (oidb_sig, oidb_pk) = get_pk_and_sig_func();

    let mut info = swarm.aptos_public_info();
    let signed_txn =
        sign_oidb_transaction(&mut info, oidb_sig.clone(), oidb_pk.clone(), &tw_sk).await;

    (oidb_sig, oidb_pk, swarm, signed_txn)
}

async fn setup_local_net() -> (Ed25519PrivateKey, LocalSwarm) {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let tw_sk = spawn_network_and_execute_gov_proposals(&mut swarm, &mut cli).await;
    (tw_sk, swarm)
}

async fn spawn_network_and_execute_gov_proposals(
    swarm: &mut LocalSwarm,
    cli: &mut CliTestFramework,
) -> Ed25519PrivateKey {
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
            "0x1::openid_account::Groth16VerificationKey",
        )
        .await;
    let vk = maybe_response.unwrap().into_inner();
    println!("Groth16 VK: {:?}", vk);

    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(
            AccountAddress::ONE,
            "0x1::openid_account::Configuration",
        )
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("OIDB configuration before: {:?}", config);

    let iss = get_sample_iss();
    let jwk = get_sample_jwk();

    let training_wheels_sk = Ed25519PrivateKey::generate(&mut thread_rng());
    let training_wheels_pk = Ed25519PublicKey::from(&training_wheels_sk);

    info!("Insert a JWK.");
    let jwk_patch_script = format!(
        r#"
script {{
use aptos_framework::jwks;
use aptos_framework::openid_account;
use aptos_framework::aptos_governance;
use std::string::utf8;
use std::option;
fun main(core_resources: &signer) {{
    let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
    let jwk_0 = jwks::new_rsa_jwk(
        utf8(b"{}"),
        utf8(b"{}"),
        utf8(b"{}"),
        utf8(b"{}")
    );
    let patches = vector[
        jwks::new_patch_remove_all(),
        jwks::new_patch_upsert_jwk(b"{}", jwk_0),
    ];
    jwks::set_patches(&framework_signer, patches);

    openid_account::update_max_exp_horizon(&framework_signer, {});
    openid_account::update_training_wheels(&framework_signer, option::some(x"{}"));
}}
}}
"#,
        jwk.kid,
        jwk.alg,
        jwk.e,
        jwk.n,
        iss,
        Configuration::new_for_testing().max_exp_horizon_secs,
        hex::encode(training_wheels_pk.to_bytes())
    );

    let txn_summary = cli.run_script(root_idx, &jwk_patch_script).await.unwrap();
    debug!("txn_summary={:?}", txn_summary);

    info!("Use resource API to check the patch result.");
    let patched_jwks = get_latest_jwkset(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);

    let expected_providers_jwks = AllProvidersJWKs {
        entries: vec![ProviderJWKs {
            issuer: iss.into_bytes(),
            version: 0,
            jwks: vec![JWKMoveStruct::from(JWK::RSA(jwk))],
        }],
    };
    assert_eq!(expected_providers_jwks, patched_jwks.jwks);

    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(
            AccountAddress::ONE,
            "0x1::openid_account::Configuration",
        )
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("OIDB configuration after: {:?}", config);

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
