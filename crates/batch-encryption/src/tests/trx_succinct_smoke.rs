// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::trx_succinct::TRXSuccinct, shared::key_derivation::BIBEDecryptionKeyShare,
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{seq::SliceRandom, thread_rng, CryptoRng, Rng as _, RngCore};

fn smoke_with_setup<R: RngCore + CryptoRng>(
    rng: &mut R,
    tc: <TRXSuccinct as BatchThresholdEncryption>::ThresholdConfig,
    ek: <TRXSuccinct as BatchThresholdEncryption>::EncryptionKey,
    dk: <TRXSuccinct as BatchThresholdEncryption>::DigestKey,
    vks: Vec<<TRXSuccinct as BatchThresholdEncryption>::VerificationKey>,
    msk_shares: Vec<<TRXSuccinct as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = TRXSuccinct::encrypt(&ek, rng, &plaintext, &associated_data).unwrap();
    TRXSuccinct::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = TRXSuccinct::digest(&dk, std::slice::from_ref(&ct), 0).unwrap();
    let pfs = TRXSuccinct::eval_proofs_compute_all(&pfs_promise, &dk);

    let dk_shares: Vec<<TRXSuccinct as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
        .into_iter()
        .map(|msk_share| msk_share.derive_decryption_key_share(&d).unwrap())
        .collect();

    dk_shares
        .iter()
        .zip(&vks)
        .map(|(dk_share, vk)| TRXSuccinct::verify_decryption_key_share(vk, &d, dk_share))
        .collect::<Result<Vec<()>>>()
        .unwrap();

    let dk = TRXSuccinct::reconstruct_decryption_key(
        &dk_shares
            .choose_multiple(rng, tc.t)
            .cloned()
            .collect::<Vec<BIBEDecryptionKeyShare>>(),
        &tc,
    )
    .unwrap();

    ek.verify_decryption_key(&d, &dk).unwrap();

    let decrypted_plaintext: String =
        TRXSuccinct::decrypt(&dk, &ct.prepare(&d, &pfs).unwrap()).unwrap();

    assert_eq!(decrypted_plaintext, plaintext);

    // Test individual decryption
    let eval_proof = TRXSuccinct::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        TRXSuccinct::decrypt_slow(&dk, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}

#[test]
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(5, 8);

    let (ek, dk, vks, msk_shares) =
        TRXSuccinct::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    smoke_with_setup(&mut rng, tc, ek, dk, vks, msk_shares);
}
