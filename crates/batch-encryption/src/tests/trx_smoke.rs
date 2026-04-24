// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::trx::TRX, shared::key_derivation::BIBEDecryptionKeyShare,
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{seq::SliceRandom, thread_rng, CryptoRng, Rng as _, RngCore};

fn smoke_with_setup<R: RngCore + CryptoRng>(
    rng: &mut R,
    tc: <TRX as BatchThresholdEncryption>::ThresholdConfig,
    ek: <TRX as BatchThresholdEncryption>::EncryptionKey,
    dk: <TRX as BatchThresholdEncryption>::DigestKey,
    vks: Vec<<TRX as BatchThresholdEncryption>::VerificationKey>,
    msk_shares: Vec<<TRX as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = TRX::encrypt(&ek, rng, &plaintext, &associated_data).unwrap();
    TRX::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = TRX::digest(&dk, std::slice::from_ref(&ct), 0).unwrap();
    let pfs = TRX::eval_proofs_compute_all(&pfs_promise, &dk);

    let dk_shares: Vec<<TRX as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
        .into_iter()
        .map(|msk_share| msk_share.derive_decryption_key_share(&d).unwrap())
        .collect();

    dk_shares
        .iter()
        .zip(&vks)
        .map(|(dk_share, vk)| TRX::verify_decryption_key_share(vk, &d, dk_share))
        .collect::<Result<Vec<()>>>()
        .unwrap();

    let dk = TRX::reconstruct_decryption_key(
        &dk_shares
            .choose_multiple(rng, tc.t)
            .cloned()
            .collect::<Vec<BIBEDecryptionKeyShare>>(),
        &tc,
    )
    .unwrap();

    ek.verify_decryption_key(&d, &dk).unwrap();

    let decrypted_plaintext: String = TRX::decrypt(&dk, &ct.prepare(&d, &pfs).unwrap()).unwrap();

    assert_eq!(decrypted_plaintext, plaintext);

    // Test decryption verification
    let eval_proof = TRX::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        TRX::decrypt_slow(&dk, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}

#[test]
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(3, 8);

    let (ek, dk, vks, msk_shares) = TRX::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    smoke_with_setup(&mut rng, tc, ek, dk, vks, msk_shares);
}

#[test]
fn trx_serialize_deserialize_setup() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(3, 8);

    let setup = TRX::setup_for_testing(rng.r#gen(), 8, 2, &tc).unwrap();

    let bytes = bcs::to_bytes(&setup).unwrap();
    let setup2: (
        <TRX as BatchThresholdEncryption>::EncryptionKey,
        <TRX as BatchThresholdEncryption>::DigestKey,
        Vec<<TRX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<TRX as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) = bcs::from_bytes(&bytes).unwrap();

    assert_eq!(setup, setup2);

    let json = serde_json::to_string(&setup).unwrap();
    let setup2: (
        <TRX as BatchThresholdEncryption>::EncryptionKey,
        <TRX as BatchThresholdEncryption>::DigestKey,
        Vec<<TRX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<TRX as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) = serde_json::from_str(&json).unwrap();
    assert_eq!(setup, setup2);
}
