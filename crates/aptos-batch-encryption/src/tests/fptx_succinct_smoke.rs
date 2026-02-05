// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::fptx_succinct::FPTXSuccinct, shared::key_derivation::BIBEDecryptionKeyShare,
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use aptos_crypto::arkworks::shamir::ShamirThresholdConfig;
use ark_std::rand::{seq::SliceRandom, thread_rng, CryptoRng, Rng as _, RngCore};

fn smoke_with_setup<R: RngCore + CryptoRng>(
    rng: &mut R,
    tc: <FPTXSuccinct as BatchThresholdEncryption>::ThresholdConfig,
    ek: <FPTXSuccinct as BatchThresholdEncryption>::EncryptionKey,
    dk: <FPTXSuccinct as BatchThresholdEncryption>::DigestKey,
    vks: Vec<<FPTXSuccinct as BatchThresholdEncryption>::VerificationKey>,
    msk_shares: Vec<<FPTXSuccinct as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = FPTXSuccinct::encrypt(&ek, rng, &plaintext, &associated_data).unwrap();
    FPTXSuccinct::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = FPTXSuccinct::digest(&dk, std::slice::from_ref(&ct), 0).unwrap();
    let pfs = FPTXSuccinct::eval_proofs_compute_all(&pfs_promise, &dk);

    let dk_shares: Vec<<FPTXSuccinct as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
        .into_iter()
        .map(|msk_share| msk_share.derive_decryption_key_share(&d).unwrap())
        .collect();

    dk_shares
        .iter()
        .zip(&vks)
        .map(|(dk_share, vk)| FPTXSuccinct::verify_decryption_key_share(vk, &d, dk_share))
        .collect::<Result<Vec<()>>>()
        .unwrap();

    let dk = FPTXSuccinct::reconstruct_decryption_key(
        &dk_shares
            .choose_multiple(rng, tc.t)
            .cloned()
            .collect::<Vec<BIBEDecryptionKeyShare>>(),
        &tc,
    )
    .unwrap();

    ek.verify_decryption_key(&d, &dk).unwrap();

    let decrypted_plaintexts: Vec<String> =
        FPTXSuccinct::decrypt(&dk, &[ct.prepare(&d, &pfs).unwrap()]).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    // Test individual decryption
    let eval_proof = FPTXSuccinct::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        FPTXSuccinct::decrypt_individual(&dk, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}

#[test]
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = ShamirThresholdConfig::new(5, 8);

    let (ek, dk, vks, msk_shares) =
        FPTXSuccinct::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    smoke_with_setup(&mut rng, tc, ek, dk, vks, msk_shares);
}
