// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::runner::run_ts;
use crate::{
    group::{Fq, Fr, G1Affine, G2Affine},
    shared::symmetric::{hash_g2_element, hmac_kdf, OneTimePad, SymmetricCiphertext, SymmetricKey},
};
use ark_ff::{
    field_hashers::{DefaultFieldHasher, HashToField},
    BigInteger, PrimeField, UniformRand as _,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize as _, Compress, Validate};
use ark_std::rand::thread_rng;
use rand::RngCore;
use sha2::Sha256;

#[test]
fn test_hmac_kdf() {
    for i in [1, 2, 7, 8, 31, 32, 33, 63, 64, 65] {
        let mut input = vec![0u8; i];
        rand::thread_rng().fill_bytes(&mut input);
        let ts_result = run_ts("hmac_kdf", &input).unwrap();
        let rust_result = hmac_kdf(&input);
        println!("{:?}", ts_result);
        println!("{:?}", rust_result);
        assert_eq!(ts_result, rust_result.to_vec());
    }
}

#[test]
fn test_hash_to_fr() {
    for i in [1, 2, 7, 8, 31, 32, 33, 63, 64, 65] {
        let mut input = vec![0u8; i];
        rand::thread_rng().fill_bytes(&mut input);
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fr>>::new(&[]);
        let [fr]: [Fr; 1] = field_hasher.hash_to_field::<1>(&input);
        let rust_result = fr.into_bigint().to_bytes_le();
        let ts_result = run_ts("hash_to_fr", &input).unwrap();
        println!("{:?}", ts_result);
        println!("{:?}", rust_result);
        assert_eq!(ts_result, rust_result.to_vec());
    }
}

#[test]
fn test_hash_to_fq() {
    for i in [1, 2, 7, 8, 31, 32, 33, 63, 64, 65] {
        let mut input = vec![0u8; i];
        rand::thread_rng().fill_bytes(&mut input);
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fq>>::new(&[]);
        let [fq]: [Fq; 1] = field_hasher.hash_to_field::<1>(&input);
        let rust_result = fq.into_bigint().to_bytes_le();
        let ts_result = run_ts("hash_to_fq", &input).unwrap();
        println!("{:?}", ts_result);
        println!("{:?}", rust_result);
        assert_eq!(ts_result, rust_result.to_vec());
    }
}

#[test]
fn test_symmetric_key_serialize() {
    let mut input = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut input);
    let rust_result = bcs::to_bytes(&SymmetricKey::from_bytes(input)).unwrap();
    let ts_result = run_ts("symmetric_key_serialize", &input).unwrap();
    println!("{:?}", ts_result);
    println!("{:?}", rust_result);
    assert_eq!(ts_result, rust_result);
}

#[test]
fn test_symmetric_encrypt() {
    let mut input = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut input);
    let symmetric_key_rust = SymmetricKey::from_bytes(input);
    let ct_from_ts: SymmetricCiphertext =
        bcs::from_bytes(&run_ts("symmetric_encrypt", &input).unwrap()).unwrap();
    let result: String = symmetric_key_rust.decrypt(&ct_from_ts).unwrap();

    assert_eq!(result, String::from("hi"));
}

#[test]
fn test_otp_generation() {
    let mut input = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut input);
    let rust_result = bcs::to_bytes(&OneTimePad::from_source_bytes(input)).unwrap();
    let ts_result = run_ts("otp_generation", &input).unwrap();
    println!("{:?}", ts_result);
    println!("{:?}", rust_result);
    assert_eq!(ts_result, rust_result);
}

#[test]
fn test_otp_padding() {
    let mut key_bytes = [0u8; 16];
    let mut otp_bytes = [0u8; 64];
    rand::thread_rng().fill_bytes(&mut key_bytes);
    rand::thread_rng().fill_bytes(&mut otp_bytes);
    let rust_key = SymmetricKey::from_bytes(key_bytes);
    let rust_otp = OneTimePad::from_source_bytes(otp_bytes);
    let rust_result = bcs::to_bytes(&rust_otp.pad_key(&rust_key)).unwrap();
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(&key_bytes);
    input.extend_from_slice(&otp_bytes);
    let ts_result = run_ts("otp_padding", &input).unwrap();
    println!("{:?}", ts_result);
    println!("{:?}", rust_result);
    assert_eq!(ts_result, rust_result);
}

#[test]
fn test_hash_g2_element() {
    let mut rng = thread_rng();
    let g2 = G2Affine::rand(&mut rng);
    let rust_result = hash_g2_element(g2).unwrap();
    let mut input = vec![];
    g2.serialize_with_mode(&mut input, Compress::Yes).unwrap();
    let ts_result_bytes = run_ts("hash_g2_element", &input).unwrap();
    let ts_result =
        G1Affine::deserialize_with_mode(ts_result_bytes.as_slice(), Compress::Yes, Validate::Yes)
            .unwrap();

    assert_eq!(rust_result, ts_result);
}
