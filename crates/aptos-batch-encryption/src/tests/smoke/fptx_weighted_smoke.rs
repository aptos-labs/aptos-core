// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(test)]
use crate::tests::smoke::run_smoke;
use crate::{
    group::{Fr, G1Affine, G2Affine, Pairing},
    schemes::fptx_weighted::{
        FPTXWeighted, WeightedBIBEMasterSecretKeyShare, WeightedBIBEVerificationKey,
    },
    shared::{digest::DigestKey, encryption_key::EncryptionKey},
    traits::BatchThresholdEncryption,
};
use aptos_crypto::{
    arkworks::{srs::SrsType, GroupGenerators},
    weighted_config::WeightedConfigArkworks,
    TSecretSharingConfig as _,
};
use aptos_dkg::{
    pcs::univariate_hiding_kzg,
    pvss::{
        chunky::{self, chunked_elgamal::num_chunks_per_scalar},
        traits::transcript::Aggregatable,
    },
};
use ark_ec::AffineRepr as _;
#[cfg(test)]
use ark_std::rand::{thread_rng, Rng as _};

pub fn run_pvss(
    dk: &DigestKey,
) -> (
    chunky::PublicParameters<Pairing>,
    WeightedConfigArkworks<Fr>,
    EncryptionKey,
    Vec<WeightedBIBEVerificationKey>,
    Vec<WeightedBIBEMasterSecretKeyShare>,
) {
    let mut aptos_rng = rand::thread_rng();

    let tc = WeightedConfigArkworks::new(3, vec![1, 2, 5]).unwrap();

    let num_chunks =
        num_chunks_per_scalar::<Fr>(aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_DEPLOYMENT);
    let max_num_chunks_padded = (tc.get_total_weight() * num_chunks + 1).next_power_of_two() - 1;

    let trapdoor = univariate_hiding_kzg::Trapdoor::rand(&mut aptos_rng);
    let hkzg_setup = univariate_hiding_kzg::setup_with_trapdoor(
        max_num_chunks_padded + 1,
        SrsType::Lagrange,
        GroupGenerators {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
        },
        trapdoor,
    );

    run_pvss_with_hkzg(dk, (hkzg_setup.1, hkzg_setup.0), &tc)
}

pub fn run_pvss_with_hkzg(
    dk: &DigestKey,
    hkzg_setup: (
        univariate_hiding_kzg::CommitmentKey<Pairing>,
        univariate_hiding_kzg::VerificationKey<Pairing>,
    ),
    tc: &WeightedConfigArkworks<Fr>,
) -> (
    chunky::PublicParameters<Pairing>,
    WeightedConfigArkworks<Fr>,
    EncryptionKey,
    Vec<WeightedBIBEVerificationKey>,
    Vec<WeightedBIBEMasterSecretKeyShare>,
) {
    let mut rng_aptos = rand::thread_rng();

    let pp = <T as TranscriptCore>::PublicParameters::new(
        tc.get_total_weight(),
        aptos_dkg::pvss::chunky::DEFAULT_ELL_FOR_DEPLOYMENT,
        tc.get_total_num_players(),
        G2Affine::generator(),
        hkzg_setup,
        &mut rng_aptos,
    );

    let ssks = (0..tc.get_total_num_players())
        .map(|_| <T as Transcript>::SigningSecretKey::generate(&mut rng_aptos))
        .collect::<Vec<<T as Transcript>::SigningSecretKey>>();
    let spks = ssks
        .iter()
        .map(|ssk| ssk.verifying_key())
        .collect::<Vec<<T as Transcript>::SigningPubKey>>();

    let dks: Vec<<T as TranscriptCore>::DecryptPrivKey> = (0..tc.get_total_num_players())
        .map(|_| <T as TranscriptCore>::DecryptPrivKey::generate(&mut rng_aptos))
        .collect();
    let eks: Vec<<T as TranscriptCore>::EncryptPubKey> = dks
        .iter()
        .map(|dk| dk.to(pp.get_encryption_public_params()))
        .collect();

    let secrets: Vec<<T as Transcript>::InputSecret> = (0..tc.get_total_num_players())
        .map(|_| <T as Transcript>::InputSecret::generate(&mut rng_aptos))
        .collect();

    // Test dealing
    let subtrx_paths: Vec<<T as HasAggregatableSubtranscript>::Subtranscript> = secrets
        .iter()
        .enumerate()
        .map(|(i, s)| {
            T::deal(
                tc,
                &pp,
                &ssks[i],
                &spks[i],
                &eks,
                s,
                &NoAux,
                &tc.get_player(i),
                &mut rng_aptos,
            )
            .get_subtranscript()
        })
        .collect();

    let subtranscript =
        <T as HasAggregatableSubtranscript>::Subtranscript::aggregate(tc, subtrx_paths).unwrap();

    let (ek, vks, _) =
        FPTXWeighted::setup(dk, &pp, &subtranscript, tc, tc.get_player(0), &dks[0]).unwrap();

    let msk_shares: Vec<<FPTXWeighted as BatchThresholdEncryption>::MasterSecretKeyShare> = tc
        .get_players()
        .into_iter()
        .map(|p| {
            let (_, _, msk_share) =
                FPTXWeighted::setup(dk, &pp, &subtranscript, tc, p, &dks[p.get_id()]).unwrap();
            msk_share
        })
        .collect();

    (pp, tc.clone(), ek, vks, msk_shares)
}

#[test]
fn weighted_smoke_with_setup_for_testing() {
    let mut rng = thread_rng();
    let tc = WeightedConfigArkworks::new(3, vec![1, 2, 5]).unwrap();

    let (ek, dk, vks, msk_shares) =
        FPTXWeighted::setup_for_testing(rng.r#gen(), 8, 1, &tc).unwrap();

    run_smoke::<FPTXWeighted>(tc, ek, dk, vks, msk_shares);
}

type T = aptos_dkg::pvss::chunky::SignedWeightedTranscript<crate::group::Pairing>;
use aptos_crypto::{SigningKey, Uniform};
use aptos_dkg::pvss::{
    test_utils::NoAux,
    traits::{
        transcript::{HasAggregatableSubtranscript, TranscriptCore},
        Convert, HasEncryptionPublicParams, Transcript,
    },
};

#[test]
fn weighted_smoke_with_pvss() {
    let dk = DigestKey::new(&mut thread_rng(), 8, 1).unwrap();
    let (_, tc, ek, vks, msk_shares) = run_pvss(&dk);

    run_smoke::<FPTXWeighted>(tc, ek, dk, vks, msk_shares);
}
