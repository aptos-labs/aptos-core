// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_keyless_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, slip_10::ed25519_dalek::Digest, VUF},
    PepperV0VufPubKey,
};
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use std::sync::Arc;

/// Generates a random VUF public and private keypair for testing purposes
pub fn create_vuf_public_private_keypair() -> Arc<(String, ark_bls12_381::Fr)> {
    // Generate a random VUF seed
    let private_key_seed = rand::random::<[u8; 32]>();

    // Derive the VUF private key from the seed
    let mut sha3_hasher = sha3::Sha3_512::new();
    sha3_hasher.update(private_key_seed);
    let vuf_private_key =
        ark_bls12_381::Fr::from_be_bytes_mod_order(sha3_hasher.finalize().as_slice());

    // Derive the VUF public key from the private key
    let vuf_public_key = Bls12381G1Bls::pk_from_sk(&vuf_private_key).unwrap();

    // Create the pepper public key object
    let mut public_key_buf = vec![];
    vuf_public_key
        .into_affine()
        .serialize_compressed(&mut public_key_buf)
        .unwrap();
    let pepper_vuf_public_key = PepperV0VufPubKey::new(public_key_buf);

    // Transform the public key object to a pretty JSON string
    let vuf_public_key_string = serde_json::to_string_pretty(&pepper_vuf_public_key).unwrap();

    Arc::new((vuf_public_key_string, vuf_private_key))
}
