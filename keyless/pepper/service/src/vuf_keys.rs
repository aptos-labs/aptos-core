// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure};
use aptos_keyless_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF},
    PepperV0VufPubKey,
};
use aptos_logger::warn;
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use once_cell::sync::Lazy;
use sha3::Digest;

/// Derive the VUF private key from a seed given in the environment.
fn derive_sk_from_env_seed() -> anyhow::Result<ark_bls12_381::Fr> {
    let seed_hexlified = std::env::var("VUF_KEY_SEED_HEX")
        .map_err(|e| anyhow!("error while reading envvar `VUF_KEY_SEED_HEX`: {e}"))?;
    let seed =
        hex::decode(seed_hexlified).map_err(|e| anyhow!("seed unhexlification error: {e}"))?;
    ensure!(seed.len() >= 32, "seed entropy should be at least 32 bytes");
    let mut hasher = sha3::Sha3_512::new();
    hasher.update(seed);
    let sk = ark_bls12_381::Fr::from_be_bytes_mod_order(hasher.finalize().as_slice());
    Ok(sk)
}

/// A backward-compatible path to load a sk (serialized and hexlified) from envvar.
/// TODO: once secret seed is stable, remove this.
fn deserialize_sk_from_env() -> anyhow::Result<ark_bls12_381::Fr> {
    let vuf_key_hex = std::env::var("VUF_KEY_HEX")
        .map_err(|e| anyhow!("error while reading envvar `VUF_KEY_HEX`: {e}"))?;
    let mut sk_bytes =
        hex::decode(vuf_key_hex).map_err(|e| anyhow!("sk unhexlification error: {e}"))?;
    sk_bytes.reverse();
    let sk = ark_bls12_381::Fr::deserialize_compressed(sk_bytes.as_slice())
        .map_err(|e| anyhow!("Fr deserialization error: {e}"))?;
    Ok(sk)
}

pub static VUF_SK: Lazy<ark_bls12_381::Fr> = Lazy::new(|| {
    match derive_sk_from_env_seed() {
        Ok(sk) => {
            return sk;
        },
        Err(e) => {
            warn!("`derive_sk_from_env_seed`failed: {e}");
            warn!("falling back to `deserialize_sk_from_env`");
            //TODO: once secret seed is stable, remove the fallback path.
        },
    }

    deserialize_sk_from_env().expect("fallback sk also failed")
});

pub static PEPPER_VUF_VERIFICATION_KEY_JSON: Lazy<String> = Lazy::new(|| {
    let pk = Bls12381G1Bls::pk_from_sk(&VUF_SK).expect("bad sk");
    let mut buf = vec![];
    pk.into_affine().serialize_compressed(&mut buf).unwrap();
    let obj = PepperV0VufPubKey { public_key: buf };
    serde_json::to_string_pretty(&obj).unwrap()
});
