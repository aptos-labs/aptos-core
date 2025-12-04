// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    group::{G2Affine}, schemes::fptx::{FPTX}, shared::{digest::DigestKey, key_derivation::{BIBEDecryptionKey, BIBEDecryptionKeyShare}}, traits::BatchThresholdEncryption
};
use anyhow::Result;
use aptos_crypto::{arkworks::shamir::ShamirThresholdConfig,  SecretSharingConfig as _};
use ark_ec::AffineRepr as _;
use ark_std::rand::{seq::SliceRandom, thread_rng, CryptoRng, Rng as _, RngCore};


fn smoke_with_setup<R: RngCore + CryptoRng>(
    rng: &mut R,
    tc_happy: <FPTX as BatchThresholdEncryption>::ThresholdConfig,
    tc_slow: <FPTX as BatchThresholdEncryption>::ThresholdConfig,
    ek: <FPTX as BatchThresholdEncryption>::EncryptionKey,
    dk: <FPTX as BatchThresholdEncryption>::DigestKey,
    vks_happy: Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
    msk_shares_happy: Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
    vks_slow: Vec<<FPTX as BatchThresholdEncryption>::VerificationKey>,
    msk_shares_slow: Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>,
) {
    let plaintext: String = String::from("hi");
    let associated_data: String = String::from("hi");

    let ct = FPTX::encrypt(&ek, rng, &plaintext, &associated_data).unwrap();
    FPTX::verify_ct(&ct, &associated_data).unwrap();

    let (d, pfs_promise) = FPTX::digest(&dk, &vec![ct.clone()], 0).unwrap();
    let pfs = FPTX::eval_proofs_compute_all(&pfs_promise, &dk);

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
                .choose_multiple(rng, tc.t)
                .cloned()
                .collect::<Vec<BIBEDecryptionKeyShare>>(),
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
        FPTX::decrypt(&dk_happy, &vec![ct.prepare(&d, &pfs).unwrap()]).unwrap();

    assert_eq!(decrypted_plaintexts[0], plaintext);

    let decrypted_plaintexts: Vec<String> =
        FPTX::decrypt(&dk_slow, &vec![ct.prepare(&d, &pfs).unwrap()]).unwrap();

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
fn smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc_happy = ShamirThresholdConfig::new(5, 8);
    let tc_slow = ShamirThresholdConfig::new(3, 8);

    let (ek, dk, vks_happy, msk_shares_happy, vks_slow, msk_shares_slow) =
        FPTX::setup_for_testing(rng.r#gen(), 8, 1, &tc_happy, &tc_slow).unwrap();

    smoke_with_setup(&mut rng, tc_happy, tc_slow, ek, dk, vks_happy, msk_shares_happy, vks_slow, msk_shares_slow);
}

type T = aptos_dkg::pvss::chunky::Transcript<crate::group::Pairing>;
use aptos_dkg::{pvss::{test_utils::NoAux, traits::{transcript::HasAggregatableSubtranscript, Transcript}, Player}, Scalar};
use aptos_dkg::pvss::traits::{HasEncryptionPublicParams, Convert};
use aptos_crypto::{SigningKey, Uniform};

#[test]
fn smoke_with_pvss() {
    let mut rng = thread_rng();
    let mut rng_aptos_crypto = rand::thread_rng();

    let tc_happy = ShamirThresholdConfig::new(5, 8);
    let tc_slow = ShamirThresholdConfig::new(3, 8);
    let pp = <T as Transcript>::PublicParameters::new_with_commitment_base(
        tc_happy.get_total_num_players(),
        aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_TESTING,
        G2Affine::generator(),
        &mut rng_aptos_crypto);

    let ssks = (0..tc_happy.get_total_num_players())
        .map(|_| <T as Transcript>::SigningSecretKey::generate(&mut rng_aptos_crypto))
        .collect::<Vec<<T as Transcript>::SigningSecretKey>>();
    let spks = ssks
        .iter()
        .map(|ssk| ssk.verifying_key())
        .collect::<Vec<<T as Transcript>::SigningPubKey>>();

    let dks = (0..tc_happy.get_total_num_players())
        .map(|_| <T as Transcript>::DecryptPrivKey::generate(&mut rng_aptos_crypto))
        .collect::<Vec<<T as Transcript>::DecryptPrivKey>>();
    let eks = dks
        .iter()
        .map(|dk| dk.to(&pp.get_encryption_public_params()))
        .collect();

    let s = <T as Transcript>::InputSecret::generate(&mut rng_aptos_crypto);


    // Test dealing
    let subtrx_happypath = T::deal(
        &tc_happy,
        &pp,
        &ssks[0],
        &spks[0],
        &eks,
        &s,
        &NoAux,
        &tc_happy.get_player(0),
        &mut rng_aptos_crypto,
    ).get_subtranscript();

    let subtrx_slowpath = T::deal(
        &tc_slow,
        &pp,
        &ssks[0],
        &spks[0],
        &eks,
        &s,
        &NoAux,
        &tc_slow.get_player(0),
        &mut rng_aptos_crypto,
    ).get_subtranscript();

    let dk = DigestKey::new(&mut rng, 8, 1).unwrap();

    let (ek, vks_happy, _, vks_slow, _) = FPTX::setup(
        &dk,
        &pp,
        &subtrx_happypath,
        &subtrx_slowpath,
        &tc_happy,
        &tc_slow,
        tc_happy.get_player(0),
        &dks[0]
    ).unwrap();

    let (msk_shares_happy, msk_shares_slow) : (Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>, Vec<<FPTX as BatchThresholdEncryption>::MasterSecretKeyShare>) = tc_happy.get_players()
        .into_iter()
    .map(|p| {
            let (__, _, msk_share_happypath, _, msk_share_slowpath) = FPTX::setup(
                &dk,
                &pp,
                &subtrx_happypath,
                &subtrx_slowpath,
                &tc_happy,
                &tc_slow,
                p,
                &dks[p.get_id()]).unwrap();
            (msk_share_happypath, msk_share_slowpath)
        }).collect();



    smoke_with_setup(
        &mut rng,
        tc_happy,
        tc_slow,
        ek,
        dk,
        vks_happy,
        msk_shares_happy,
        vks_slow,
        msk_shares_slow
    );
}

#[test]
fn fptx_serialize_deserialize_setup() {
    let mut rng = thread_rng();
    let tc_happy = ShamirThresholdConfig::new(5, 8);
    let tc_slow = ShamirThresholdConfig::new(3, 8);

    let setup = FPTX::setup_for_testing(rng.r#gen(), 8, 2, &tc_happy, &tc_slow).unwrap();

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
