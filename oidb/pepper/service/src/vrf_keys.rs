// Copyright © Aptos Foundation

use aptos_oidb_pepper_common::{
    vrf::{scheme0::Scheme0, VRF},
    VRFVerificationKey,
};
use ark_ec::CurveGroup;
use ark_serialize::CanonicalSerialize;
use once_cell::sync::Lazy;
use ark_ff::PrimeField;

pub struct VrfScheme0Sk {
    pub sk_bytes: Vec<u8>,
}

pub static VRF_SCHEME0_SK: Lazy<ark_bls12_381::Fr> = Lazy::new(|| {
    let vrf_key_hex =
        std::env::var("VRF_KEY_HEX").expect("VRF_KEY_HEX is required for pepper calculation");
    let sk_bytes = hex::decode(vrf_key_hex).expect("vrf_key_hex should be a valid hex string");
    ark_bls12_381::Fr::from_be_bytes_mod_order(sk_bytes.as_slice())
});

pub static VRF_VERIFICATION_KEY_JSON: Lazy<String> = Lazy::new(|| {
    let pk = Scheme0::pk_from_sk(&VRF_SCHEME0_SK).expect("bad sk");
    let mut buf = vec![];
    pk.into_affine().serialize_compressed(&mut buf).unwrap();
    let obj = VRFVerificationKey {
        scheme_name: Scheme0::scheme_name(),
        vrf_public_key_hex_string: hex::encode(buf),
    };
    serde_json::to_string_pretty(&obj).unwrap()
});
