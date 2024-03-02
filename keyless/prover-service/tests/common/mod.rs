use figment::{
    providers::{Format as _, Yaml},
    Figment,
};

use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
};
use aptos_types::{keyless::Pepper, transaction::authenticator::EphemeralPublicKey};
use rust_rapidsnark::FullProver;
use serde::Serialize;
use serde_json::Value;
use std::{fs, str::FromStr};
use prover_service::{
    handlers::encode_proof, input_conversion::{
        preprocess,
        config::CircuitConfig, derive_circuit_input_signals}, config::ProverServerConfig};


use crate::common::types::{ProofTestCase};

use self::types::{DefaultTestJWKKeyPair, TestJWKKeyPair, WithNonce};

pub mod types;
pub mod load_vk;

use load_vk::prepared_vk;


pub fn init_test_full_prover() -> FullProver {
    let prover_server_config = Figment::new()
        .merge(Yaml::file("config.yml"))
        .extract()
        .expect("Couldn't load config file");
    let ProverServerConfig {
        zkey_path,
        witness_gen_binary_folder_path,
        test_verification_key_path: _,
        oidc_providers: _,
        jwk_refresh_rate_secs: _,
        port: _,
        metrics_port: _,
    } = prover_server_config;

    let full_prover = FullProver::new(&zkey_path, &witness_gen_binary_folder_path)
        .expect("failed to initialize rapidsnark prover");

    full_prover
}

pub fn get_test_circuit_config() -> CircuitConfig {
    serde_yaml::from_str(&fs::read_to_string("conversion_config.yml").expect("Unable to read file"))
        .expect("should parse correctly")
}

pub fn gen_test_ephemeral_pk() -> EphemeralPublicKey {
    let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
        .decode_key(
            "zkid test ephemeral private key",
            "0x76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
                .as_bytes()
                .to_vec(),
        )
        .unwrap();
    let ephemeral_public_key_unwrapped: Ed25519PublicKey =
        Ed25519PublicKey::from(&ephemeral_private_key);
    let epk = EphemeralPublicKey::ed25519(ephemeral_public_key_unwrapped);
    epk
}


pub fn gen_test_ephemeral_pk_blinder() -> ark_bn254::Fr {
    ark_bn254::Fr::from_str("42").unwrap()
}

pub fn gen_test_jwk_keypair() -> impl TestJWKKeyPair {
    let mut rng = rsa::rand_core::OsRng;
    let jwk_keypair =
        DefaultTestJWKKeyPair::new_with_kid_and_exp(&mut rng, 
                                                    "tesk_jwk",
                                                    num_bigint::BigUint::from_str("65537").unwrap()).unwrap();
    jwk_keypair
}

pub fn get_test_pepper() -> Pepper {
    Pepper::from_number(42)
}

pub fn get_config() -> ProverServerConfig {
    Figment::new()
    .merge(Yaml::file("config.yml"))
        .extract().expect("Couldn't load config file")
}


pub fn convert_prove_and_verify(testcase: &ProofTestCase<impl Serialize + WithNonce + Clone>) -> Result<(), anyhow::Error> {
    let mut full_prover = init_test_full_prover();
    let circuit_config = get_test_circuit_config();
    let jwk_keypair = gen_test_jwk_keypair();
    let prover_server_config = get_config();

    let prover_request_input = testcase.convert_to_prover_request(&jwk_keypair);

    println!("Prover request: {}", serde_json::to_string_pretty(&prover_request_input).unwrap());

    let (circuit_input_signals, public_inputs_hash) = derive_circuit_input_signals(
        preprocess::decode_and_add_jwk(prover_request_input).unwrap(),
        &circuit_config,
        Some(&jwk_keypair.into_rsa_jwk()),
    )
    .unwrap();

    let formatted_input_str = serde_json::to_string(&circuit_input_signals.to_json_value()).unwrap();
    let (json, _) = full_prover.prove(&formatted_input_str).unwrap();
    let g16p = encode_proof(&Value::from_str(json).unwrap());

    let g16vk = prepared_vk(&prover_server_config.test_verification_key_path);
    g16p.verify_proof(public_inputs_hash, &g16vk)
}




