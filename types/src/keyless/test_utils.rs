// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{Groth16ProofAndStatement, Pepper, TransactionAndProof};
use crate::{
    jwks::rsa::RSA_JWK,
    keyless::{
        base64url_encode_str,
        circuit_testcases::{
            sample_jwt_payload_json, SAMPLE_EPK, SAMPLE_EPK_BLINDER, SAMPLE_ESK, SAMPLE_EXP_DATE,
            SAMPLE_EXP_HORIZON_SECS, SAMPLE_JWK, SAMPLE_JWK_SK, SAMPLE_JWT_EXTRA_FIELD,
            SAMPLE_JWT_HEADER_B64, SAMPLE_JWT_HEADER_JSON, SAMPLE_JWT_PARSED, SAMPLE_PEPPER,
            SAMPLE_PK, SAMPLE_PROOF, SAMPLE_PROOF_FOR_UPGRADED_VK, SAMPLE_PROOF_NO_EXTRA_FIELD,
            SAMPLE_UID_KEY, SAMPLE_UID_VAL, SAMPLE_UPGRADED_VK,
        },
        get_public_inputs_hash,
        proof_simulation::Groth16SimulatorBn254,
        zkp_sig::ZKP,
        Configuration, EphemeralCertificate, FederatedKeylessPublicKey, Groth16Proof,
        KeylessPublicKey, KeylessSignature, OpenIdSig, ZeroKnowledgeSig,
    },
    transaction::{authenticator::EphemeralSignature, RawTransaction, SignedTransaction},
};
use velor_crypto::{
    ed25519::Ed25519PrivateKey, poseidon_bn254::keyless::fr_to_bytes_le, SigningKey, Uniform,
};
use ark_bn254::Bn254;
use ark_groth16::{prepare_verifying_key, PreparedVerifyingKey};
use base64::{encode_config, URL_SAFE_NO_PAD};
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use ring::{signature, signature::RsaKeyPair};

static DUMMY_EPHEMERAL_SIGNATURE: Lazy<EphemeralSignature> = Lazy::new(|| {
    let sk = Ed25519PrivateKey::generate_for_testing();
    // Signing the sample proof, for lack of any other dummy struct to sign.
    EphemeralSignature::ed25519(sk.sign::<Groth16Proof>(&SAMPLE_PROOF).unwrap())
});

pub fn get_sample_esk() -> Ed25519PrivateKey {
    // Cloning is disabled outside #[cfg(test)]
    let serialized: &[u8] = &(SAMPLE_ESK.to_bytes());
    Ed25519PrivateKey::try_from(serialized).unwrap()
}

pub fn get_sample_tw_sk() -> Ed25519PrivateKey {
    let sk_bytes =
        hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap();
    Ed25519PrivateKey::try_from(sk_bytes.as_slice()).unwrap()
}

pub fn get_sample_iss() -> String {
    SAMPLE_JWT_PARSED.oidc_claims.iss.clone()
}

pub fn get_sample_aud() -> String {
    SAMPLE_JWT_PARSED.oidc_claims.aud.clone()
}

pub fn get_sample_jwk() -> RSA_JWK {
    SAMPLE_JWK.clone()
}

pub fn get_sample_pepper() -> Pepper {
    SAMPLE_PEPPER.clone()
}

pub fn get_sample_epk_blinder() -> Vec<u8> {
    SAMPLE_EPK_BLINDER.clone()
}

pub fn get_sample_exp_date() -> u64 {
    SAMPLE_EXP_DATE
}

pub fn get_sample_jwt_header_json() -> String {
    SAMPLE_JWT_HEADER_JSON.to_string()
}

pub fn get_sample_uid_key() -> String {
    SAMPLE_UID_KEY.to_string()
}

pub fn get_sample_uid_val() -> String {
    SAMPLE_UID_VAL.to_string()
}

pub fn get_sample_groth16_zkp_and_statement() -> Groth16ProofAndStatement {
    let config = Configuration::new_for_testing();
    let (sig, pk) = get_sample_groth16_sig_and_pk();
    let public_inputs_hash =
        fr_to_bytes_le(&get_public_inputs_hash(&sig, &pk, &SAMPLE_JWK, &config).unwrap());

    let proof = match sig.cert {
        EphemeralCertificate::ZeroKnowledgeSig(ZeroKnowledgeSig {
            proof,
            exp_horizon_secs: _,
            extra_field: _,
            override_aud_val: _,
            training_wheels_signature: _,
        }) => proof,
        _ => unreachable!(),
    };

    Groth16ProofAndStatement {
        proof: match proof {
            ZKP::Groth16(proof) => proof,
        },
        public_inputs_hash,
    }
}

pub fn get_sample_zk_sig() -> ZeroKnowledgeSig {
    let proof = *SAMPLE_PROOF;

    let mut zks = ZeroKnowledgeSig {
        proof: proof.into(),
        extra_field: Some(SAMPLE_JWT_EXTRA_FIELD.to_string()),
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    };

    let sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    let public_inputs_hash = fr_to_bytes_le(
        &get_public_inputs_hash(
            &sig,
            &SAMPLE_PK.clone(),
            &SAMPLE_JWK,
            &Configuration::new_for_testing(),
        )
        .unwrap(),
    );

    let proof_and_statement = Groth16ProofAndStatement {
        proof,
        public_inputs_hash,
    };

    zks.training_wheels_signature = Some(EphemeralSignature::ed25519(
        get_sample_tw_sk().sign(&proof_and_statement).unwrap(),
    ));
    zks
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_random_simulated_groth16_sig_and_pk() -> (
    KeylessSignature,
    KeylessPublicKey,
    PreparedVerifyingKey<Bn254>,
) {
    // We need a ZeroKnowledgeSig inside of a KeylessSignature to derive a public input hash. The Groth16 proof
    // is not used to actually derive the hash so we can temporarily give a dummy
    // proof before later replacing it with a simulated proof
    let dummy_proof = *SAMPLE_PROOF;
    let mut zks = ZeroKnowledgeSig {
        proof: ZKP::Groth16(dummy_proof),
        extra_field: Some(SAMPLE_JWT_EXTRA_FIELD.to_string()),
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    };
    let mut sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };
    let pk = SAMPLE_PK.clone();
    let rsa_jwk = get_sample_jwk();
    let config = Configuration::new_for_testing();
    let pih = get_public_inputs_hash(&sig, &pk, &rsa_jwk, &config).unwrap();

    let mut rng = rand::thread_rng();
    let (sim_pk, vk) =
        Groth16SimulatorBn254::circuit_agnostic_setup_with_trapdoor(&mut rng, 1).unwrap();
    let proof = Groth16SimulatorBn254::create_random_proof_with_trapdoor(&[pih], &sim_pk, &mut rng)
        .unwrap();
    let pvk = prepare_verifying_key(&vk);

    // Replace dummy proof with the simulated proof
    zks.proof = ZKP::Groth16(proof);
    sig.cert = EphemeralCertificate::ZeroKnowledgeSig(zks.clone());

    (sig, pk, pvk)
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_groth16_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let zks = get_sample_zk_sig();

    let sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (sig, SAMPLE_PK.clone())
}

pub fn get_sample_groth16_sig_and_fed_pk(
    jwk_addr: AccountAddress,
) -> (KeylessSignature, FederatedKeylessPublicKey) {
    let zks = get_sample_zk_sig();

    let sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    let fed_pk = FederatedKeylessPublicKey {
        jwk_addr,
        pk: SAMPLE_PK.clone(),
    };

    (sig, fed_pk)
}

pub fn get_upgraded_vk() -> PreparedVerifyingKey<Bn254> {
    SAMPLE_UPGRADED_VK.clone()
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_groth16_sig_and_pk_for_upgraded_vk() -> (KeylessSignature, KeylessPublicKey) {
    let proof = *SAMPLE_PROOF_FOR_UPGRADED_VK;

    let zks = ZeroKnowledgeSig {
        proof: proof.into(),
        extra_field: Some(SAMPLE_JWT_EXTRA_FIELD.to_string()),
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    };

    let sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (sig, SAMPLE_PK.clone())
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_groth16_sig_and_pk_no_extra_field() -> (KeylessSignature, KeylessPublicKey) {
    let proof = *SAMPLE_PROOF_NO_EXTRA_FIELD;

    let zks = ZeroKnowledgeSig {
        proof: proof.into(),
        extra_field: None,
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    };

    let sig = KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(zks.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (sig, SAMPLE_PK.clone())
}

pub fn get_sample_jwt_token() -> String {
    get_sample_jwt_token_from_payload(sample_jwt_payload_json().as_str())
}

pub fn get_sample_jwt_token_from_payload(payload: &str) -> String {
    let jwt_header_b64 = SAMPLE_JWT_HEADER_B64.to_string();
    let jwt_payload_b64 = base64url_encode_str(payload);
    let msg = jwt_header_b64.clone() + "." + jwt_payload_b64.as_str();
    let rng = ring::rand::SystemRandom::new();
    let sk = &*SAMPLE_JWK_SK;
    let mut jwt_sig = vec![0u8; sk.public_modulus_len()];

    sk.sign(
        &signature::RSA_PKCS1_SHA256,
        &rng,
        msg.as_bytes(),
        jwt_sig.as_mut_slice(),
    )
    .unwrap();

    let base64url_string = encode_config(jwt_sig.clone(), URL_SAFE_NO_PAD);

    format!("{}.{}", msg, base64url_string)
}

pub fn oidc_provider_sign(sk: &RsaKeyPair, msg: &[u8]) -> Vec<u8> {
    let mut jwt_sig = vec![0u8; sk.public_modulus_len()];
    let rng = ring::rand::SystemRandom::new();
    sk.sign(
        &signature::RSA_PKCS1_SHA256,
        &rng,
        msg,
        jwt_sig.as_mut_slice(),
    )
    .unwrap();

    jwt_sig
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_openid_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let jwt_header_b64 = SAMPLE_JWT_HEADER_B64.to_string();
    let jwt_payload_b64 = base64url_encode_str(sample_jwt_payload_json().as_str());
    let msg = jwt_header_b64.clone() + "." + jwt_payload_b64.as_str();
    let rng = ring::rand::SystemRandom::new();
    let sk = *SAMPLE_JWK_SK;
    let mut jwt_sig = vec![0u8; sk.public_modulus_len()];

    sk.sign(
        &signature::RSA_PKCS1_SHA256,
        &rng,
        msg.as_bytes(),
        jwt_sig.as_mut_slice(),
    )
    .unwrap();

    let openid_sig = OpenIdSig {
        jwt_sig,
        jwt_payload_json: sample_jwt_payload_json().to_string(),
        uid_key: SAMPLE_UID_KEY.to_owned(),
        epk_blinder: SAMPLE_EPK_BLINDER.clone(),
        pepper: SAMPLE_PEPPER.clone(),
        idc_aud_val: None,
    };

    let zk_sig = KeylessSignature {
        cert: EphemeralCertificate::OpenIdSig(openid_sig.clone()),
        jwt_header_json: SAMPLE_JWT_HEADER_JSON.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (zk_sig, SAMPLE_PK.clone())
}

pub fn maul_raw_groth16_txn(
    pk: KeylessPublicKey,
    mut sig: KeylessSignature,
    raw_txn: RawTransaction,
) -> SignedTransaction {
    let mut txn_and_zkp = TransactionAndProof {
        message: raw_txn.clone(),
        proof: None,
    };

    // maul ephemeral signature to be over a different proof: (a, b, a) instead of (a, b, c)
    match &mut sig.cert {
        EphemeralCertificate::ZeroKnowledgeSig(proof) => {
            let ZKP::Groth16(old_proof) = proof.proof;

            txn_and_zkp.proof = Some(
                Groth16Proof::new(*old_proof.get_a(), *old_proof.get_b(), *old_proof.get_a())
                    .into(),
            );
        },
        EphemeralCertificate::OpenIdSig(_) => {},
    };

    let esk = get_sample_esk();
    sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&txn_and_zkp).unwrap());

    // reassemble TXN
    SignedTransaction::new_keyless(raw_txn, pk, sig)
}

#[cfg(test)]
mod test {
    use crate::{
        keyless::{
            circuit_testcases::{
                SAMPLE_EPK, SAMPLE_EPK_BLINDER, SAMPLE_EXP_DATE, SAMPLE_EXP_HORIZON_SECS,
                SAMPLE_JWK, SAMPLE_JWT_EXTRA_FIELD_KEY,
            },
            get_public_inputs_hash,
            test_utils::{
                get_sample_epk_blinder, get_sample_esk, get_sample_exp_date,
                get_sample_groth16_sig_and_pk, get_sample_jwt_token, get_sample_pepper,
            },
            Configuration, Groth16Proof, OpenIdSig, VERIFICATION_KEY_FOR_TESTING,
        },
        transaction::authenticator::EphemeralPublicKey,
    };
    use velor_crypto::PrivateKey;
    use ark_ff::PrimeField;
    use reqwest::Client;
    use serde_json::{json, to_string_pretty, Value};
    use std::ops::Deref;

    /// Since our proof generation toolkit is incomplete; currently doing it here.
    #[test]
    fn keyless_print_nonce_commitment_and_public_inputs_hash() {
        let config = Configuration::new_for_testing();
        let nonce = OpenIdSig::reconstruct_oauth_nonce(
            SAMPLE_EPK_BLINDER.as_slice(),
            SAMPLE_EXP_DATE,
            &SAMPLE_EPK,
            &config,
        )
        .unwrap();
        println!(
            "Nonce computed from exp_date {} and EPK blinder {}: {}",
            SAMPLE_EXP_DATE,
            hex::encode(SAMPLE_EPK_BLINDER.as_slice()),
            nonce
        );

        let (sig, pk) = get_sample_groth16_sig_and_pk();
        let public_inputs_hash = get_public_inputs_hash(&sig, &pk, &SAMPLE_JWK, &config).unwrap();

        println!("Public inputs hash: {}", public_inputs_hash);
    }

    #[derive(Debug, serde::Deserialize)]
    struct ProverResponse {
        proof: Groth16Proof,
        #[serde(with = "hex")]
        public_inputs_hash: [u8; 32],
    }

    // Run the prover service locally - https://github.com/velor-chain/keyless-zk-proofs/tree/main/prover
    // Follow the README and make sure to use port 8083
    #[ignore]
    #[tokio::test]
    async fn fetch_sample_proofs_from_prover() {
        let client = Client::new();

        let body = json!({
            "jwt_b64": get_sample_jwt_token(),
            "epk": hex::encode(bcs::to_bytes(&EphemeralPublicKey::ed25519(get_sample_esk().public_key())).unwrap()),
            "epk_blinder": hex::encode(get_sample_epk_blinder()),
            "exp_date_secs": get_sample_exp_date(),
            "exp_horizon_secs": SAMPLE_EXP_HORIZON_SECS,
            "pepper": hex::encode(get_sample_pepper().to_bytes()),
            "uid_key": "sub",
            "extra_field": SAMPLE_JWT_EXTRA_FIELD_KEY,
            "use_insecure_test_jwk": true,
        });

        println!("Request Body: {}", to_string_pretty(&body).unwrap());

        make_prover_request(&client, body, "SAMPLE_PROOF").await;

        let body = json!({
            "jwt_b64": get_sample_jwt_token(),
            "epk": hex::encode(bcs::to_bytes(&EphemeralPublicKey::ed25519(get_sample_esk().public_key())).unwrap()),
            "epk_blinder": hex::encode(get_sample_epk_blinder()),
            "exp_date_secs": get_sample_exp_date(),
            "exp_horizon_secs": SAMPLE_EXP_HORIZON_SECS,
            "pepper": hex::encode(get_sample_pepper().to_bytes()),
            "uid_key": "sub",
            "use_insecure_test_jwk": true,
        });
        make_prover_request(&client, body, "SAMPLE_PROOF_NO_EXTRA_FIELD").await;
    }

    async fn make_prover_request(
        client: &Client,
        body: Value,
        test_proof_name: &str,
    ) -> ProverResponse {
        let url = "http://localhost:8083/v0/prove";

        // Send the POST request and await the response
        let response = client.post(url).json(&body).send().await.unwrap();

        // Check if the request was successful
        if response.status().is_success() {
            let prover_response = response.json::<ProverResponse>().await.unwrap();
            let proof = prover_response.proof;
            let public_inputs_hash =
                ark_bn254::Fr::from_le_bytes_mod_order(&prover_response.public_inputs_hash);

            let code = format!(
                r#"
            Groth16Proof::new(
                G1Bytes::new_from_vec(hex::decode("{}").unwrap()).unwrap(),
                G2Bytes::new_from_vec(hex::decode("{}").unwrap()).unwrap(),
                G1Bytes::new_from_vec(hex::decode("{}").unwrap()).unwrap(),
            )
            "#,
                hex::encode(proof.get_a().0),
                hex::encode(proof.get_b().0),
                hex::encode(proof.get_c().0)
            );
            println!();
            println!(
                "----- Update the {} in circuit_testcases.rs with the output below -----",
                test_proof_name
            );
            println!("{}", code);
            println!("----------------------------------------------------------------------------------");

            // TODO: Assumes proofs are to be generated w.r.t the devnet VK. This must be manually
            //  modified to deal with generating proofs for a different VK.

            // Verify the proof with the test verifying key.  If this fails the verifying key does not match the proving used
            // to generate the proof.
            proof
                .verify_proof(public_inputs_hash, VERIFICATION_KEY_FOR_TESTING.deref())
                .unwrap();

            prover_response
        } else {
            // Print an error message if the request failed
            println!("Request failed with status code: {}", response.status());
            panic!("Prover request failed")
        }
    }
}
