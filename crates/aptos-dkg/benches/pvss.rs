// Copyright © Aptos Foundation

use aptos_crypto::Uniform;
use aptos_dkg::constants::{
    BEST_CASE_N, BEST_CASE_THRESHOLD, WORST_CASE_N, WORST_CASE_THRESHOLD,
};
use aptos_dkg::pvss;
use aptos_dkg::pvss::traits::transcript::Transcript;
use aptos_dkg::pvss::traits::SecretSharingConfig;
use aptos_dkg::pvss::{test_utils, ThresholdConfig};
use criterion::measurement::WallTime;
use criterion::{
    criterion_group, criterion_main, measurement::Measurement, BenchmarkGroup, Criterion,
    Throughput,
};
use rand::rngs::ThreadRng;
use rand::{thread_rng, Rng};

pub fn all_groups(c: &mut Criterion) {
    let best_case_tc = ThresholdConfig::new(BEST_CASE_THRESHOLD, BEST_CASE_N).unwrap();
    let worst_case_tc = ThresholdConfig::new(WORST_CASE_THRESHOLD, WORST_CASE_N).unwrap();

    for tc in [best_case_tc, worst_case_tc] {
        pvss_group::<pvss::scrape::Transcript>(&tc, c);
        pvss_group::<pvss::das::Transcript>(&tc, c);
    }
}

pub fn pvss_group<T: Transcript>(sc: &T::SecretSharingConfig, c: &mut Criterion) {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("pvss/{}", name));

    pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &mut group);
    pvss_aggregate::<T, WallTime>(sc, &mut group);
    pvss_verify::<T, WallTime>(sc, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(sc, &mut group);

    group.finish();
}

fn pvss_deal<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();
    let (pp, _, eks, _, _) = test_utils::setup_dealing::<T, ThreadRng>(sc, &mut rng);

    g.bench_function(format!("deal/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                (s, rng)
            },
            |(s, mut rng)| T::deal(&sc, &pp, &eks, &s, &mut rng),
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
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();
    let (pp, _, eks, _, _) = test_utils::setup_dealing::<T, ThreadRng>(sc, &mut rng);

    g.bench_function(format!("verify/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                T::deal(&sc, &pp, &eks, &s, &mut rng)
            },
            |trx| {
                trx.verify(&sc, &pp, &eks)
                    .expect("PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_decrypt_own_share<T: Transcript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = thread_rng();
    let (pp, dks, eks, _, _) = test_utils::setup_dealing::<T, ThreadRng>(sc, &mut rng);

    let s = T::InputSecret::generate(&mut rng);
    let trx = T::deal(&sc, &pp, &eks, &s, &mut rng);

    g.bench_function(format!("decrypt-share/{}", sc), move |b| {
        b.iter_with_setup(
            || rng.gen_range(0, sc.get_total_num_players()),
            |i| {
                trx.decrypt_own_share(&sc, &sc.get_player(i), &dks[i]);
            },
        )
    });
}

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
