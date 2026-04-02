// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::traits::{BatchThresholdEncryption, DecryptionKeyShare};
use anyhow::Result;
use aptos_crypto::TSecretSharingConfig;

#[cfg(test)]
pub mod fptx_smoke;
#[cfg(test)]
pub mod fptx_succinct_smoke;
pub mod fptx_weighted_smoke;

pub fn run_smoke<Scheme: BatchThresholdEncryption>(
    tc: <Scheme as BatchThresholdEncryption>::ThresholdConfig,
    ek: <Scheme as BatchThresholdEncryption>::EncryptionKey,
    dk: <Scheme as BatchThresholdEncryption>::DigestKey,
    vks: Vec<<Scheme as BatchThresholdEncryption>::VerificationKey>,
    msk_shares: Vec<<Scheme as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let mut rng_arkworks = ark_std::rand::thread_rng();
    let mut rng_aptos = rand::thread_rng();

    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = Scheme::encrypt(&ek, &mut rng_arkworks, &plaintext, &associated_data).unwrap();
    Scheme::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = Scheme::digest(&dk, std::slice::from_ref(&ct), 0).unwrap();
    let pfs = Scheme::eval_proofs_compute_all(&pfs_promise, &dk);

    let dk_shares: Vec<<Scheme as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
        .into_iter()
        .map(|msk_share| {
            <Scheme as BatchThresholdEncryption>::derive_decryption_key_share(&msk_share, &d)
                .unwrap()
        })
        .collect();

    dk_shares
        .iter()
        .zip(&vks)
        .map(|(dk_share, vk)| Scheme::verify_decryption_key_share(vk, &d, dk_share))
        .collect::<Result<Vec<()>>>()
        .unwrap();

    let eligible_share_subset: Vec<<Scheme as BatchThresholdEncryption>::DecryptionKeyShare> = tc
        .get_random_eligible_subset_of_players(&mut rng_aptos)
        .into_iter()
        .map(|player| {
            dk_shares
                .iter()
                .find(|share| share.player() == player)
                .unwrap()
                .clone()
        })
        .collect();

    let dk = Scheme::reconstruct_decryption_key(&eligible_share_subset, &tc).unwrap();

    <Scheme as BatchThresholdEncryption>::verify_decryption_key(&ek, &d, &dk).unwrap();

    let decrypted_plaintext: String = Scheme::decrypt(
        &dk,
        &<Scheme as BatchThresholdEncryption>::prepare_ct(&ct, &d, &pfs).unwrap(),
    )
    .unwrap();

    assert_eq!(decrypted_plaintext, plaintext);

    // Test decryption verification
    let eval_proof = Scheme::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        Scheme::decrypt_slow(&dk, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}
