// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    schemes::fptx_weighted::{FPTXWeighted, WeightedBIBEDecryptionKeyShare},
    shared::{digest::DigestKey, key_derivation::BIBEDecryptionKey},
    traits::BatchThresholdEncryption,
};
use anyhow::Result;
use aptos_crypto::{weighted_config::WeightedConfigArkworks, SecretSharingConfig as _};
use ark_ec::AffineRepr as _;
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
        let dk_shares: Vec<<FPTXWeighted as BatchThresholdEncryption>::DecryptionKeyShare> =
            msk_shares
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

type T = aptos_dkg::pvss::chunky::WeightedTranscript<crate::group::Pairing>;
type C = WeightedConfigArkworks<Fr>;
use crate::group::{Fr, G2Affine};
use aptos_crypto::{SigningKey, Uniform};
use aptos_dkg::pvss::{
    test_utils::NoAux,
    traits::{
        transcript::{Aggregatable, HasAggregatableSubtranscript},
        Convert, HasEncryptionPublicParams, Transcript,
    },
};

#[test]
fn weighted_smoke_with_pvss() {
    let mut rng = thread_rng();
    let mut rng_aptos = rand::thread_rng();

    let tc_happy = WeightedConfigArkworks::new(5, vec![1, 2, 5]).unwrap();
    let tc_slow = WeightedConfigArkworks::new(3, vec![1, 2, 5]).unwrap();
    let pp = <T as Transcript>::PublicParameters::new_with_commitment_base(
        tc_happy.get_total_weight(),
        aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_TESTING,
        tc_happy.get_total_num_players(),
        G2Affine::generator(),
        &mut rng_aptos,
    );

    let ssks = (0..tc_happy.get_total_num_players())
        .map(|_| <T as Transcript>::SigningSecretKey::generate(&mut rng_aptos))
        .collect::<Vec<<T as Transcript>::SigningSecretKey>>();
    let spks = ssks
        .iter()
        .map(|ssk| ssk.verifying_key())
        .collect::<Vec<<T as Transcript>::SigningPubKey>>();

    let dks: Vec<<T as Transcript>::DecryptPrivKey> = (0..tc_happy.get_total_num_players())
        .map(|_| <T as Transcript>::DecryptPrivKey::generate(&mut rng_aptos))
        .collect();
    let eks: Vec<<T as Transcript>::EncryptPubKey> = dks
        .iter()
        .map(|dk| dk.to(pp.get_encryption_public_params()))
        .collect();

    let secrets: Vec<<T as Transcript>::InputSecret> = (0..tc_happy.get_total_num_players())
        .map(|_| <T as Transcript>::InputSecret::generate(&mut rng_aptos))
        .collect();

    // Test dealing
    let subtrx_happypaths: Vec<<T as HasAggregatableSubtranscript<C>>::SubTranscript> = secrets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            T::deal(
                &tc_happy,
                &pp,
                &ssks[i],
                &spks[i],
                &eks,
                s,
                &NoAux,
                &tc_happy.get_player(i),
                &mut rng_aptos,
            )
            .get_subtranscript()
        })
        .collect();

    let subtrx_slowpaths: Vec<<T as HasAggregatableSubtranscript<C>>::SubTranscript> = secrets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            T::deal(
                &tc_slow,
                &pp,
                &ssks[i],
                &spks[i],
                &eks,
                s,
                &NoAux,
                &tc_slow.get_player(i),
                &mut rng_aptos,
            )
            .get_subtranscript()
        })
        .collect();

    let mut subtrx_happypath = subtrx_happypaths[0].clone();
    for acc in &subtrx_happypaths[1..] {
        subtrx_happypath.aggregate_with(&tc_happy, acc).unwrap();
    }

    let mut subtrx_slowpath = subtrx_slowpaths[0].clone();
    for acc in &subtrx_slowpaths[1..] {
        subtrx_slowpath.aggregate_with(&tc_slow, acc).unwrap();
    }

    let dk = DigestKey::new(&mut rng, 8, 1).unwrap();

    let (ek, vks_happy, _, vks_slow, _) = FPTXWeighted::setup(
        &dk,
        &pp,
        &subtrx_happypath,
        &subtrx_slowpath,
        &tc_happy,
        &tc_slow,
        tc_happy.get_player(0),
        &dks[0],
    )
    .unwrap();

    let (msk_shares_happy, msk_shares_slow): (
        Vec<<FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare>,
        Vec<<FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) = tc_happy
        .get_players()
        .into_iter()
        .map(|p| {
            let (_, _, msk_share_happypath, _, msk_share_slowpath) = FPTXWeighted::setup(
                &dk,
                &pp,
                &subtrx_happypath,
                &subtrx_slowpath,
                &tc_happy,
                &tc_slow,
                p,
                &dks[p.get_id()],
            )
            .unwrap();
            (msk_share_happypath, msk_share_slowpath)
        })
        .collect();

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
