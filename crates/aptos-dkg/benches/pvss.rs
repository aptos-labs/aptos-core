// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::ptr_arg)]
#![allow(clippy::needless_borrow)]

use aptos_crypto::{TSecretSharingConfig, Uniform};
use aptos_dkg::pvss::{
    chunky::{UnsignedWeightedTranscript as Chunky_v1, UnsignedWeightedTranscriptv2 as Chunky_v2},
    das,
    test_utils::{
        self, get_threshold_configs_for_benchmarking, get_weighted_configs_for_benchmarking,
        DealingArgs, NoAux,
    },
    traits::transcript::{
        Aggregatable, AggregatableTranscript, Aggregated, HasAggregatableSubtranscript,
        MalleableTranscript, Transcript, WithMaxNumShares,
    },
    WeightedConfigBlstrs,
};
use ark_bls12_381::Bls12_381;
use criterion::{
    black_box, criterion_group, criterion_main,
    measurement::{Measurement, WallTime},
    BenchmarkGroup, Criterion, Throughput,
};
use more_asserts::assert_le;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};

// const BN254: &str = "bn254";
const BLS12_381: &str = "bls12-381";

pub fn all_groups(c: &mut Criterion) {
    println!("Rayon num threads: {}", rayon::current_num_threads());

    // weighted PVSS with aggregatable subtranscript; only doing one at the moment because large configs are a bit slow and not relevant anyway
    // Chunky_v1
    for tc in get_weighted_configs_for_benchmarking().into_iter().take(1) {
        subaggregatable_pvss_group::<Chunky_v1<Bls12_381>>(&tc, c, Some(16u8), BLS12_381);
    }
    for tc in get_weighted_configs_for_benchmarking().into_iter().take(1) {
        subaggregatable_pvss_group::<Chunky_v1<Bls12_381>>(&tc, c, Some(32u8), BLS12_381);
    }

    // Chunky_v2
    for tc in get_weighted_configs_for_benchmarking().into_iter().take(1) {
        subaggregatable_pvss_group::<Chunky_v2<Bls12_381>>(&tc, c, Some(16u8), BLS12_381);
    }
    for tc in get_weighted_configs_for_benchmarking().into_iter().take(1) {
        subaggregatable_pvss_group::<Chunky_v2<Bls12_381>>(&tc, c, Some(32u8), BLS12_381);
    }

    // unweighted aggregatable PVSS, `blstrs` only so this is BLS12-381
    for tc in get_threshold_configs_for_benchmarking() {
        aggregatable_pvss_group::<das::Transcript>(&tc, c);
    }

    // weighted aggregatable PVSS, `blstrs` only so this is BLS12-381
    for wc in get_weighted_configs_for_benchmarking() {
        let d = aggregatable_pvss_group::<das::WeightedTranscript>(&wc, c);
        weighted_pvss_group(&wc, d, c);

        // Note: Insecure, so not interested in benchmarks.
        // let d = pvss_group::<GenericWeighting<pvss::das::Transcript>>(&wc, c);
        // weighted_pvss_group(&wc, d, c);
    }
}

pub fn aggregatable_pvss_group<T: AggregatableTranscript + MalleableTranscript>(
    sc: &<T as Transcript>::SecretSharingConfig,
    c: &mut Criterion,
) -> DealingArgs<T> {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("pvss/{}", name));
    let mut rng = StdRng::seed_from_u64(42);

    // TODO: use a lazy pattern to avoid this expensive step when no benchmarks are run
    let d = test_utils::setup_dealing::<T, _>(sc, None, &mut rng);

    // pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_aggregate::<T, WallTime>(sc, &mut group);
    pvss_verify::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(
        sc, &d.pp, &d.ssks, &d.spks, &d.dks, &d.eks, &d.s, &mut group,
    );

    group.finish();

    d
}

// TODO: combine with function above, rather than copy-paste
pub fn subaggregatable_pvss_group<T>(
    sc: &T::SecretSharingConfig,
    c: &mut Criterion,
    ell: Option<u8>,
    curve_name: &str,
) -> DealingArgs<T>
where
    T: MalleableTranscript
        + HasAggregatableSubtranscript<
            Subtranscript: Aggregatable<
                SecretSharingConfig = <T as Transcript>::SecretSharingConfig,
            >,
        >,
{
    let name = T::scheme_name();
    let group_name = match ell {
        Some(ell) => format!("pvss/{}/{}/{}", name, curve_name, ell),
        None => format!("pvss/{}/{}", name, curve_name),
    };
    let mut group = c.benchmark_group(group_name);
    let mut rng = StdRng::seed_from_u64(42);

    // TODO: use a lazy pattern to avoid this expensive step when no benchmarks are run
    let d = test_utils::setup_dealing::<T, _>(sc, ell, &mut rng);

    // pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_nonaggregate_serialize::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_subaggregate::<T, WallTime>(sc, &mut group);
    pvss_nonaggregate_verify::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(
        sc, &d.pp, &d.ssks, &d.spks, &d.dks, &d.eks, &d.s, &mut group,
    );

    group.finish();

    d
}

pub fn weighted_pvss_group<
    T: AggregatableTranscript + MalleableTranscript<SecretSharingConfig = WeightedConfigBlstrs>,
>(
    sc: &<T as Transcript>::SecretSharingConfig,
    d: DealingArgs<T>,
    c: &mut Criterion,
) {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("wpvss/{}", name));
    let mut rng = StdRng::seed_from_u64(42);

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
    ssks: &[T::SigningSecretKey],
    spks: &[T::SigningPubKey],
    eks: &[T::EncryptPubKey],
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
                    &spks[0],
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

fn pvss_aggregate<T: AggregatableTranscript, M: Measurement>(
    sc: &<T as Transcript>::SecretSharingConfig,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));
    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("aggregate/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let trx = T::generate(
                    &sc,
                    &T::PublicParameters::with_max_num_shares(
                        sc.get_total_num_shares().try_into().unwrap(),
                    ),
                    &mut rng,
                );
                (trx.clone(), trx)
            },
            |(first, second)| {
                let mut agg = first.to_aggregated();
                agg.aggregate_with(&sc, &second).unwrap();
            },
        )
    });
}

fn pvss_subaggregate<T, M: Measurement>(sc: &T::SecretSharingConfig, g: &mut BenchmarkGroup<M>)
where
    T: HasAggregatableSubtranscript<
        Subtranscript: Aggregatable<SecretSharingConfig = <T as Transcript>::SecretSharingConfig>,
    >,
{
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));
    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("aggregate/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let trs = T::generate(
                    &sc,
                    &T::PublicParameters::with_max_num_shares(
                        sc.get_total_num_shares().try_into().unwrap(),
                    ),
                    &mut rng,
                );
                (trs.clone(), trs)
            },
            |(first, second)| {
                let mut agg = first.get_subtranscript().to_aggregated();
                agg.aggregate_with(&sc, &second.get_subtranscript())
                    .unwrap();
            },
        )
    });
}

fn pvss_verify<T: AggregatableTranscript, M: Measurement>(
    sc: &<T as Transcript>::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &[T::SigningSecretKey],
    spks: &[T::SigningPubKey],
    eks: &[T::EncryptPubKey],
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("verify/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                T::deal(
                    &sc,
                    &pp,
                    &ssks[0],
                    &spks[0],
                    &eks,
                    &s,
                    &NoAux,
                    &sc.get_player(0),
                    &mut rng,
                )
            },
            |trx| {
                trx.verify(&sc, &pp, &[spks[0].clone()], &eks, &[NoAux])
                    .expect("PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_nonaggregate_serialize<T: HasAggregatableSubtranscript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &[T::SigningSecretKey],
    spks: &[T::SigningPubKey],
    eks: &[T::EncryptPubKey],
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = StdRng::seed_from_u64(42);

    let transcript_size = {
        let s = T::InputSecret::generate(&mut rng);
        let trs = T::deal(
            &sc,
            &pp,
            &ssks[0],
            &spks[0],
            &eks,
            &s,
            &NoAux,
            &sc.get_player(0),
            &mut rng,
        );
        trs.to_bytes().len()
    };

    g.bench_function(
        format!("serialize/{}/transcript_bytes={}", sc, transcript_size),
        move |b| {
            b.iter_with_setup(
                || {
                    let s = T::InputSecret::generate(&mut rng);
                    T::deal(
                        &sc,
                        &pp,
                        &ssks[0],
                        &spks[0],
                        &eks,
                        &s,
                        &NoAux,
                        &sc.get_player(0),
                        &mut rng,
                    )
                },
                |trs| {
                    let bytes = trs.to_bytes();
                    black_box(&bytes);
                },
            )
        },
    );
}

fn pvss_nonaggregate_verify<T: HasAggregatableSubtranscript, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &[T::SigningSecretKey],
    spks: &[T::SigningPubKey],
    eks: &[T::EncryptPubKey],
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("verify/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let s = T::InputSecret::generate(&mut rng);
                let trs = T::deal(
                    &sc,
                    &pp,
                    &ssks[0],
                    &spks[0],
                    &eks,
                    &s,
                    &NoAux,
                    &sc.get_player(0),
                    &mut rng,
                );
                T::try_from(trs.to_bytes().as_slice())
                    .expect("serialized transcript should deserialize correctly")
                // we have to serialize and deserialize because otherwise verify gets a transcript with "non-normalised" projective group elements
            },
            |trx| {
                trx.verify(&sc, &pp, &[spks[0].clone()], &eks, &NoAux)
                    .expect("PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_aggregate_verify<T: AggregatableTranscript + MalleableTranscript, M: Measurement>(
    sc: &<T as Transcript>::SecretSharingConfig,
    pp: &T::PublicParameters,
    ssks: &[T::SigningSecretKey],
    spks: &Vec<T::SigningPubKey>,
    eks: &[T::EncryptPubKey],
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

    let mut rng = StdRng::seed_from_u64(42);

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
                    &spks[0],
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
    ssks: &[T::SigningSecretKey],
    spks: &[T::SigningPubKey],
    dks: &[T::DecryptPrivKey],
    eks: &[T::EncryptPubKey],
    s: &T::InputSecret,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));

    let mut rng = StdRng::seed_from_u64(42);

    let trx = T::deal(
        &sc,
        &pp,
        &ssks[0],
        &spks[0],
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
                trx.decrypt_own_share(&sc, &sc.get_player(i), &dks[i], pp);
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

    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("transcript-random/{}", sc), move |b| {
        b.iter(|| {
            T::generate(
                &sc,
                &T::PublicParameters::with_max_num_shares(
                    sc.get_total_num_shares().try_into().unwrap(),
                ),
                &mut rng,
            )
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    //config = Criterion::default();
    targets = all_groups);
criterion_main!(benches);
