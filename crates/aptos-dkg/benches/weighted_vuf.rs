// Copyright © Aptos Foundation

use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, BenchmarkId,
    Criterion, Throughput,
};
use rand::rngs::ThreadRng;
use rand::thread_rng;
use std::ops::Mul;

use aptos_dkg::constants::BEST_CASE_THRESHOLD;
use aptos_dkg::pvss::test_utils::{get_weighted_configs_for_benchmarking, setup_dealing, NoAux};
use aptos_dkg::pvss::traits::{SecretSharingConfig, Transcript};
use aptos_dkg::pvss::{das, Player, ThresholdConfig, WeightedConfig, WeightedTranscript};
use aptos_dkg::utils::random::{random_g1_points, random_g2_points, random_scalars};
use aptos_dkg::utils::{g1_multi_exp, g2_multi_exp, multi_pairing};
use aptos_dkg::weighted_vuf::pinkas::PinkasWUF;
use aptos_dkg::weighted_vuf::traits::WeightedUF;

const BENCH_MSG: &[u8; 36] = b"some dummy message for the benchmark";

pub fn all_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("vuf");
    for t in [10, BEST_CASE_THRESHOLD] {
        unoptimized_threshold_vuf_share_verification(t, &mut group);
    }
    group.finish();

    let mut group = c.benchmark_group("das-pinkas-vuf");
    wvuf_benches::<das::Transcript, PinkasWUF, WallTime>(&mut group);
    group.finish();
}

pub fn wvuf_benches<
    T: Transcript<SecretSharingConfig = ThresholdConfig>,
    WUF: WeightedUF<
        SecretKey = T::DealtSecretKey,
        PubKeyShare = Vec<T::DealtPubKeyShare>,
        SecretKeyShare = Vec<T::DealtSecretKeyShare>,
    >,
    M: Measurement,
>(
    group: &mut BenchmarkGroup<M>,
) where
    WUF::PublicParameters: for<'a> From<&'a T::PublicParameters>,
{
    let mut rng = thread_rng();

    let mut bench_cases = vec![];
    for wc in get_weighted_configs_for_benchmarking() {
        let (pvss_pp, ssks, _spks, dks, eks, iss, _s, dsk) =
            setup_dealing::<WeightedTranscript<T>, ThreadRng>(&wc, &mut rng);

        println!("Dealing a {} PVSS transcript", T::scheme_name());
        let trx = WeightedTranscript::<T>::deal(
            &wc,
            &pvss_pp,
            &ssks[0],
            &eks,
            &iss[0],
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        let vuf_pp = WUF::PublicParameters::from(&pvss_pp);

        let mut sks = vec![];
        let mut pks = vec![];
        let mut asks = vec![];
        let mut apks = vec![];
        let mut deltas = vec![];
        println!(
            "Decrypting shares from {} PVSS transcript",
            T::scheme_name()
        );
        for i in 0..wc.get_total_num_players() {
            let (sk, pk) = trx.decrypt_own_share(&wc, &wc.get_player(i), &dks[i]);

            let (ask, apk) = WUF::augment_key_pair(&vuf_pp, sk.clone(), pk.clone(), &mut rng);
            sks.push(sk);
            pks.push(pk);
            deltas.push(WUF::get_public_delta(&apk).clone());
            asks.push(ask);
            apks.push(apk);
        }

        bench_cases.push((wc, vuf_pp, dsk, sks, pks, asks, apks, deltas));
    }

    for (wc, vuf_pp, sk, sks, pks, asks, apks, deltas) in bench_cases {
        // TODO: smallest weight player and largest weight player
        wvuf_augment_random_keypair::<WeightedTranscript<T>, WUF, ThreadRng, M>(
            &wc, &vuf_pp, &sks, &pks, group, &mut rng,
        );

        // TODO: pick smallest weight player and largest weight player
        // TODO: benchmark augmenting all pubkeys too
        wvuf_augment_random_pubkey::<WeightedTranscript<T>, WUF, ThreadRng, M>(
            &wc, &vuf_pp, &pks, &deltas, group, &mut rng,
        );

        wvuf_create_share::<WeightedTranscript<T>, WUF, ThreadRng, M>(&wc, &asks, group, &mut rng);

        wvuf_verify_share::<WeightedTranscript<T>, WUF, ThreadRng, M>(
            &wc, &vuf_pp, &asks, &apks, group, &mut rng,
        );

        // TODO: pick worst subset and best subset
        wvuf_aggregate_random_shares::<WeightedTranscript<T>, WUF, ThreadRng, M>(
            &wc, &asks, &apks, group, &mut rng,
        );

        wvuf_eval::<WeightedTranscript<T>, WUF, M>(&wc, &sk, group);

        // TODO: derive_eval (needs create_proof)

        // TODO: verify_eval (needs create_proof)
    }
}

fn wvuf_augment_random_keypair<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WUF::PublicParameters,
    sks: &Vec<WT::DealtSecretKeyShare>,
    pks: &Vec<WT::DealtPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_keypair/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(&mut thread_rng()).id;
                (sks[id].clone(), pks[id].clone())
            },
            |(sk, pk)| WUF::augment_key_pair(vuf_pp, sk, pk, rng),
        )
    });
}

fn wvuf_augment_random_pubkey<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WUF::PublicParameters,
    pks: &Vec<WUF::PubKeyShare>,
    deltas: &Vec<WUF::Delta>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_pubkey/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(rng).id;
                let pk = pks[id].clone();
                let delta = deltas[id].clone();

                (pk, delta)
            },
            |(pk, delta)| WUF::augment_pubkey(vuf_pp, pk, delta),
        )
    });
}

fn wvuf_create_share<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    wc: &WeightedConfig,
    asks: &Vec<WUF::AugmentedSecretKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("create_share/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                &asks[player.id]
            },
            |ask| WUF::create_share(ask, BENCH_MSG),
        )
    });
}

fn wvuf_verify_share<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    wc: &WeightedConfig,
    vuf_pp: &WUF::PublicParameters,
    asks: &Vec<WUF::AugmentedSecretKeyShare>,
    apks: &Vec<WUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("verify_share/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                let ask = &asks[player.id];

                (WUF::create_share(ask, BENCH_MSG), &apks[player.id])
            },
            |(proof, apk)| WUF::verify_share(vuf_pp, apk, BENCH_MSG, &proof),
        )
    });
}

fn wvuf_aggregate_random_shares<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
    M: Measurement,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<WUF::AugmentedSecretKeyShare>,
    apks: &Vec<WUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<M>,
    rng: &mut R,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("aggregate_shares/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let players = wc.get_random_eligible_subset_of_players(rng);

                players
                    .iter()
                    .map(|p| {
                        (
                            *p,
                            apks[p.id].clone(),
                            WUF::create_share(&asks[p.id], BENCH_MSG),
                        )
                    })
                    .collect::<Vec<(Player, WUF::AugmentedPubKeyShare, WUF::ProofShare)>>()
            },
            |apks_and_proofs| {
                WUF::aggregate_shares(&wc, apks_and_proofs.as_slice());
            },
        )
    });
}

fn wvuf_eval<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WUF: WeightedUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    M: Measurement,
>(
    wc: &WeightedConfig,
    sk: &WUF::SecretKey,
    group: &mut BenchmarkGroup<M>,
) where
    WUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("eval/{}", wc), move |b| {
        b.iter_with_setup(|| {}, |_| WUF::eval(sk, BENCH_MSG))
    });
}

fn unoptimized_threshold_vuf_share_verification<M: Measurement>(
    t: usize,
    g: &mut BenchmarkGroup<M>,
) {
    let mut rng = thread_rng();

    g.throughput(Throughput::Elements(1u64));

    g.bench_function(
        BenchmarkId::new("unoptimized_threshold_vuf_share_verification", t),
        move |b| {
            b.iter_with_setup(
                || {
                    let g1 = random_g1_points(t + 3, &mut rng);
                    let g2 = random_g2_points(t + 3, &mut rng);
                    let a = random_scalars(t + 3, &mut rng);
                    let b = random_scalars(3, &mut rng);
                    let c = random_scalars(3, &mut rng);

                    (g1, g2, a, b, c)
                },
                |(g1, g2, a, b, c)| {
                    // 2 G_1 exps
                    let _ = g1[0].mul(a[0]);
                    let _ = g1[1].mul(a[1]);
                    // size-3 multipairing
                    multi_pairing(g1.iter().take(3), g2.iter().take(3));
                    // size-4 multipairing
                    multi_pairing(g1.iter().take(4), g2.iter().take(4));
                    // 3 size-2 G_1 multiexps
                    g1_multi_exp(&g1[0..2], &a[0..2]);
                    g1_multi_exp(&g1[0..2], &b[0..2]);
                    g1_multi_exp(&g1[0..2], &c[0..2]);
                    // 1 size-t G_1 multiexp
                    g1_multi_exp(&g1[0..t], &a[0..t]);
                    // 1 size-t G_2 multiexp
                    g2_multi_exp(&g2[0..t], &a[0..t]);
                },
            )
        },
    );
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    //config = Criterion::default();
    targets = all_groups);
criterion_main!(benches);
