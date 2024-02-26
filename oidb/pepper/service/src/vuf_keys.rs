// Copyright © Aptos Foundation

use aptos_oidb_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF},
    VUFVerificationKey,
};
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use once_cell::sync::Lazy;
use ark_ff::PrimeField;

pub struct VufScheme0Sk {
    pub sk_bytes: Vec<u8>,
}

pub static VUF_SK: Lazy<ark_bls12_381::Fr> = Lazy::new(|| {
    let vuf_key_hex =
        std::env::var("VUF_KEY_HEX").expect("VUF_KEY_HEX is required for pepper calculation");
    let sk_bytes = hex::decode(vuf_key_hex).expect("vuf_key_hex should be a valid hex string");
    ark_bls12_381::Fr::from_be_bytes_mod_order(sk_bytes.as_slice())
});

pub static VUF_VERIFICATION_KEY_JSON: Lazy<String> = Lazy::new(|| {
    let pk = Bls12381G1Bls::pk_from_sk(&VUF_SK).expect("bad sk");
    let mut buf = vec![];
    pk.into_affine().serialize_compressed(&mut buf).unwrap();
    let obj = VUFVerificationKey {
        scheme_name: Bls12381G1Bls::scheme_name(),
        vuf_public_key_hex_string: hex::encode(buf),
    };
    serde_json::to_string_pretty(&obj).unwrap()
});
