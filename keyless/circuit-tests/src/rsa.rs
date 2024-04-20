use num_traits::FromPrimitive;
use rand::thread_rng;
use rsa::RsaPrivateKey;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::signature::{Keypair, RandomizedSigner, SignatureEncoding, Verifier};
use rsa::sha2::{Digest, Sha256};
use rsa::traits::{PublicKeyParts};
use num_bigint::BigUint;

use aptos_keyless_common::input_processing::circuit_input_signals::CircuitInputSignals;
use aptos_keyless_common::input_processing::config::CircuitPaddingConfig;
use aptos_keyless_common::input_processing::encoding::As64BitLimbs;
use crate::TestCircuitHandle;
use std::convert::TryInto;
use num_modular::*;

const EXPONENT : u64 = 65537;


// to debug, I'm trying to verify the signature manually wrt the hash limbs
fn verify_sig(modulus: &BigUint, signature: &BigUint, expected_hash_limbs: &[u64]) {
    let exponent_bigint = BigUint::from_u64(EXPONENT).unwrap();
    let computed_hash_bigint = signature.powm(&exponent_bigint, modulus);
    let computed_hash_limbs = computed_hash_bigint.to_u64_digits();

    println!("{:?}", expected_hash_limbs);
    println!("{:?}", computed_hash_limbs);

    assert_eq!(expected_hash_limbs, computed_hash_limbs);
}


#[test]
fn test_rsa_verify() {
    println!("Compiling circuit");
    let circuit = TestCircuitHandle::new("rsa_verify_test.circom").unwrap();
    println!("Computing input signals");
    let mut rng = rsa::rand_core::OsRng;
    let e = rsa::BigUint::from_u64(EXPONENT).unwrap();
    let private_key = RsaPrivateKey::new_with_exp(&mut rng, 2048, &e).unwrap();
    let modulus = BigUint::from_bytes_be(&private_key.to_public_key().n().to_bytes_be());
    let modulus_limbs = modulus.to_u64_digits();
    let signing_key = SigningKey::<Sha256>::new(private_key);

    let message = b"Hello, RSA signing!";

    let mut hasher = Sha256::new();
    hasher.update(message);
    
    let hashed_msg = hasher.finalize().to_vec();

    let hashed_msg_limbs : Vec<u64> = 
        hashed_msg.chunks(8)
                  .map(|bytes| bytes.try_into())
                  .collect::<Result<Vec<[u8; 8]>, _>>()
                  .unwrap()
                  .into_iter()
                  .map(u64::from_be_bytes)
                  .collect();


    let signature = BigUint::from_bytes_be(&signing_key.sign_with_rng(&mut rng, &hashed_msg).to_bytes());
    let signature_limbs = signature.to_u64_digits();

    verify_sig(&modulus, &signature, &hashed_msg_limbs);

    let config = CircuitPaddingConfig::new();

    let circuit_input_signals = CircuitInputSignals::new()
        .limbs_input("sign", signature_limbs.as_slice())
        .limbs_input("modulus", modulus_limbs.as_slice())
        .limbs_input("hashed", hashed_msg_limbs.as_slice())
        .pad(&config)
        .unwrap();
    println!("{:?}", circuit_input_signals);
    println!("Generating witness");
    let result = circuit.gen_witness(circuit_input_signals);
    println!("Done.");
    println!("{:?}", result);
    assert!(result.is_ok());
}
