// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{Groth16ProofAndStatement, Pepper, TransactionAndProof};
use crate::{
    jwks::rsa::RSA_JWK,
    keyless::{
        base64url_encode_str,
        circuit_testcases::{
            SAMPLE_EPK, SAMPLE_EPK_BLINDER, SAMPLE_ESK, SAMPLE_EXP_DATE, SAMPLE_EXP_HORIZON_SECS,
            SAMPLE_JWK, SAMPLE_JWK_SK, SAMPLE_JWT_EXTRA_FIELD, SAMPLE_JWT_HEADER_B64,
            SAMPLE_JWT_HEADER_JSON, SAMPLE_JWT_PARSED, SAMPLE_JWT_PAYLOAD_JSON, SAMPLE_PEPPER,
            SAMPLE_PK, SAMPLE_PROOF, SAMPLE_PROOF_NO_EXTRA_FIELD, SAMPLE_UID_KEY,
        },
        get_public_inputs_hash,
        zkp_sig::ZKP,
        Configuration, EphemeralCertificate, Groth16Proof, KeylessPublicKey, KeylessSignature,
        OpenIdSig, ZeroKnowledgeSig,
    },
    transaction::{authenticator::EphemeralSignature, RawTransaction, SignedTransaction},
};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey, poseidon_bn254::fr_to_bytes_le, SigningKey, Uniform,
};
use base64::{encode_config, URL_SAFE_NO_PAD};
use once_cell::sync::Lazy;
use ring::signature;

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

    ZeroKnowledgeSig {
        proof: proof.into(),
        extra_field: Some(SAMPLE_JWT_EXTRA_FIELD.to_string()),
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    }
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_groth16_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let proof = *SAMPLE_PROOF;

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
    let jwt_header_b64 = SAMPLE_JWT_HEADER_B64.to_string();
    let jwt_payload_b64 = base64url_encode_str(SAMPLE_JWT_PAYLOAD_JSON.as_str());
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

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_openid_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let jwt_header_b64 = SAMPLE_JWT_HEADER_B64.to_string();
    let jwt_payload_b64 = base64url_encode_str(SAMPLE_JWT_PAYLOAD_JSON.as_str());
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

    println!("JWT token is: {}.{}", msg, base64url_string);

    let openid_sig = OpenIdSig {
        jwt_sig,
        jwt_payload_json: SAMPLE_JWT_PAYLOAD_JSON.to_string(),
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
    use crate::keyless::{
        circuit_testcases::{SAMPLE_EPK, SAMPLE_EPK_BLINDER, SAMPLE_EXP_DATE, SAMPLE_JWK},
        get_public_inputs_hash,
        test_utils::get_sample_groth16_sig_and_pk,
        Configuration, OpenIdSig,
    };

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
}
