// Copyright © Aptos Foundation

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use aptos_crypto::Uniform;
use aptos_dkg::{
    pvss,
    pvss::{
        test_utils,
        test_utils::{
            get_threshold_configs_for_benchmarking, get_weighted_configs_for_benchmarking, NoAux,
        },
        traits::{
            transcript::{MalleableTranscript, Transcript},
            SecretSharingConfig,
        },
        GenericWeighting,
    },
};
use criterion::{
    criterion_group, criterion_main,
    measurement::{Measurement, WallTime},
    BenchmarkGroup, Criterion, Throughput,
};
use more_asserts::assert_le;
use rand::{rngs::ThreadRng, thread_rng, Rng};

pub fn all_groups(c: &mut Criterion) {
    // unweighted PVSS
    for tc in get_threshold_configs_for_benchmarking() {
        pvss_group::<pvss::das::Transcript>(&tc, c);
    }

    // weighted PVSS
    for wc in get_weighted_configs_for_benchmarking() {
        pvss_group::<pvss::das::WeightedTranscript>(&wc, c);
        pvss_group::<GenericWeighting<pvss::das::Transcript>>(&wc, c);
    }
}

pub fn pvss_group<T: MalleableTranscript>(sc: &T::SecretSharingConfig, c: &mut Criterion) {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("pvss/{}", name));
    let mut rng = thread_rng();

    // TODO: use a lazy pattern to avoid this expensive step when no benchmarks are run
    let (pp, ssks, spks, dks, eks, iss, s, _) =
        test_utils::setup_dealing::<T, ThreadRng>(sc, &mut rng);

    // pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &pp, &ssks, &eks, &mut group);
    pvss_aggregate::<T, WallTime>(sc, &mut group);
    pvss_verify::<T, WallTime>(sc, &pp, &ssks, &spks, &eks, &mut group);
    pvss_aggregate_verify::<T, WallTime>(sc, &pp, &ssks, &spks, &eks, &iss[0], 100, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(sc, &pp, &ssks, &dks, &eks, &s, &mut group);

    group.finish();
}

fn pvss_deal<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &Vec<T::SigningSecretKey>,
    eks: &Vec<T::EncryptPubKey>,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();

    g.bench_function(format!("deal/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                (s, rng)
            },
            |(s, mut rng)| {
                T::deal(
                    &sc,
                    &pp,
                    &ssks[0],
                    &eks,
                    &s,
                    &NoAux,
                    &sc.get_player(0),
                    &mut rng,
                )
            },
        )
    });
}

fn pvss_aggregate<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));
    let mut rng = thread_rng();

    g.bench_function(format!("aggregate/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let trx = T::generate(&sc, &mut rng);
                (trx.clone(), trx)
            },
            |(mut first, second)| {
                first.aggregate_with(&sc, &second);
            },
        )
    });
}

fn pvss_verify<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &Vec<T::SigningSecretKey>,
    spks: &Vec<T::SigningPubKey>,
    eks: &Vec<T::EncryptPubKey>,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();

    g.bench_function(format!("verify/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                T::deal(
                    &sc,
                    &pp,
                    &ssks[0],
                    &eks,
                    &s,
                    &NoAux,
                    &sc.get_player(0),
                    &mut rng,
                )
            },
            |trx| {
                trx.verify(&sc, &pp, &vec![spks[0].clone()], &eks, &vec![NoAux])
                    .expect("PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_aggregate_verify<T: MalleableTranscript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &Vec<T::SigningSecretKey>,
    spks: &Vec<T::SigningPubKey>,
    eks: &Vec<T::EncryptPubKey>,
    iss: &T::InputSecret,
    num_aggr: usize,
    g: &mut BenchmarkGroup<M>,
) {
    // Currently, our codebase assumes a DKG setting where there are as many dealers as there are
    // players obtaining shares. (In other settings, there could be 1 million dealers, dealing a
    // secret to only 100 players such that, say, any 50 can reconstruct them.)
    assert_le!(num_aggr, sc.get_total_num_players());

    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();

    // Aggregated transcript will have SoKs from `num_aggr` players.
    let mut spks = spks.clone();
    spks.truncate(num_aggr);

    g.bench_function(format!("aggregate_verify/{}/{}", sc, num_aggr), move |b| {
        b.iter_with_setup(
            || {
                let mut trxs = vec![];
                trxs.push(T::deal(
                    &sc,
                    &pp,
                    &ssks[0],
                    &eks,
                    iss,
                    &NoAux,
                    &sc.get_player(0),
                    &mut rng,
                ));

                for (i, ssk) in ssks.iter().enumerate().skip(1).take(num_aggr) {
                    let mut trx = trxs[0].clone();
                    trx.maul_signature(ssk, &NoAux, &sc.get_player(i));
                    trxs.push(trx);
                }

                T::aggregate(sc, trxs).unwrap()
            },
            |trx| {
                trx.verify(&sc, &pp, &spks, &eks, &vec![NoAux; num_aggr])
                    .expect("aggregate PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_decrypt_own_share<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &Vec<T::SigningSecretKey>,
    dks: &Vec<T::DecryptPrivKey>,
    eks: &Vec<T::EncryptPubKey>,
    s: &T::InputSecret,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();

    let trx = T::deal(
        &sc,
        &pp,
        &ssks[0],
        &eks,
        &s,
        &NoAux,
        &sc.get_player(0),
        &mut rng,
    );

    g.bench_function(format!("decrypt-share/{}", sc), move |b| {
        b.iter_with_setup(
            || rng.gen_range(0, sc.get_total_num_players()),
            |i| {
                trx.decrypt_own_share(&sc, &sc.get_player(i), &dks[i]);
            },
        )
    });
}

#[allow(dead_code)]
fn pvss_transcript_random<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();

    g.bench_function(format!("transcript-random/{}", sc), move |b| {
        b.iter(|| T::generate(&sc, &mut rng))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    //config = Criterion::default();
    targets = all_groups);
criterion_main!(benches);
