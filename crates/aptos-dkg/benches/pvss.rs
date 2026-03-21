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
        MalleableTranscript, Transcript, TranscriptCore, WithMaxNumShares,
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
use std::collections::HashSet;

// const BN254: &str = "bn254";
const BLS12_381: &str = "bls12-381";

/// If set, only run these benchmark groups (comma-separated). Avoids expensive setup for groups
/// you're not running. Values: `unweighted_das`, `chunky_v1`, `chunky_v2`, `weighted_das`. Unset or `all` = run all.
fn enabled_bench_groups() -> Option<HashSet<String>> {
    let v = std::env::var("DKG_BENCH_GROUP").ok()?;
    let v = v.trim();
    if v.is_empty() || v.eq_ignore_ascii_case("all") {
        return None;
    }
    Some(
        v.split(',')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect(),
    )
}

fn group_enabled(groups: &Option<HashSet<String>>, name: &str) -> bool {
    match groups {
        None => true,
        Some(set) => set.contains(&name.to_ascii_lowercase()),
    }
}

pub fn all_groups(c: &mut Criterion) {
    println!("Rayon num threads: {}", rayon::current_num_threads());
    let enabled = enabled_bench_groups();

    // unweighted aggregatable PVSS, `blstrs` only so this is BLS12-381
    if group_enabled(&enabled, "unweighted_das") {
        for tc in get_threshold_configs_for_benchmarking() {
            aggregatable_pvss_group::<das::Transcript>(&tc, c);
        }
    }

    // weighted aggregatable PVSS, `blstrs` only so this is BLS12-381
    if group_enabled(&enabled, "weighted_das") {
        for wc in get_weighted_configs_for_benchmarking() {
            let d = aggregatable_pvss_group::<das::WeightedTranscript>(&wc, c);
            weighted_pvss_group(&wc, d, c);

            // Note: Insecure, so not interested in benchmarks.
            // let d = pvss_group::<GenericWeighting<pvss::das::Transcript>>(&wc, c);
            // weighted_pvss_group(&wc, d, c);
        }
    }

    // Chunky_v1 and Chunky_v2 share the same setup (config, keys, PP); run it once and reuse.
    let chunky_v1_enabled = group_enabled(&enabled, "chunky_v1");
    let chunky_v2_enabled = group_enabled(&enabled, "chunky_v2");
    if chunky_v1_enabled || chunky_v2_enabled {
        let configs: Vec<_> = get_weighted_configs_for_benchmarking()
            .into_iter()
            .take(1)
            .collect();
        for tc in &configs {
            let mut rng = StdRng::seed_from_u64(42);
            let ell = Some(32u8);
            // Single PP setup (table + dekart); keys/secrets generated once per variant.
            let (d1, d2) = test_utils::setup_dealing_chunky_both(tc, ell, &mut rng);
            if chunky_v1_enabled {
                subaggregatable_pvss_group_with_dealing::<Chunky_v1<Bls12_381>>(
                    tc, c, ell, BLS12_381, &d1,
                );
            }
            if chunky_v2_enabled {
                subaggregatable_pvss_group_with_dealing::<Chunky_v2<Bls12_381>>(
                    tc, c, ell, BLS12_381, &d2,
                );
            }
        }
    }
}

pub fn aggregatable_pvss_group<T: AggregatableTranscript + MalleableTranscript>(
    sc: &<T as TranscriptCore>::SecretSharingConfig,
    c: &mut Criterion,
) -> DealingArgs<T> {
    let name = T::scheme_name();
    let mut group = c.benchmark_group(format!("pvss/{}", name));
    let mut rng = StdRng::seed_from_u64(42);

    // TODO: use a lazy pattern to avoid this expensive step when no benchmarks are run
    let d = test_utils::setup_dealing::<T, _>(sc, None, &mut rng);

    // pvss_transcript_random::<T, WallTime>(sc, &mut group);
    pvss_deal::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_aggregate::<T, WallTime>(sc, &d.pp, &mut group);
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
                SecretSharingConfig = <T as TranscriptCore>::SecretSharingConfig,
            >,
        >,
{
    let mut rng = StdRng::seed_from_u64(42);
    let d = test_utils::setup_dealing::<T, _>(sc, ell, &mut rng);
    subaggregatable_pvss_group_with_dealing(sc, c, ell, curve_name, &d);
    d
}

/// Same benchmarks as `subaggregatable_pvss_group` but use pre-computed dealing args (e.g. shared with another chunky variant).
pub fn subaggregatable_pvss_group_with_dealing<T>(
    sc: &T::SecretSharingConfig,
    c: &mut Criterion,
    ell: Option<u8>,
    curve_name: &str,
    d: &DealingArgs<T>,
) where
    T: MalleableTranscript
        + HasAggregatableSubtranscript<
            Subtranscript: Aggregatable<
                SecretSharingConfig = <T as TranscriptCore>::SecretSharingConfig,
            >,
        >,
{
    let name = T::scheme_name();
    let group_name = match ell {
        Some(ell) => format!("pvss/{}/{}/{}", name, curve_name, ell),
        None => format!("pvss/{}/{}", name, curve_name),
    };
    let mut group = c.benchmark_group(group_name);

    pvss_deal::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_nonaggregate_serialize::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_subaggregate::<T, WallTime>(sc, &d.pp, &mut group);
    pvss_nonaggregate_verify::<T, WallTime>(sc, &d.pp, &d.ssks, &d.spks, &d.eks, &mut group);
    pvss_decrypt_own_share::<T, WallTime>(
        sc, &d.pp, &d.ssks, &d.spks, &d.dks, &d.eks, &d.s, &mut group,
    );

    group.finish();
}

pub fn weighted_pvss_group<
    T: AggregatableTranscript + MalleableTranscript<SecretSharingConfig = WeightedConfigBlstrs>,
>(
    sc: &<T as TranscriptCore>::SecretSharingConfig,
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
    sc: &<T as TranscriptCore>::SecretSharingConfig,
    pp: &T::PublicParameters,
    g: &mut BenchmarkGroup<M>,
) {
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));
    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("aggregate/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let trx = T::generate(&sc, &pp, &mut rng);
                (trx.clone(), trx)
            },
            |(first, second)| {
                let mut agg = first.to_aggregated();
                agg.aggregate_with(&sc, &second).unwrap();
            },
        )
    });
}

fn pvss_subaggregate<T, M: Measurement>(
    sc: &T::SecretSharingConfig,
    pp: &T::PublicParameters,
    g: &mut BenchmarkGroup<M>,
) where
    T: HasAggregatableSubtranscript<
        Subtranscript: Aggregatable<
            SecretSharingConfig = <T as TranscriptCore>::SecretSharingConfig,
        >,
    >,
{
    g.throughput(Throughput::Elements(sc.get_total_num_shares() as u64));
    let mut rng = StdRng::seed_from_u64(42);

    g.bench_function(format!("aggregate/{}", sc), move |b| {
        b.iter_with_setup(
            || {
                let trs = T::generate(&sc, &pp, &mut rng);
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
    sc: &<T as TranscriptCore>::SecretSharingConfig,
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
    let mut rng2 = StdRng::seed_from_u64(43);

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
                trx.verify(&sc, &pp, spks, &eks, &NoAux, &mut rng2)
                    .expect("PVSS transcript verification should succeed");
            },
        )
    });
}

fn pvss_aggregate_verify<T: AggregatableTranscript + MalleableTranscript, M: Measurement>(
    sc: &<T as TranscriptCore>::SecretSharingConfig,
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

    // TODO_1: the following code is obviously messy. Easiest fix is to extend `get_player_weight()`
    // to `SecretSharingConfig`
    // TODO_2: it is also inaccurate, it should be aggregating transactions first which means the
    // actual bit size of the decrypted shares is much higher!!
    g.bench_function(format!("decrypt-share/{}", sc), move |b| {
        // Pre-compute valid player indices by checking if get_public_key_share
        // returns non-empty results. For weighted transcripts, DealtPubKeyShare is Vec,
        // for unweighted it's a single value. We can't check weight generically since
        // get_player_weight is not (yet!) part of the SecretSharingConfig trait, so we
        // check if the share is non-empty by attempting to use it.
        let valid_players: Vec<usize> = (0..sc.get_total_num_players())
            .filter(|&i| {
                // Ensure player has a decryption key
                if dks.get(i).is_none() {
                    return false;
                }
                let player = sc.get_player(i);
                let pk_share = trx.get_public_key_share(&sc, &player);
                // For weighted configs, pk_share is Vec<...>, check if non-empty
                // For unweighted, it's always valid (single value)
                // We can't check this generically, so we use Debug formatting as a proxy
                // If the Debug representation is meaningful (not empty/default), assume valid
                let debug_str = format!("{:?}", pk_share);
                // Empty Vec would show as "[]", single values would show their content
                !debug_str.is_empty() && debug_str != "[]"
            })
            .collect();

        assert!(
            !valid_players.is_empty(),
            "No valid players found for benchmark"
        );

        b.iter_with_setup(
            || {
                let idx = rng.gen_range(0, valid_players.len());
                valid_players[idx]
            },
            |i| {
                black_box(trx.decrypt_own_share(&sc, &sc.get_player(i), &dks[i], pp));
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
    config = Criterion::default().sample_size(10);
    targets = all_groups);
criterion_main!(benches);
