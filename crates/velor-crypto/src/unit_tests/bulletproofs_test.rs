// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::bulletproofs::MAX_RANGE_BITS;
use bulletproofs::{BulletproofGens, PedersenGens, RangeProof};
use curve25519_dalek_ng::{ristretto::CompressedRistretto, scalar::Scalar};
use merlin::Transcript;
use rand::{thread_rng, Rng};
use std::convert::TryFrom;

const TEST_DOMAIN_SEPARATION_TAG: &[u8; 21] = b"VelorTestBulletproofs";

#[test]
#[ignore]
pub fn test_generated_bulletproof_verifies() {
    let mut rng = thread_rng();

    let pg = PedersenGens::default();
    let bg = BulletproofGens::new(64, 1);

    // Pick a random value and a Pedersen commitment blinder for it
    let value = rng.gen_range(0u64, (2u128.pow(MAX_RANGE_BITS as u32) - 1u128) as u64);
    let value_scalar = Scalar::from(value);
    let blinder = Scalar::hash_from_bytes::<sha3::Sha3_512>(b"some random blinder");

    // Compute a range proof for 'value' committed with commitment key `pg` and `blinder`
    let mut t_prv = Transcript::new(TEST_DOMAIN_SEPARATION_TAG);
    let range_proof =
        RangeProof::prove_single(&bg, &pg, &mut t_prv, value, &blinder, MAX_RANGE_BITS);

    assert!(range_proof.is_ok());

    let (range_proof, comm_expected) = range_proof.unwrap();

    // Make sure the proof passes verification
    let comm = pg.commit(value_scalar, blinder);
    assert!(comm.eq(&comm_expected.decompress().unwrap()));

    let mut t_ver = Transcript::new(TEST_DOMAIN_SEPARATION_TAG);
    let success = range_proof.verify_single(&bg, &pg, &mut t_ver, &comm.compress(), MAX_RANGE_BITS);

    assert!(success.is_ok());

    println!("Value: {value}");
    println!(
        "Value (as Scalar): {}",
        hex::encode(value_scalar.to_bytes())
    );
    println!("Blinder: {}", hex::encode(blinder.to_bytes()));
    println!("Commitment: {}", hex::encode(comm.compress().to_bytes()));
    println!("Range proof: {}", hex::encode(range_proof.to_bytes()));
    println!(
        "Domain: {}",
        String::from_utf8(TEST_DOMAIN_SEPARATION_TAG.to_vec()).unwrap()
    );
}

#[test]
#[ignore]
pub fn test_valid_bulletproof_verifies() {
    // value is 5020644638028926087
    let range_proof_bytes = hex::decode("d8d422d3fb9511d1942b78e3ec1a8c82fe1c01a0a690c55a4761e7e825633a753cca816667d2cbb716fe04a9c199cad748c2d4e59de4ed04fedf5f04f4341a74ae75b63c1997fd65d5fb3a8c03ad8771abe2c0a4f65d19496c11d948d6809503eac4d996f2c6be4e64ebe2df31102c96f106695bdf489dc9290c93b4d4b5411fb6298d0c33afa57e2e1948c38ef567268a661e7b1c099272e29591e717930a06a2c6e0e2d56aedea3078fd59334634f1a4543069865409eba074278f191039083102a9a0621791a9be09212a847e22061e083d7a712b05bca7274b25e4cb1201c679c4957f0842d7661fa1d3f5456a651e89112628b456026f8ad3a7abeaba3fec8031ec8b0392c0aa6c96205f7b21b0c2d6b5d064bd5bd1a1d91c41625d910688fa0dca35ec0f0e31a45792f8d6a330be970a22e1e0773111a083de893c89419ee7de97295978de90bcdf873a2826746809e64f9143417dbed09fa1c124e673febfed65c137cc45fabda963c96b64645802d1440cba5e58717e539f55f3321ab0c0f60410fba70070c5db500fee874265a343a2a59773fd150bcae09321a5166062e176e2e76bef0e3dd1a9250bcb7f4c971c10f0b24eb2a94e009b72c1fc21ee4267881e27b4edba8bed627ddf37e0c53cd425bc279d0c50d154d136503e54882e9541820d6394bd52ca2b438fd8c517f186fec0649c4846c4e43ce845d80e503dee157ce55392188039a7efc78719107ab989db8d9363b9dfc1946f01a84dbca5e742ed5f30b07ac61cf17ce2cf2c6a49d799ed3968a63a3ccb90d9a0e50960d959f17f202dd5cf0f2c375a8a702e063d339e48c0227e7cf710157f63f13136d8c3076c672ea2c1028fc1825366a145a4311de6c2cc46d3144ae3d2bc5808819b9817be3fce1664ecb60f74733e75e97ca8e567d1b81bdd4c56c7a340ba00").unwrap();
    let range_proof = RangeProof::from_bytes(range_proof_bytes.as_slice()).unwrap();

    let comm_bytes =
        hex::decode("0a665260a4e42e575882c2cdcb3d0febd6cf168834f6de1e9e61e7b2e53dbf14").unwrap();
    let compressed = CompressedRistretto(<[u8; 32]>::try_from(comm_bytes).unwrap());
    let comm = compressed.decompress().unwrap();

    let pg = PedersenGens::default();
    let bg = BulletproofGens::new(64, 1);

    // scalar is 870c2fa1b2e9ac45000000000000000000000000000000000000000000000000
    let value = 5020644638028926087u64;
    let blinder = <[u8; 32]>::try_from(
        hex::decode("e7c7b42b75503bfc7b1932783786d227ebf88f79da752b68f6b865a9c179640c").unwrap(),
    )
    .unwrap();

    assert!(pg
        .commit(
            Scalar::from(value),
            Scalar::from_canonical_bytes(blinder).unwrap()
        )
        .eq(&comm));

    let mut t_ver = Transcript::new(TEST_DOMAIN_SEPARATION_TAG);
    let success = range_proof.verify_single(&bg, &pg, &mut t_ver, &comm.compress(), MAX_RANGE_BITS);

    assert!(success.is_ok());
}

#[test]
#[ignore]
fn print_rand_base() {
    println!(
        "Default PedersenGens's blinding factor hex: {}",
        hex::encode(PedersenGens::default().B_blinding.compress().to_bytes())
    );
}
