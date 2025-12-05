// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    schemes::fptx_weighted::{FPTXWeighted, WeightedBIBEDecryptionKeyShare},
    shared::{
        key_derivation::{BIBEDecryptionKey},
    },
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use aptos_crypto::{weighted_config::WeightedConfigArkworks, SecretSharingConfig as _};
use ark_std::rand::{seq::SliceRandom, thread_rng, CryptoRng, Rng as _, RngCore};

fn weighted_smoke_with_setup<R: RngCore + CryptoRng>(
    rng: &mut R,
    tc_happy: <FPTXWeighted as BatchThresholdEncryption>::ThresholdConfig,
    tc_slow: <FPTXWeighted as BatchThresholdEncryption>::ThresholdConfig,
    ek: <FPTXWeighted as BatchThresholdEncryption>::EncryptionKey,
    dk: <FPTXWeighted as BatchThresholdEncryption>::DigestKey,
    vks_happy: Vec<<FPTXWeighted as BatchThresholdEncryption>::VerificationKey>,
    msk_shares_happy: Vec<<FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare>,
    vks_slow: Vec<<FPTXWeighted as BatchThresholdEncryption>::VerificationKey>,
    msk_shares_slow: Vec<<FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = FPTXWeighted::encrypt(&ek, rng, &plaintext, &associated_data).unwrap();
    FPTXWeighted::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = FPTXWeighted::digest(&dk, &vec![ct.clone()], 0).unwrap();
    let pfs = FPTXWeighted::eval_proofs_compute_all(&pfs_promise, &dk);

    let [dk_happy, dk_slow] = [
        (tc_happy, vks_happy, msk_shares_happy),
        (tc_slow, vks_slow, msk_shares_slow),
    ]
    .into_iter()
    .map(|(tc, vks, msk_shares)| {
        let dk_shares: Vec<<FPTXWeighted as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
            .into_iter()
            .map(|msk_share| msk_share.derive_decryption_key_share(&d).unwrap())
            .collect();

        dk_shares
            .iter()
            .zip(vks)
            .map(|(dk_share, vk)| FPTXWeighted::verify_decryption_key_share(&vk, &d, dk_share))
            .collect::<Result<Vec<()>>>()
            .unwrap();

        let dk = FPTXWeighted::reconstruct_decryption_key(
            &dk_shares
                .choose_multiple(rng, tc.get_total_num_players()) // will be truncated
                .cloned()
                .collect::<Vec<WeightedBIBEDecryptionKeyShare>>(),
            &tc,
        )
        .unwrap();

        ek.verify_decryption_key(&d, &dk).unwrap();

        dk
    })
    .collect::<Vec<BIBEDecryptionKey>>()
    .try_into()
    .unwrap();

    let decrypted_plaintexts: Vec<String> =
        FPTXWeighted::decrypt(&dk_happy, &vec![ct.prepare(&d, &pfs).unwrap()]).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    let decrypted_plaintexts: Vec<String> =
        FPTXWeighted::decrypt(&dk_slow, &vec![ct.prepare(&d, &pfs).unwrap()]).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    // Test individual decryption
    let eval_proof = FPTXWeighted::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        FPTXWeighted::decrypt_individual(&dk_happy, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);

    let eval_proof = FPTXWeighted::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        FPTXWeighted::decrypt_individual(&dk_slow, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}

#[test]
fn weighted_smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc_happy = WeightedConfigArkworks::new(5, vec![1, 2, 5]).unwrap();
    let tc_slow = WeightedConfigArkworks::new(3, vec![1, 2, 5]).unwrap();

    let (ek, dk, vks_happy, msk_shares_happy, vks_slow, msk_shares_slow) =
        FPTXWeighted::setup_for_testing(rng.r#gen(), 8, 1, &tc_happy, &tc_slow).unwrap();

    weighted_smoke_with_setup(
        &mut rng,
        tc_happy,
        tc_slow,
        ek,
        dk,
        vks_happy,
        msk_shares_happy,
        vks_slow,
        msk_shares_slow,
    );
}
