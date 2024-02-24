// Copyright © Aptos Foundation

use ark_ec::CurveGroup;
use aptos_oidb_pepper_common::{vuf, vuf::VUF, VUFVerificationKey};
use once_cell::sync::Lazy;
use aptos_oidb_pepper_common::vuf::scheme0::Scheme0;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

pub struct VufScheme0Sk {
    pub sk_bytes: Vec<u8>,
}

pub static VUF_SCHEME0_SK: Lazy<ark_bls12_381::Fr> = Lazy::new(|| {
    let vuf_key_hex =
        std::env::var("VRF_KEY_HEX").expect("VRF_KEY_HEX is required for pepper calculation");
    let sk_bytes = hex::decode(vuf_key_hex).expect("vrf_key_hex should be a valid hex string");
    ark_bls12_381::Fr::deserialize_compressed(sk_bytes.as_slice()).unwrap()
});

pub static VUF_VERIFICATION_KEY_JSON: Lazy<String> = Lazy::new(|| {
    let pk = Scheme0::pk_from_sk(&VUF_SCHEME0_SK).expect("bad sk");
    let mut buf = vec![];
    pk.into_affine().serialize_compressed(&mut buf).unwrap();
    let obj = VUFVerificationKey {
        scheme_name: Scheme0::scheme_name(),
        payload_hexlified: hex::encode(buf),
    };
    serde_json::to_string_pretty(&obj).unwrap()
});
