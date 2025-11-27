
use std::any::type_name;

use ark_std::rand::Rng as _;
use ark_std::{rand::thread_rng, Zero, One};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use serde::{Deserialize, Serialize};

use crate::schemes::fptx::{EncryptionKey, FPTX};
use crate::shared::algebra::shamir::ThresholdConfig;
use crate::shared::ciphertext::{BIBECTDecrypt as _, BIBECTEncrypt, Ciphertext, CTDecrypt};
use crate::shared::ids::{FreeRootIdSet, IdSet as _};
use crate::shared::key_derivation::BIBEDecryptionKey;
use crate::shared::{ciphertext::BIBECiphertext, ids::FreeRootId};
use crate::group::Fr;
use crate::traits::BatchThresholdEncryption as _;

use super::runner::run_ts;

#[test]
fn test_bibe_ciphertext_serialization() {
    let ct: BIBECiphertext<FreeRootId> = BIBECiphertext::blank_for_testing();
    let input = bcs::to_bytes(&ct).unwrap();

    let ts_result = run_ts("bibe_ciphertext_serialization", &input).unwrap();

    let ct_deserialized : BIBECiphertext<FreeRootId> = bcs::from_bytes(&ts_result).unwrap();

    assert_eq!(ct_deserialized, ct);

}

#[test]
fn test_dummy() {
    let ek = EncryptionKey::new_for_testing();
    let mut rng = thread_rng();
    let bibe_ct = ek.bibe_encrypt(&mut rng, &String::from("hi"), FreeRootId::new(Fr::one()));
}

#[test]
fn test_bibe_ct_encrypt_decrypt_ts() {
    let mut rng = thread_rng();
    let tc = ThresholdConfig::new(1, 1);
    let (ek, dk, _, msk_shares, _, _) =
    FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

    let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
    let mut counter = Fr::zero();

    for _ in 0..dk.capacity() {
        ids.add(&FreeRootId::new(counter));
        counter += Fr::one();
    }

    ids.compute_poly_coeffs();
    let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
    let pfs = pfs.compute_all(&dk);

    let plaintext = String::from("hi");

    let id = FreeRootId::new(Fr::one());

    let ek_bytes = bcs::to_bytes(&ek).unwrap();
    let ct_bytes = run_ts("bibe_ciphertext_encrypt", &ek_bytes).unwrap();
    let ct : BIBECiphertext<FreeRootId> = bcs::from_bytes(&ct_bytes).unwrap();


    let dk = BIBEDecryptionKey::reconstruct(
        &vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()],
        &tc,
    )
        .unwrap();

    let decrypted_plaintext: String = dk
        .bibe_decrypt(&ct.prepare(&digest, &pfs).unwrap())
        .unwrap();

    assert_eq!(decrypted_plaintext, plaintext);
}

#[allow(non_snake_case)]
#[test]
fn test_ed25519() {
    #[derive(Serialize, Deserialize)]
    struct TestEd25519 {
        secretKey: SigningKey,
        publicKey: VerifyingKey,
        msg: Vec<u8>,
        signature: Signature,
    }

    let mut rng = thread_rng();
    let secretKey: SigningKey = SigningKey::generate(&mut rng);
    let publicKey = secretKey.verifying_key();

    let msg = vec![0u8, 1u8, 2u8];
    let signature = secretKey.sign(&msg);
    println!("{:?}", bcs::to_bytes(&secretKey).unwrap().len());
    println!("{:?}", bcs::to_bytes(&publicKey).unwrap().len());
    println!("{:?}", bcs::to_bytes(&signature).unwrap().len());
    
    let input = bcs::to_bytes(&TestEd25519 {
        secretKey,
        publicKey,
        msg: msg.clone(),
        signature,
    }).unwrap();
    println!("{:?}", input.len());
    println!("{:?}", input);


    let ts_signature_bytes = run_ts("ed25519", &input).unwrap();
    let ts_signature : Signature = bcs::from_bytes(&ts_signature_bytes).unwrap();

    publicKey.verify(&msg, &ts_signature).unwrap();


}

#[test]
fn test_ct_verify_ts() {
    let mut rng = thread_rng();
    let tc = ThresholdConfig::new(1, 1);
    let (ek, _, _,  _, _, _) =
    FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

    let associated_data = String::from("associated data");
    let ek_bytes = bcs::to_bytes(&ek).unwrap();
    let ct_bytes = run_ts("ciphertext_encrypt", &ek_bytes).unwrap();
    let ct : Ciphertext<FreeRootId> = bcs::from_bytes(&ct_bytes).unwrap();

    // Verification with the correct associated data should succeed.
    ct.verify(&associated_data).unwrap();
}

#[test]
fn test_ct_encrypt_decrypt_ts() {
    let mut rng = thread_rng();
    let tc = ThresholdConfig::new(1, 1);
    let (ek, dk, _, msk_shares, _, _) =
    FPTX::setup_for_testing(rng.gen(), 8, 1, &tc, &tc).unwrap();

    let plaintext = String::from("hi");

    let ek_bytes = bcs::to_bytes(&ek).unwrap();
    let ct_bytes = run_ts("ciphertext_encrypt", &ek_bytes).unwrap();
    let ct : Ciphertext<FreeRootId> = bcs::from_bytes(&ct_bytes).unwrap();

    let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
    ids.add(&ct.id());

    ids.compute_poly_coeffs();
    let (digest, pfs) = dk.digest(&mut ids, 0).unwrap();
    let pfs = pfs.compute_all(&dk);

    let dk = BIBEDecryptionKey::reconstruct(
        &vec![msk_shares[0].derive_decryption_key_share(&digest).unwrap()],
        &tc,
    )
        .unwrap();

    let decrypted_plaintext: String = dk.decrypt(&ct.prepare(&digest, &pfs).unwrap()).unwrap();

    assert_eq!(decrypted_plaintext, plaintext);
}
