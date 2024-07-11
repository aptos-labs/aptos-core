// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//^ This file stores the details associated with a sample ZK proof. The constants are outputted by
//^ `input_gen.py` in the `keyless-circuit` repo (or can be derived implicitly from that code).

use crate::{
    jwks::{
        insecure_test_rsa_jwk,
        rsa::{INSECURE_TEST_RSA_KEY_PAIR, RSA_JWK},
    },
    keyless::{
        configuration,
        base64url_encode_str,
        bn254_circom::{G1Bytes, G2Bytes},
        g1_projective_str_to_affine, g2_projective_str_to_affine, Claims, Configuration,
        Groth16Proof, IdCommitment, KeylessPublicKey, OpenIdSig, Pepper,
    },
    transaction::authenticator::EphemeralPublicKey,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
use ark_bn254::Bn254;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use once_cell::sync::Lazy;
use ring::signature::RsaKeyPair;

/// The JWT header, decoded as JSON
pub(crate) static SAMPLE_JWT_HEADER_JSON: Lazy<String> = Lazy::new(|| {
    format!(
        r#"{{"alg":"{}","kid":"{}","typ":"JWT"}}"#,
        SAMPLE_JWK.alg.as_str(),
        SAMPLE_JWK.kid.as_str()
    )
});

/// The JWT header, base64url-encoded
pub(crate) static SAMPLE_JWT_HEADER_B64: Lazy<String> =
    Lazy::new(|| base64url_encode_str(SAMPLE_JWT_HEADER_JSON.as_str()));

/// The JWT payload, decoded as JSON

static SAMPLE_NONCE: Lazy<String> = Lazy::new(|| {
    let config = Configuration::new_for_testing();
    OpenIdSig::reconstruct_oauth_nonce(
        SAMPLE_EPK_BLINDER.as_slice(),
        SAMPLE_EXP_DATE,
        &SAMPLE_EPK,
        &config,
    )
    .unwrap()
});

pub(crate) const SAMPLE_TEST_ISS_VALUE: &str = "test.oidc.provider";

pub(crate) static SAMPLE_JWT_PAYLOAD_JSON: Lazy<String> = Lazy::new(|| {
    format!(
        r#"{{
            "iss":"{}",
            "azp":"407408718192.apps.googleusercontent.com",
            "aud":"407408718192.apps.googleusercontent.com",
            "sub":"113990307082899718775",
            "hd":"aptoslabs.com",
            "email":"michael@aptoslabs.com",
            "email_verified":true,
            "at_hash":"bxIESuI59IoZb5alCASqBg",
            "name":"Michael Straka",
            "picture":"https://lh3.googleusercontent.com/a/ACg8ocJvY4kVUBRtLxe1IqKWL5i7tBDJzFp9YuWVXMzwPpbs=s96-c",
            "given_name":"Michael",
            {}
            "locale":"en",
            "iat":1700255944,
            "nonce":"{}",
            "exp":2700259544
         }}"#,
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_JWT_EXTRA_FIELD.as_str(),
        SAMPLE_NONCE.as_str()
    )
});

/// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
pub(crate) const SAMPLE_JWT_EXTRA_FIELD_KEY: &str = "family_name";

/// Consistent with what is in `SAMPLE_JWT_PAYLOAD_JSON`
pub(crate) static SAMPLE_JWT_EXTRA_FIELD: Lazy<String> =
    Lazy::new(|| format!(r#""{}":"Straka","#, SAMPLE_JWT_EXTRA_FIELD_KEY));

/// The JWT parsed as a struct
pub(crate) static SAMPLE_JWT_PARSED: Lazy<Claims> =
    Lazy::new(|| serde_json::from_str(SAMPLE_JWT_PAYLOAD_JSON.as_str()).unwrap());

pub(crate) static SAMPLE_JWK: Lazy<RSA_JWK> = Lazy::new(insecure_test_rsa_jwk);

/// This is the SK from https://token.dev/.
/// To convert it into a JSON, you can use https://irrte.ch/jwt-js-decode/pem2jwk.html
pub(crate) static SAMPLE_JWK_SK: Lazy<&RsaKeyPair> = Lazy::new(|| &*INSECURE_TEST_RSA_KEY_PAIR);

pub(crate) const SAMPLE_UID_KEY: &str = "sub";

/// The nonce-committed expiration date (not the JWT `exp`), 12/21/5490
pub(crate) const SAMPLE_EXP_DATE: u64 = 111_111_111_111;

/// ~31,710 years
pub(crate) const SAMPLE_EXP_HORIZON_SECS: u64 = configuration::TESTING_EXP_HORIZON_SECS;

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
/// https://github.com/aptos-labs/devnet-groth16-keys/commit/02e5675f46ce97f8b61a4638e7a0aaeaa4351f76
pub(crate) static SAMPLE_PROOF: Lazy<Groth16Proof> = Lazy::new(|| {
    Groth16Proof::new(
        G1Bytes::new_from_vec(hex::decode("95030afdb785624d3f305655579775fe216a4780496ccb7abe899dc8a7bcf798").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("08d5814954cb04ac1f80771f783585162abb2cd673203f7ee22ec87a783f431cc649ca449bc1d2d93ba4aaecd382f94306c23003ed1e1c0592a46c64e5cf6504").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("117177343a361982883579c3f49eb30b05d79b7ca1e96556be7ccb2ee88ea391").unwrap()).unwrap(),
    )
});

/// A valid Groth16 proof for the JWT under `SAMPLE_JWK`, where the public inputs have:
///  - uid_key set to `sub`
///  - no override aud
///  - no extra field
/// https://github.com/aptos-labs/devnet-groth16-keys/commit/02e5675f46ce97f8b61a4638e7a0aaeaa4351f76
pub(crate) static SAMPLE_PROOF_NO_EXTRA_FIELD: Lazy<Groth16Proof> = Lazy::new(|| {
    Groth16Proof::new(
        G1Bytes::new_from_vec(hex::decode("62b167d43c33169b96c018deaa4efdc5223e095016dc6ee2cd1ad1e61755d8a5").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("2dd8fcc64014ab9f877c7ba03618cf59c49c69204b981be297dc835f60d4f923192e0f894ad947bd4aee96fd19305eb79ff359e8c341a17e8f6ba470be655e12").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("5e6e3e076e632f1440327758c634a311d83244ad7d32edcdf66458d7e0ed5619").unwrap()).unwrap(),
    )
});

/// A new Groth16 VK to test the VK rotation.
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
            "15739617451905904008434505563810388078669603068902989994513586227673794325099",
            "21857380320483623058628157959587768917537193338055331958890662600728443003915",
        ],
        [
            "19098250091710633666997475602144489052978746302163092635335135789683361496958",
            "5464980335669797405967071507706948120862078317539655982950789440091501244210",
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        g1_projective_str_to_affine(
            "19759886250806183187785579505109257837989251596255610913102572077808842056375",
            "8515569072948108462120402914801299810016610043704833841603450087200707784492",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "18250059095913215666541561118844673017538035392793529003420365565251085504261",
            "21846936675713878002567053788450833465715833259428778772043736890983365407823",
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
        G1Bytes::new_from_vec(hex::decode("f8c6b4182fcb28be5e1392297e86e03ed97c0166fcda3861cdb2b17a77778006").unwrap()).unwrap(),
        G2Bytes::new_from_vec(hex::decode("0264b7e4bb0ab8eecbed406f02d11f6b0c22a055aa9918a84a81bcf93a5a1324be81a8098c44127eab5cc4fb9cf06d58e1562d69d3b43686d82a1886fd41bf15").unwrap()).unwrap(),
        G1Bytes::new_from_vec(hex::decode("58c3e6c6ad0fa09123e4c415b3759b8b61d9ffebf90119b7592a5dc707016299").unwrap()).unwrap(),
    )
    // println!("SAMPLE_PROOF_FOR_UPGRADED_VK: {}", &proof.hash());
});
