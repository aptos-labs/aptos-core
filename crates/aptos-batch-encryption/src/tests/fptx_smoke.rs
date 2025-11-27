// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    schemes::fptx::FPTX,
    shared::{
        algebra::shamir::ThresholdConfig,
        key_derivation::{BIBEDecryptionKey, BIBEDecryptionKeyShare},
    },
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use ark_std::rand::{seq::SliceRandom, thread_rng, Rng as _};
use rayon::ThreadPoolBuilder;

#[test]
fn smoke() {
    let mut rng = thread_rng();
    let tc_happy = ThresholdConfig::new(8, 5);
    let tc_slow = ThresholdConfig::new(8, 3);
    let tp = ThreadPoolBuilder::new().build().unwrap();

    let (ek, dk, vks_happy, msk_shares_happy, vks_slow, msk_shares_slow) =
        FPTX::setup_for_testing(rng.gen(), 8, 1, &tc_happy, &tc_slow).unwrap();

    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = FPTX::encrypt(&ek, &mut rng, &plaintext, &associated_data).unwrap();
    FPTX::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = FPTX::digest(&dk, &vec![ct.clone()], 0, &tp).unwrap();
    let pfs = FPTX::eval_proofs_compute_all(&pfs_promise, &dk, &tp);

    let [dk_happy, dk_slow] = [
        (tc_happy, vks_happy, msk_shares_happy),
        (tc_slow, vks_slow, msk_shares_slow),
    ]
    .into_iter()
    .map(|(tc, vks, msk_shares)| {
        let dk_shares: Vec<<FPTX as BatchThresholdEncryption>::DecryptionKeyShare> = msk_shares
            .into_iter()
            .map(|msk_share| msk_share.derive_decryption_key_share(&d).unwrap())
            .collect();

        dk_shares
            .iter()
            .zip(vks)
            .map(|(dk_share, vk)| FPTX::verify_decryption_key_share(&vk, &d, dk_share))
            .collect::<Result<Vec<()>>>()
            .unwrap();

        let dk = FPTX::reconstruct_decryption_key(
            &dk_shares
                .choose_multiple(&mut rng, tc.t)
                .cloned()
                .collect::<Vec<BIBEDecryptionKeyShare>>(),
            &tc,
            &tp,
        )
        .unwrap();

        ek.verify_decryption_key(&d, &dk).unwrap();

        dk
    })
    .collect::<Vec<BIBEDecryptionKey>>()
    .try_into()
    .unwrap();

    let decrypted_plaintexts: Vec<String> =
        FPTX::decrypt(&dk_happy, &vec![ct.prepare(&d, &pfs).unwrap()], &tp).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    let decrypted_plaintexts: Vec<String> =
        FPTX::decrypt(&dk_slow, &vec![ct.prepare(&d, &pfs).unwrap()], &tp).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    // Test individual decryption
    let eval_proof = FPTX::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        FPTX::decrypt_individual(&dk_happy, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);

    let eval_proof = FPTX::eval_proof_for_ct(&pfs, &ct).unwrap();
    let individual_decrypted_plaintext: String =
        FPTX::decrypt_individual(&dk_slow, &ct, &d, &eval_proof).unwrap();
    assert_eq!(individual_decrypted_plaintext, plaintext);
}

#[test]
fn fptx_serialize_deserialize_setup() {
    let mut rng = thread_rng();
    let tc_happy = ThresholdConfig::new(8, 5);
    let tc_slow = ThresholdConfig::new(8, 3);

    let setup = FPTX::setup_for_testing(rng.gen(), 8, 2, &tc_happy, &tc_slow).unwrap();

    let bytes = bcs::to_bytes(&setup).unwrap();
    let setup2: (
        <FPTX as BatchThresholdEncryption>::EncryptionKey,
        <FPTX as BatchThresholdEncryption>::DigestKey,
        Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
        Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) = bcs::from_bytes(&bytes).unwrap();

    assert_eq!(setup, setup2);

    let json = serde_json::to_string(&setup).unwrap();
    let setup2: (
        <FPTX as BatchThresholdEncryption>::EncryptionKey,
        <FPTX as BatchThresholdEncryption>::DigestKey,
        Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
        Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
        Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) = serde_json::from_str(&json).unwrap();
    assert_eq!(setup, setup2);
}
