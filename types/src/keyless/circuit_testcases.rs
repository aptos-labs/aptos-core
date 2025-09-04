// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//^ This file stores the details associated with a sample ZK proof. The constants are outputted by
//^ `input_gen.py` in the `keyless-circuit` repo (or can be derived implicitly from that code).

use crate::{
    jwks::{
        insecure_test_rsa_jwk,
        rsa::{INSECURE_TEST_RSA_KEY_PAIR, RSA_JWK},
    },
    keyless::{
        base64url_encode_str,
        bn254_circom::{G1Bytes, G2Bytes},
        g1_projective_str_to_affine, g2_projective_str_to_affine, Claims, Configuration,
        Groth16Proof, IdCommitment, KeylessPublicKey, OpenIdSig, Pepper,
    },
    transaction::authenticator::EphemeralPublicKey,
};
use velor_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use ark_bn254::Bn254;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use once_cell::sync::Lazy;
use ring::signature::RsaKeyPair;

/// The JWT header, decoded as JSON
pub(crate) static SAMPLE_JWT_HEADER_JSON: Lazy<String> = Lazy::new(|| {
    format!(
        r#"{{"alg":"{}","typ":"JWT","kid":"{}"}}"#,
        SAMPLE_JWK.alg.as_str(),
        SAMPLE_JWK.kid.as_str()
    )
});

/// The JWT header, base64url-encoded
pub(crate) static SAMPLE_JWT_HEADER_B64: Lazy<String> =
    Lazy::new(|| base64url_encode_str(SAMPLE_JWT_HEADER_JSON.as_str()));

/// The JWT payload, decoded as JSON

pub static SAMPLE_NONCE: Lazy<String> = Lazy::new(|| {
    let config = Configuration::new_for_testing();
    OpenIdSig::reconstruct_oauth_nonce(
        SAMPLE_EPK_BLINDER.as_slice(),
        SAMPLE_EXP_DATE,
        &SAMPLE_EPK,
        &config,
    )
    .unwrap()
});

pub const SAMPLE_TEST_ISS_VALUE: &str = "test.oidc.provider";

pub fn sample_jwt_payload_json() -> String {
    sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        SAMPLE_JWT_EXTRA_FIELD.as_str(),
        SAMPLE_JWT_IAT,
        SAMPLE_NONCE.as_str(),
    )
}

pub fn render_jwt_payload_json(
    iss: &str,
    aud: &str,
    uid_key: &str,
    uid_val: &str,
    extra_field: &str,
    iat: u64,
    nonce: &str,
    exp: u64,
) -> String {
    format!(
        r#"{{
            "iss":"{}",
            "aud":"{}",
            "{}":"{}",
            {}
            "iat":{},
            "nonce":"{}",
            "exp":{}
        }}
        "#,
        iss, aud, uid_key, uid_val, extra_field, iat, nonce, exp
    )
}

pub fn sample_jwt_payload_json_overrides(
    iss: &str,
    uid_val: &str,
    extra_field: &str,
    iat: u64,
    nonce: &str,
) -> String {
    format!(
        r#"{{
            "iss":"{}",
            "azp":"407408718192.apps.googleusercontent.com",
            "aud":"407408718192.apps.googleusercontent.com",
            "sub":"{}",
            "hd":"velorlabs.com",
            "email":"michael@velorlabs.com",
            "email_verified":true,
            "at_hash":"bxIESuI59IoZb5alCASqBg",
            "name":"Michael Straka",
            "picture":"https://lh3.googleusercontent.com/a/ACg8ocJvY4kVUBRtLxe1IqKWL5i7tBDJzFp9YuWVXMzwPpbs=s96-c",
            "given_name":"Michael",
            {}
            "locale":"en",
            "iat":{},
            "nonce":"{}",
            "exp":2700259544
         }}"#,
        iss, uid_val, extra_field, iat, nonce
    )
}

/// An example IAT.
pub const SAMPLE_JWT_IAT: u64 = 1700255944;

/// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
pub(crate) const SAMPLE_JWT_EXTRA_FIELD_KEY: &str = "family_name";

/// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
pub static SAMPLE_JWT_EXTRA_FIELD: Lazy<String> =
    Lazy::new(|| format!(r#""{}":"Straka","#, SAMPLE_JWT_EXTRA_FIELD_KEY));

/// The JWT parsed as a struct
pub(crate) static SAMPLE_JWT_PARSED: Lazy<Claims> =
    Lazy::new(|| serde_json::from_str(sample_jwt_payload_json().as_str()).unwrap());

pub(crate) static SAMPLE_JWK: Lazy<RSA_JWK> = Lazy::new(insecure_test_rsa_jwk);

/// This is the SK from https://token.dev/.
/// To convert it into a JSON, you can use https://irrte.ch/jwt-js-decode/pem2jwk.html
pub static SAMPLE_JWK_SK: Lazy<&RsaKeyPair> = Lazy::new(|| &*INSECURE_TEST_RSA_KEY_PAIR);

pub(crate) const SAMPLE_UID_KEY: &str = "sub";

pub const SAMPLE_UID_VAL: &str = "113990307082899718775";

/// The nonce-committed expiration date (not the JWT `exp`), 12/21/5490
pub const SAMPLE_EXP_DATE: u64 = 111_111_111_111;

/// ~31,710 years
pub(crate) const SAMPLE_EXP_HORIZON_SECS: u64 = 999_999_999_999;

pub(crate) static SAMPLE_PEPPER: Lazy<Pepper> = Lazy::new(|| Pepper::from_number(76));

pub(crate) static SAMPLE_ESK: Lazy<Ed25519PrivateKey> =
    Lazy::new(Ed25519PrivateKey::generate_for_testing);

pub(crate) static SAMPLE_EPK: Lazy<EphemeralPublicKey> =
    Lazy::new(|| EphemeralPublicKey::ed25519(SAMPLE_ESK.public_key()));

pub(crate) static SAMPLE_EPK_BLINDER: Lazy<Vec<u8>> = Lazy::new(|| {
    let mut byte_vector = vec![0; 31];
    byte_vector[0] = 42;
    byte_vector
});

pub(crate) static SAMPLE_PK: Lazy<KeylessPublicKey> = Lazy::new(|| {
    assert_eq!(SAMPLE_UID_KEY, "sub");

    KeylessPublicKey {
        iss_val: SAMPLE_JWT_PARSED.oidc_claims.iss.to_owned(),
        idc: IdCommitment::new_from_preimage(
            &SAMPLE_PEPPER,
            SAMPLE_JWT_PARSED.oidc_claims.aud.as_str(),
            SAMPLE_UID_KEY,
            SAMPLE_JWT_PARSED.oidc_claims.sub.as_str(),
        )
        .unwrap(),
    }
});

/// A valid Groth16 proof for the JWT under `SAMPLE_JWK`, where the public inputs have:
///  - uid_key set to `sub`
///  - no override aud
///  - the extra field enabled
/// https://github.com/velor-chain/devnet-groth16-keys/commit/02e5675f46ce97f8b61a4638e7a0aaeaa4351f76
pub(crate) static SAMPLE_PROOF: Lazy<Groth16Proof> = Lazy::new(|| {
    Groth16Proof::new(
        G1Bytes::new_from_vec(hex::decode("3304cc0defd488d770af0439480ec24c8473b30dbcbfad9fdf99ca62256bd908").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("2f432b9459375ed2032bcb1ff3ccc1dd5d05a752d6956d2bb003f4e3b42d0b242cf4ab4d3dc8dc700ede17bbfeaddedd42033691e3d85ff8d6621663cb2e779a").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("d44ee2772f4b48fdb0dbd8d870d3fb4401cd3a28fbdde535e9c57bac9a263f9c").unwrap()).unwrap(),
    )
});

/// A valid Groth16 proof for the JWT under `SAMPLE_JWK`, where the public inputs have:
///  - uid_key set to `sub`
///  - no override aud
///  - no extra field
/// https://github.com/velor-chain/devnet-groth16-keys/commit/02e5675f46ce97f8b61a4638e7a0aaeaa4351f76
pub(crate) static SAMPLE_PROOF_NO_EXTRA_FIELD: Lazy<Groth16Proof> = Lazy::new(|| {
    Groth16Proof::new(
        G1Bytes::new_from_vec(hex::decode("bdfda383c9131ab44dd3d8efe65c59842b28e17467e2d07c4020742407c580a7").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("d27b4c0296ec1045dd050894c635095c25ff8d89c8adf5da401b3434639c560550e3da14e5ec953769aac9d256ddc9b2a8071c021f271f0937fd5be404f2b919").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("52a25b0b58013a77f8713105d7e0f817468bbdd25d644e9f2a9b3eabd7d4bc17").unwrap()).unwrap(),
    )
});

/// A new Groth16 VK to test the VK rotation.  Using https://raw.githubusercontent.com/velor-chain/devnet-groth16-keys/refs/heads/master/verification_key.json
pub(crate) static SAMPLE_UPGRADED_VK: Lazy<PreparedVerifyingKey<Bn254>> = Lazy::new(|| {
    let alpha_g1 = g1_projective_str_to_affine(
        "20491192805390485299153009773594534940189261866228447918068658471970481763042",
        "9383485363053290200918347156157836566562967994039712273449902621266178545958",
    )
    .unwrap();

    let beta_g2 = g2_projective_str_to_affine(
        [
            "6375614351688725206403948262868962793625744043794305715222011528459656738731",
            "4252822878758300859123897981450591353533073413197771768651442665752259397132",
        ],
        [
            "10505242626370262277552901082094356697409835680220590971873171140371331206856",
            "21847035105528745403288232691147584728191162732299865338377159692350059136679",
        ],
    )
    .unwrap();

    let gamma_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let delta_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        g1_projective_str_to_affine(
            "3314139460766150258181182511839382093976747705712051605578952681462625768062",
            "15177929890957116336235565528373348502554233971408496072173139426537995658198",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "11040819149070528816396253292991080175919431363817777522273571096667537087166",
            "13976660124609527451731647657081915019685631850685519260597009755390746148997",
        )
        .unwrap(),
    ] {
        gamma_abc_g1.push(points);
    }

    let vk = VerifyingKey {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    };

    // println!("SAMPLE_UPGRADED_VK: {}", Groth16VerificationKey::from(&PreparedVerifyingKey::from(vk)).hash());

    PreparedVerifyingKey::from(vk)
});

/// Like `SAMPLE_PROOF` but w.r.t. to `SAMPLE_UPGRADED_VK`.
pub(crate) static SAMPLE_PROOF_FOR_UPGRADED_VK: Lazy<Groth16Proof> = Lazy::new(|| {
    Groth16Proof::new(
        G1Bytes::new_from_vec(hex::decode("4889b1896f0335f8d375370879136577633c8f7ff6957e66bbb10afe244dfa95").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("f0e1971b492baf5aff3c5ab2c0083fe2bca911d7414416ca160fce4ae9290d07457a6820251d08500f5f4c3d680b063c7bbb0ab4fe52a509f175cf02ef6afe18").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("7aab5c31cf2fc43acb9cc470e28c7917259a424a2c9d53fbc1c473d49c302c8d").unwrap()).unwrap(),
    )
    // println!("SAMPLE_PROOF_FOR_UPGRADED_VK: {}", &proof.hash());
});
