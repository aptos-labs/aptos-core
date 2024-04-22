// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::Ed25519PrivateKey, poseidon_bn254::fr_to_bytes_le, PrivateKey, SigningKey,
};
use aptos_forge::{AptosPublicInfo, LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_sdk::types::{EphemeralKeyPair, KeylessAccount, LocalAccount};
use aptos_types::{
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        rsa::RSA_JWK,
        AllProvidersJWKs, PatchedJWKs, ProviderJWKs,
    },
    keyless::{
        get_public_inputs_hash,
        test_utils::{
            self, get_sample_epk_blinder, get_sample_esk, get_sample_exp_date,
            get_sample_groth16_sig_and_pk, get_sample_groth16_sig_and_pk_no_extra_field,
            get_sample_iss, get_sample_jwk, get_sample_jwt_token, get_sample_openid_sig_and_pk,
            get_sample_pepper, get_sample_tw_sk, get_sample_zk_sig,
        },
        Configuration, EphemeralCertificate, Groth16ProofAndStatement, Groth16VerificationKey,
        KeylessPublicKey, KeylessSignature, TransactionAndProof, KEYLESS_ACCOUNT_MODULE_NAME,
    },
    transaction::{
        authenticator::{
            AccountAuthenticator, AnyPublicKey, AnySignature, EphemeralSignature,
            TransactionAuthenticator,
        },
        SignedTransaction,
    },
};
use move_core_types::account_address::AccountAddress;
use std::time::Duration;

#[tokio::test]
async fn test_keyless_oidc_txn_verifies() {
    let (_, _, mut swarm, signed_txn) = get_transaction(get_sample_openid_sig_and_pk).await;

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
async fn test_keyless_secure_test_jwk_initialized_at_genesis() {
    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    let patched_jwks = get_latest_jwkset(&client).await;
    debug!("patched_jwks={:?}", patched_jwks);
    let iss = get_sample_iss();
    let expected_providers_jwks = AllProvidersJWKs {
        entries: vec![ProviderJWKs {
            issuer: iss.into_bytes(),
            version: 0,
            jwks: vec![JWKMoveStruct::from(RSA_JWK::secure_test_jwk())],
        }],
    };
    assert_eq!(expected_providers_jwks, patched_jwks.jwks);
}

#[tokio::test]
async fn test_keyless_oidc_txn_with_bad_jwt_sig() {
    let (tw_sk, config, jwk, mut swarm, _) = setup_local_net().await;
    let (mut sig, pk) = get_sample_openid_sig_and_pk();

    match &mut sig.cert {
        EphemeralCertificate::ZeroKnowledgeSig(_) => panic!("Internal inconsistency"),
        EphemeralCertificate::OpenIdSig(openid_sig) => {
            openid_sig.jwt_sig = vec![0u8; 16] // Mauling the signature
        },
    }

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_transaction(&mut info, sig, pk, &jwk, &config, Some(&tw_sk)).await;

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
async fn test_keyless_oidc_txn_with_expired_epk() {
    let (tw_sk, config, jwk, mut swarm, _) = setup_local_net().await;
    let (mut sig, pk) = get_sample_openid_sig_and_pk();

    sig.exp_date_secs = 1; // This should fail the verification since the expiration date is way in the past

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_transaction(&mut info, sig, pk, &jwk, &config, Some(&tw_sk)).await;

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
async fn test_keyless_groth16_verifies() {
    let (_, _, mut swarm, signed_txn) = get_transaction(get_sample_groth16_sig_and_pk).await;

    info!("Submit keyless Groth16 transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with keyless Groth16 TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_keyless_groth16_verifies_no_extra_field() {
    let (_, _, mut swarm, signed_txn) =
        get_transaction(get_sample_groth16_sig_and_pk_no_extra_field).await;

    info!("Submit keyless Groth16 transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with keyless Groth16 TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_keyless_groth16_verifies_no_training_wheels() {
    let (_tw_sk, config, jwk, mut swarm, mut cli) = setup_local_net().await;
    let (sig, pk) = get_sample_groth16_sig_and_pk();

    let mut info = swarm.aptos_public_info();
    let signed_txn =
        sign_transaction(&mut info, sig.clone(), pk.clone(), &jwk, &config, None).await;

    remove_training_wheels(&mut swarm, &mut cli).await;

    info!("Submit keyless Groth16 transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with keyless Groth16 TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_keyless_groth16_verifies_using_rust_sdk() {
    let (_tw_sk, _, _, mut swarm, mut cli) = setup_local_net().await;

    let jwt = get_sample_jwt_token();
    let blinder = get_sample_epk_blinder();
    let exp_date = get_sample_exp_date();
    let esk = get_sample_esk();
    let ephemeral_key_pair = EphemeralKeyPair::new(esk, exp_date, blinder).unwrap();
    let pepper = get_sample_pepper();

    // let zk_sig_bytes = hex::decode("00dff05e7569a58de0bc941ee362edc3fcf4819e96b3b78768f53b8d046ccbe3103f13ad4056f6fe690f20b9a0ccafe203c2dc647acff644f935da9ea9433be803c533f0a8c44c6acce49883eb33e85b9742de911b9eece37802bcb482d002e6a6f50b32777abefa53dab3b33669a21abd5501cf5eb8fd535eb749bd9ddae4769d80969800000000000000010040f6df72df5ef831d53b1222d8c6e7ab38e8755fa2f67e758db196d1c5d0f6afa55c8c2bcdbf3952457154c207e4ed3c55aff24e7708650c41517e88a5e9169d01").unwrap();
    // let zk_sig = ZeroKnowledgeSig::try_from(zk_sig_bytes.as_slice()).unwrap();
    let zk_sig = get_sample_zk_sig();

    let mut info = swarm.aptos_public_info();
    let keyless_account =
        KeylessAccount::new_from_jwt(jwt, ephemeral_key_pair, pepper, zk_sig).unwrap();
    let addr = info
        .create_user_account_with_any_key(&AnyPublicKey::keyless(
            keyless_account.public_key().clone(),
        ))
        .await
        .unwrap();
    info.mint(addr, 10_000_000_000).await.unwrap();

    let account = LocalAccount::new_keyless(
        keyless_account.authentication_key().account_address(),
        keyless_account,
        0,
    );

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let builder = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100));
    let signed_txn = account.sign_with_transaction_builder(builder);

    remove_training_wheels(&mut swarm, &mut cli).await;

    info!("Submit keyless Groth16 transaction");
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if let Err(e) = result {
        panic!("Error with keyless Groth16 TXN verification: {:?}", e)
    }
}

#[tokio::test]
async fn test_keyless_groth16_with_mauled_proof() {
    let (tw_sk, config, jwk, mut swarm, _) = setup_local_net().await;
    let (sig, pk) = get_sample_groth16_sig_and_pk();

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_transaction(&mut info, sig, pk, &jwk, &config, Some(&tw_sk)).await;
    let signed_txn = maul_groth16_zkp_signature(signed_txn);

    info!("Submit keyless Groth16 transaction");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!("Keyless Groth16 TXN with mauled proof should have failed verification")
    }
}

#[tokio::test]
async fn test_keyless_groth16_with_bad_tw_signature() {
    let (_tw_sk, config, jwk, mut swarm, _cli) = setup_local_net().await;
    let (sig, pk) = get_sample_groth16_sig_and_pk();

    let mut info = swarm.aptos_public_info();

    // using the sample ESK rather than the TW SK to get a bad training wheels signature
    let signed_txn =
        sign_transaction(&mut info, sig, pk, &jwk, &config, Some(&get_sample_esk())).await;

    info!("Submit keyless Groth16 transaction");
    let result = info
        .client()
        .submit_without_serializing_response(&signed_txn)
        .await;

    if result.is_ok() {
        panic!(
            "Keyless Groth16 TXN with bad training wheels signature should have failed verification"
        )
    }
}

async fn sign_transaction<'a>(
    info: &mut AptosPublicInfo<'a>,
    mut sig: KeylessSignature,
    pk: KeylessPublicKey,
    jwk: &RSA_JWK,
    config: &Configuration,
    tw_sk: Option<&Ed25519PrivateKey>,
) -> SignedTransaction {
    let addr = info
        .create_user_account_with_any_key(&AnyPublicKey::keyless(pk.clone()))
        .await
        .unwrap();
    info.mint(addr, 10_000_000_000).await.unwrap();

    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(recipient.address(), 100))
        .sender(addr)
        .sequence_number(1)
        .build();

    let esk = get_sample_esk();

    let public_inputs_hash: Option<[u8; 32]> =
        if let EphemeralCertificate::ZeroKnowledgeSig(_) = &sig.cert {
            // This will only calculate the hash if it's needed, avoiding unnecessary computation.
            Some(fr_to_bytes_le(
                &get_public_inputs_hash(&sig, &pk, jwk, config).unwrap(),
            ))
        } else {
            None
        };

    let mut txn_and_zkp = TransactionAndProof {
        message: raw_txn.clone(),
        proof: None,
    };

    // Compute the training wheels signature if not present
    match &mut sig.cert {
        EphemeralCertificate::ZeroKnowledgeSig(proof) => {
            let proof_and_statement = Groth16ProofAndStatement {
                proof: proof.proof.into(),
                public_inputs_hash: public_inputs_hash.unwrap(),
            };

            if let Some(tw_sk) = tw_sk {
                proof.training_wheels_signature = Some(EphemeralSignature::ed25519(
                    tw_sk.sign(&proof_and_statement).unwrap(),
                ));
            }

            txn_and_zkp.proof = Some(proof.proof);
        },
        EphemeralCertificate::OpenIdSig(_) => {},
    }

    sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&txn_and_zkp).unwrap());

    SignedTransaction::new_keyless(raw_txn, pk, sig)
}

fn maul_groth16_zkp_signature(txn: SignedTransaction) -> SignedTransaction {
    // extract the keyless PK and signature
    let (pk, sig) = match txn.authenticator() {
        TransactionAuthenticator::SingleSender {
            sender: AccountAuthenticator::SingleKey { authenticator },
        } => match (authenticator.public_key(), authenticator.signature()) {
            (AnyPublicKey::Keyless { public_key }, AnySignature::Keyless { signature }) => {
                (public_key.clone(), signature.clone())
            },
            _ => panic!("Expected keyless authenticator"),
        },
        _ => panic!("Expected keyless authenticator"),
    };

    // disassemble the txn
    let raw_txn = txn.into_raw_transaction();

    test_utils::maul_raw_groth16_txn(pk, sig, raw_txn)
}

async fn get_transaction(
    get_pk_and_sig_func: fn() -> (KeylessSignature, KeylessPublicKey),
) -> (
    KeylessSignature,
    KeylessPublicKey,
    LocalSwarm,
    SignedTransaction,
) {
    let (tw_sk, config, jwk, mut swarm, _) = setup_local_net().await;

    let (sig, pk) = get_pk_and_sig_func();

    let mut info = swarm.aptos_public_info();
    let signed_txn = sign_transaction(
        &mut info,
        sig.clone(),
        pk.clone(),
        &jwk,
        &config,
        Some(&tw_sk),
    )
    .await;

    (sig, pk, swarm, signed_txn)
}

async fn setup_local_net() -> (
    Ed25519PrivateKey,
    Configuration,
    RSA_JWK,
    LocalSwarm,
    CliTestFramework,
) {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let (tw_sk, config, jwk) = spawn_network_and_execute_gov_proposals(&mut swarm, &mut cli).await;
    (tw_sk, config, jwk, swarm, cli)
}

async fn remove_training_wheels(swarm: &mut LocalSwarm, cli: &mut CliTestFramework) {
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    let jwk_patch_script = format!(
        r#"
script {{
use aptos_framework::{};
use aptos_framework::aptos_governance;
use std::option;
fun main(core_resources: &signer) {{
    let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
    {}::update_training_wheels(&framework_signer, option::none());
}}
}}
"#,
        KEYLESS_ACCOUNT_MODULE_NAME, KEYLESS_ACCOUNT_MODULE_NAME
    );
    let txn_summary = cli.run_script(root_idx, &jwk_patch_script).await.unwrap();
    debug!("txn_summary={:?}", txn_summary);

    let mut info = swarm.aptos_public_info();

    // Increment sequence number as we ran a governance proposal
    info.root_account().increment_sequence_number();

    let configuration_type_tag = format!("0x1::{}::Configuration", KEYLESS_ACCOUNT_MODULE_NAME);
    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(
            AccountAddress::ONE,
            configuration_type_tag.as_str(),
        )
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("Keyless configuration after: {:?}", config);
}

async fn spawn_network_and_execute_gov_proposals(
    swarm: &mut LocalSwarm,
    cli: &mut CliTestFramework,
) -> (Ed25519PrivateKey, Configuration, RSA_JWK) {
    let client = swarm.validators().next().unwrap().rest_client();
    let root_idx = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(60))
        .await
        .expect("Epoch 2 taking too long to come!");

    let vk_type_tag = format!(
        "0x1::{}::Groth16VerificationKey",
        KEYLESS_ACCOUNT_MODULE_NAME
    );
    let maybe_response = client
        .get_account_resource_bcs::<Groth16VerificationKey>(
            AccountAddress::ONE,
            vk_type_tag.as_str(),
        )
        .await;
    let vk = maybe_response.unwrap().into_inner();
    println!("Groth16 VK: {:?}", vk);

    let configuration_type_tag = format!("0x1::{}::Configuration", KEYLESS_ACCOUNT_MODULE_NAME);
    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(
            AccountAddress::ONE,
            configuration_type_tag.as_str(),
        )
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("Keyless configuration before: {:?}", config);

    let iss = get_sample_iss();
    let jwk = get_sample_jwk();

    let training_wheels_sk = get_sample_tw_sk();
    let training_wheels_pk = training_wheels_sk.public_key();

    info!("Insert a JWK.");
    let jwk_patch_script = format!(
        r#"
script {{
use aptos_framework::jwks;
use aptos_framework::{};
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

    {}::update_max_exp_horizon(&framework_signer, {});
    {}::update_training_wheels(&framework_signer, option::some(x"{}"));
}}
}}
"#,
        KEYLESS_ACCOUNT_MODULE_NAME,
        jwk.kid,
        jwk.alg,
        jwk.e,
        jwk.n,
        iss,
        KEYLESS_ACCOUNT_MODULE_NAME,
        Configuration::new_for_testing().max_exp_horizon_secs,
        KEYLESS_ACCOUNT_MODULE_NAME,
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
            jwks: vec![JWKMoveStruct::from(JWK::RSA(jwk.clone()))],
        }],
    };
    assert_eq!(expected_providers_jwks, patched_jwks.jwks);

    let maybe_response = client
        .get_account_resource_bcs::<Configuration>(
            AccountAddress::ONE,
            configuration_type_tag.as_str(),
        )
        .await;
    let config = maybe_response.unwrap().into_inner();
    println!("Keyless configuration after: {:?}", config);

    let mut info = swarm.aptos_public_info();

    // Increment sequence number since we patched a JWK
    info.root_account().increment_sequence_number();

    (training_wheels_sk, config, jwk)
}

async fn get_latest_jwkset(rest_client: &Client) -> PatchedJWKs {
    let maybe_response = rest_client
        .get_account_resource_bcs::<PatchedJWKs>(AccountAddress::ONE, "0x1::jwks::PatchedJWKs")
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}
