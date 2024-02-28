// Copyright © Aptos Foundation

use crate::{
    jwks::rsa::RSA_JWK,
    keyless::{
        base64url_encode_str,
        circuit_testcases::{
            SAMPLE_EPK, SAMPLE_EPK_BLINDER, SAMPLE_ESK, SAMPLE_EXP_DATE, SAMPLE_EXP_HORIZON_SECS,
            SAMPLE_JWK, SAMPLE_JWK_SK, SAMPLE_JWT_EXTRA_FIELD, SAMPLE_JWT_HEADER_B64,
            SAMPLE_JWT_HEADER_DECODED, SAMPLE_JWT_PARSED, SAMPLE_JWT_PAYLOAD_DECODED,
            SAMPLE_PEPPER, SAMPLE_PK, SAMPLE_PROOF, SAMPLE_UID_KEY,
        },
        Groth16Zkp, KeylessPublicKey, KeylessSignature, OpenIdSig, SignedGroth16Zkp,
        ZkpOrOpenIdSig,
    },
    transaction::authenticator::EphemeralSignature,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, SigningKey, Uniform};
use once_cell::sync::Lazy;
use ring::signature;

static DUMMY_EPHEMERAL_SIGNATURE: Lazy<EphemeralSignature> = Lazy::new(|| {
    let sk = Ed25519PrivateKey::generate_for_testing();
    // Signing the sample proof, for lack of any other dummy thing to sign.
    EphemeralSignature::ed25519(sk.sign::<Groth16Zkp>(&SAMPLE_PROOF).unwrap())
});

pub fn get_sample_esk() -> Ed25519PrivateKey {
    // Cloning is disabled outside #[cfg(test)]
    let serialized: &[u8] = &(SAMPLE_ESK.to_bytes());
    Ed25519PrivateKey::try_from(serialized).unwrap()
}

pub fn get_sample_iss() -> String {
    SAMPLE_JWT_PARSED.oidc_claims.iss.clone()
}

pub fn get_sample_jwk() -> RSA_JWK {
    SAMPLE_JWK.clone()
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_groth16_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let proof = *SAMPLE_PROOF;

    let groth16zkp = SignedGroth16Zkp {
        proof,
        non_malleability_signature: EphemeralSignature::ed25519(SAMPLE_ESK.sign(&proof).unwrap()),
        extra_field: Some(SAMPLE_JWT_EXTRA_FIELD.to_string()),
        exp_horizon_secs: SAMPLE_EXP_HORIZON_SECS,
        override_aud_val: None,
        training_wheels_signature: None,
    };

    let zk_sig = KeylessSignature {
        sig: ZkpOrOpenIdSig::Groth16Zkp(groth16zkp.clone()),
        jwt_header: SAMPLE_JWT_HEADER_DECODED.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (zk_sig, SAMPLE_PK.clone())
}

/// Note: Does not have a valid ephemeral signature. Use the SAMPLE_ESK to compute one over the
/// desired TXN.
pub fn get_sample_openid_sig_and_pk() -> (KeylessSignature, KeylessPublicKey) {
    let jwt_header_b64 = SAMPLE_JWT_HEADER_B64.to_string();
    let jwt_payload_b64 = base64url_encode_str(SAMPLE_JWT_PAYLOAD_DECODED.as_str());
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

    let openid_sig = OpenIdSig {
        jwt_sig,
        jwt_payload: SAMPLE_JWT_PAYLOAD_DECODED.to_string(),
        uid_key: SAMPLE_UID_KEY.to_owned(),
        epk_blinder: SAMPLE_EPK_BLINDER.clone(),
        pepper: SAMPLE_PEPPER.clone(),
        idc_aud_val: None,
    };

    let zk_sig = KeylessSignature {
        sig: ZkpOrOpenIdSig::OpenIdSig(openid_sig.clone()),
        jwt_header: SAMPLE_JWT_HEADER_DECODED.to_string(),
        exp_date_secs: SAMPLE_EXP_DATE,
        ephemeral_pubkey: SAMPLE_EPK.clone(),
        ephemeral_signature: DUMMY_EPHEMERAL_SIGNATURE.clone(),
    };

    (zk_sig, SAMPLE_PK.clone())
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
