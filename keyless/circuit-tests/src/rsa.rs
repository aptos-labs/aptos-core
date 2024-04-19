use num_traits::FromPrimitive;
use rand::thread_rng;
use rsa::{BigUint, Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use rsa::traits::{PublicKeyParts};
use sha2::{Digest, Sha256};
use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use crate::TestCircuitHandle;
use rsa::pkcs1v15::SigningKey;

#[test]
fn test_rsa_verify() {
    let circuit = TestCircuitHandle::new("rsa_verify_test.circom").unwrap();
    let mut rng = rsa::rand_core::OsRng;
    let e = BigUint::from_u32(65537).unwrap();
    let private_key = RsaPrivateKey::new_with_exp(&mut rng, 4096, &e).unwrap();
    let public_key = RsaPublicKey::from(&private_key);
    let modulus = public_key.n();
    let num_limbs = modulus.to_bytes_be().len().div_ceil(8);
    let modulus_limbs: Vec<u64> = (0..num_limbs).map(|i| modulus.get_limb(i)).collect();

    let message = b"Hello, RSA signing!";
    let mut hasher = Sha256::new();
    hasher.update(message);
    let hashed_msg = hasher.finalize();
    let hashed_msg_limbs: Vec<u64> = vec![];
    let signature = signing_key.sign_with_rng(&mut rng, &hashed_msg);
    println!("signature_len={}", signature.len());
    let signature_limbs: Vec<u64> = vec![];

    let config = CircuitPaddingConfig::new()
        .max_length("sign", 64)
        .max_length("modulus", 64)
        .max_length("hashed", 4);
    let circuit_input_signals = CircuitInputSignals::new()
        .limbs_input("sign", signature_limbs.as_slice())
        .limbs_input("modulus", modulus_limbs.as_slice())
        .limbs_input("hashed", hashed_msg_limbs.as_slice())
        .pad(&config)
        .unwrap();
    let result = circuit.gen_witness(circuit_input_signals);
    assert!(result.is_ok());
}