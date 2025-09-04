// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use velor_crypto::Uniform;
use velor_dkg::{
    algebra::evaluation_domain::BatchEvaluationDomain,
    pvss,
    pvss::{
        test_utils,
        test_utils::{
            get_threshold_configs_for_benchmarking, get_weighted_configs_for_benchmarking,
            DealingArgs, NoAux,
        },
        traits::{
            transcript::{MalleableTranscript, Transcript},
            SecretSharingConfig,
        },
        LowDegreeTest, WeightedConfig,
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
        let d = pvss_group::<pvss::das::WeightedTranscript>(&wc, c);
        weighted_pvss_group(&wc, d, c);

        // Note: Insecure, so not interested in benchmarks.
        // let d = pvss_group::<GenericWeighting<pvss::das::Transcript>>(&wc, c);
        // weighted_pvss_group(&wc, d, c);
    }

    // LDT
    ldt_group(c);
}

pub fn ldt_group(c: &mut Criterion) {
    let mut rng = thread_rng();

    for sc in get_threshold_configs_for_benchmarking() {
        let mut group = c.benchmark_group("ldt");

        group.bench_function(format!("dual_code_word/{}", sc), move |b| {
            b.iter_with_setup(
                || {
                    let n = sc.get_total_num_players();
                    let t = sc.get_threshold();
                    let batch_dom = BatchEvaluationDomain::new(n);

                    (n, t, batch_dom)
                },
                |(n, t, batch_dom)| {
                    let ldt = LowDegreeTest::random(&mut rng, t, n, true, &batch_dom);
                    ldt.dual_code_word();
                },
            )
        });
    }
}

pub fn pvss_group<T: MalleableTranscript>(
    sc: &T::SecretSharingConfig,
    c: &mut Criterion,
) -> DealingArgs<T> {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("pvss/{}", name));
    let mut rng = thread_rng();

    // TODO: use a lazy pattern to avoid this expensive step when no benchmarks are run
    let d = test_utils::setup_dealing::<T, ThreadRng>(sc, &mut rng);

    // pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &d.pp, &d.ssks, &d.eks, &mut group);
    pvss_aggregate::<T, WallTime>(sc, &mut group);
    pvss_verify::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(sc, &d.pp, &d.ssks, &d.dks, &d.eks, &d.s, &mut group);

    group.finish();

    d
}

pub fn weighted_pvss_group<T: MalleableTranscript<SecretSharingConfig = WeightedConfig>>(
    sc: &T::SecretSharingConfig,
    d: DealingArgs<T>,
    c: &mut Criterion,
) {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("wpvss/{}", name));
    let mut rng = thread_rng();

    let average_aggregation_size = sc.get_average_size_of_eligible_subset(250, &mut rng);
    pvss_aggregate_verify::<T, WallTime>(
        sc,
        &d.pp,
        &d.ssks,
        &d.spks,
        &d.eks,
        &d.iss[0],
        average_aggregation_size,
        &mut group,
    );

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
    assert_eq!(ssks.len(), spks.len());

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

                for (i, ssk) in ssks.iter().enumerate().skip(1).take(num_aggr - 1) {
                    let mut trx = trxs[0].clone();
                    trx.maul_signature(ssk, &NoAux, &sc.get_player(i));
                    trxs.push(trx);
                }
                assert_eq!(spks.len(), trxs.len());

                let trx = T::aggregate(sc, trxs).unwrap();
                assert_eq!(trx.get_dealers().len(), num_aggr);
                trx
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
