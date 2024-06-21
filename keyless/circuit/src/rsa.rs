// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::TestCircuitHandle;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::CircuitInputSignals, config::CircuitPaddingConfig,
};
use aptos_logger::info;
use num_bigint::BigUint;
use num_traits::FromPrimitive;
use rand::{thread_rng, Rng};
use rsa::{
    pkcs1v15::SigningKey,
    sha2::{Digest, Sha256},
    signature::{RandomizedSigner, SignatureEncoding},
    traits::{PrivateKeyParts, PublicKeyParts},
    RsaPrivateKey,
};
use std::convert::TryInto;

const EXPONENT: u64 = 65537;

#[test]
fn rsa_verify_should_pass_with_valid_input() {
    common(|_, _, _| {}, true)
}

#[test]
fn rsa_verify_should_fail_with_invalid_signature() {
    common(
        |sig_limbs, _, _| {
            flip_random_bit(sig_limbs);
        },
        false,
    );
}

#[test]
fn rsa_verify_should_fail_with_invalid_modulus() {
    common(
        |_, modulus_limbs, _| {
            flip_random_bit(modulus_limbs);
        },
        false,
    );
}

#[test]
fn rsa_verify_should_fail_with_invalid_hased_msg() {
    common(
        |_, _, hashed_msg_limbs| {
            flip_random_bit(hashed_msg_limbs);
        },
        false,
    );
}

fn common<F: Fn(&mut Vec<u64>, &mut Vec<u64>, &mut Vec<u64>)>(
    update_signals: F,
    witness_gen_should_pass: bool,
) {
    // Default #iterations to 1 but allow customization.
    let num_iterations = std::env::var("NUM_ITERATIONS")
        .unwrap_or("1".to_string())
        .parse::<usize>()
        .unwrap_or(1);

    let circuit = TestCircuitHandle::new("rsa_verify_test.circom").unwrap();
    let mut rng = thread_rng();
    let e = rsa::BigUint::from_u64(EXPONENT).unwrap();

    for i in 0..num_iterations {
        info!("Iteration {i} starts. Generate RSA key pairs.");
        let private_key =
            RsaPrivateKey::new_with_exp(&mut rand_chacha::rand_core::OsRng, 2048, &e).unwrap();
        info!(
            "Key pair generated, d={:?}, n={:?}.",
            private_key.d(),
            private_key.n()
        );
        let modulus = BigUint::from_bytes_be(&private_key.to_public_key().n().to_bytes_be());
        let mut modulus_limbs = modulus.to_u64_digits();
        let signing_key = SigningKey::<Sha256>::new(private_key);

        info!("Generate a random message.");
        let msg_len: usize = rng.gen_range(0, 9999);
        let message: Vec<u8> = vec![0; msg_len];

        info!("Message generated, msg_hex={}", hex::encode(&message));
        let mut hasher = Sha256::new();
        hasher.update(&message);
        let hashed_msg = hasher.finalize().to_vec();

        let mut hashed_msg_limbs: Vec<u64> = hashed_msg
            .chunks(8)
            .map(|bytes| bytes.try_into())
            .collect::<Result<Vec<[u8; 8]>, _>>()
            .unwrap()
            .into_iter()
            .map(u64::from_be_bytes)
            .rev()
            .collect();

        let signature = BigUint::from_bytes_be(
            &signing_key
                .sign_with_rng(&mut rand_chacha::rand_core::OsRng, message.as_slice())
                .to_bytes(),
        );
        let mut signature_limbs = signature.to_u64_digits();

        let config = CircuitPaddingConfig::new();

        update_signals(
            &mut signature_limbs,
            &mut modulus_limbs,
            &mut hashed_msg_limbs,
        );

        let circuit_input_signals = CircuitInputSignals::new()
            .limbs_input("sign", signature_limbs.as_slice())
            .limbs_input("modulus", modulus_limbs.as_slice())
            .limbs_input("hashed", hashed_msg_limbs.as_slice())
            .pad(&config)
            .unwrap();
        let result = circuit.gen_witness(circuit_input_signals);
        assert_eq!(witness_gen_should_pass, result.is_ok());
    }
}

fn flip_random_bit(limbs: &mut [u64]) {
    let mut rng = thread_rng();
    let limb_idx = rng.gen_range(0, limbs.len());
    let bit_idx = rng.gen_range(0, 64);
    *limbs.get_mut(limb_idx).unwrap() ^= 1_u64 << bit_idx;
}
