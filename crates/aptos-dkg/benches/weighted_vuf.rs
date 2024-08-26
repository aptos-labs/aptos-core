// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::needless_range_loop)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::extra_unused_type_parameters)]
#![allow(clippy::needless_borrow)]

use aptos_dkg::{
    pvss::{
        das,
        dealt_secret_key::g1::DealtSecretKey,
        insecure_field,
        test_utils::{get_weighted_configs_for_benchmarking, setup_dealing, NoAux},
        traits::{SecretSharingConfig, Transcript},
        GenericWeighting, Player, WeightedConfig,
    },
    utils::random::{random_g1_points, random_g2_points},
    weighted_vuf::{
        bls,
        pinkas::{PinkasWUF, PublicParameters, RandomizedPKs},
        traits::WeightedVUF,
    },
};
use aptos_runtimes::spawn_rayon_thread_pool;
use blstrs::{G1Projective, G2Projective, Scalar};
use core::iter::zip;
use criterion::{
    black_box, criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, Criterion,
};
use rand::{rngs::ThreadRng, thread_rng};
use std::time::Instant;

const BENCH_MSG: &[u8; 36] = b"some dummy message for the benchmark";

pub fn all_groups(c: &mut Criterion) {
    let mut group = c.benchmark_group("wvuf/das-pinkas-sk-in-g1");
    let bench_cases = wvuf_benches::<das::WeightedTranscript, PinkasWUF>(&mut group);
    group.finish();

    let mut group = c.benchmark_group("wvuf/das-pinkas-sk-in-g1");
    pinkas_wvuf_derive_eval_micro_benches(bench_cases, &mut group);
    group.finish();

    let mut group = c.benchmark_group("wvuf/insecure-field-bls");
    wvuf_benches::<GenericWeighting<insecure_field::Transcript>, bls::BlsWUF>(&mut group);
    group.finish();
}

fn pinkas_wvuf_derive_eval_micro_benches(
    bench_cases: Vec<(
        WeightedConfig,
        PublicParameters,
        DealtSecretKey,
        <das::WeightedTranscript as Transcript>::DealtPubKey,
        Vec<<das::WeightedTranscript as Transcript>::DealtSecretKeyShare>,
        Vec<<das::WeightedTranscript as Transcript>::DealtPubKeyShare>,
        Vec<(
            Scalar,
            <das::WeightedTranscript as Transcript>::DealtSecretKeyShare,
        )>,
        Vec<(
            RandomizedPKs,
            <das::WeightedTranscript as Transcript>::DealtPubKeyShare,
        )>,
        Vec<RandomizedPKs>,
    )>,
    mut group: &mut BenchmarkGroup<WallTime>,
) {
    let mut max_cpus = num_cpus::get();
    println!("Max # of threads: {}", max_cpus);
    let mut num_threads_cases = vec![1usize];
    while max_cpus > 1 {
        num_threads_cases.push(num_threads_cases.last().unwrap() * 2);
        max_cpus /= 2;
    }
    println!("Benchmarking with num threads in {:?}", num_threads_cases);

    let mut rng = thread_rng();

    // bench_cases.reverse();
    for (wc, _vuf_pp, _sk, _pk, _sks, _pks, asks, apks, _deltas) in bench_cases {
        let worst_case_size = wc.get_worst_case_eligible_subset_of_players(&mut rng).len();
        println!("Worst case size for {} is {}", wc, worst_case_size);
        pinkas_wvuf_derive_eval_collect_lagrange_shares_and_rks(
            &wc,
            &asks,
            &apks,
            &mut group,
            &mut rng,
            WeightedConfig::get_worst_case_eligible_subset_of_players,
            "worst_case",
        );

        for num_threads in &num_threads_cases {
            pinkas_wvuf_derive_eval_rks_multiexps(
                &wc,
                &asks,
                &apks,
                &mut group,
                &mut rng,
                WeightedConfig::get_worst_case_eligible_subset_of_players,
                "worst_case",
                *num_threads,
            );

            pinkas_wvuf_derive_eval_multipairing(
                &wc,
                &mut group,
                &mut rng,
                WeightedConfig::get_worst_case_eligible_subset_of_players,
                "worst_case",
                *num_threads,
            );
        }
    }
}

pub fn wvuf_benches<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
>(
    group: &mut BenchmarkGroup<WallTime>,
) -> Vec<(
    WeightedConfig,
    <WVUF as WeightedVUF>::PublicParameters,
    <WT as Transcript>::DealtSecretKey,
    <WT as Transcript>::DealtPubKey,
    Vec<<WT as Transcript>::DealtSecretKeyShare>,
    Vec<<WT as Transcript>::DealtPubKeyShare>,
    Vec<<WVUF as WeightedVUF>::AugmentedSecretKeyShare>,
    Vec<<WVUF as WeightedVUF>::AugmentedPubKeyShare>,
    Vec<<WVUF as WeightedVUF>::Delta>,
)>
where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    let mut rng = thread_rng();

    let mut bench_cases = vec![];
    for wc in get_weighted_configs_for_benchmarking() {
        // TODO: use a lazy pattern to avoid this expensive dealing when no benchmarks are run
        let d = setup_dealing::<WT, ThreadRng>(&wc, &mut rng);

        println!(
            "Best-case subset size: {}",
            wc.get_best_case_eligible_subset_of_players(&mut rng).len()
        );
        println!(
            "Worst-case subset size: {}",
            wc.get_worst_case_eligible_subset_of_players(&mut rng).len()
        );

        println!("Dealing a {} PVSS transcript", WT::scheme_name());
        let trx = WT::deal(
            &wc,
            &d.pp,
            &d.ssks[0],
            &d.eks,
            &d.iss[0],
            &NoAux,
            &wc.get_player(0),
            &mut rng,
        );

        let vuf_pp = WVUF::PublicParameters::from(&d.pp);

        let mut sks = vec![];
        let mut pks = vec![];
        let mut deltas = vec![];
        let mut asks = vec![];
        let mut apks = vec![];
        println!(
            "Decrypting shares from {} PVSS transcript",
            WT::scheme_name()
        );
        for i in 0..wc.get_total_num_players() {
            let (sk, pk) = trx.decrypt_own_share(&wc, &wc.get_player(i), &d.dks[i]);
            let (ask, apk) = WVUF::augment_key_pair(&vuf_pp, sk.clone(), pk.clone(), &mut rng);

            sks.push(sk);
            pks.push(pk);
            deltas.push(WVUF::get_public_delta(&apk).clone());
            asks.push(ask);
            apks.push(apk);
        }

        println!();

        bench_cases.push((wc, vuf_pp, d.dsk, d.dpk, sks, pks, asks, apks, deltas));
    }

    for (wc, vuf_pp, sk, pk, sks, pks, asks, apks, deltas) in &bench_cases {
        wvuf_augment_random_keypair::<WT, WVUF, ThreadRng>(
            &wc, &vuf_pp, &sks, &pks, group, &mut rng,
        );

        wvuf_augment_all_pubkeys::<WT, WVUF, ThreadRng>(&wc, &vuf_pp, &pks, &deltas, group);

        wvuf_augment_random_pubkey::<WT, WVUF, ThreadRng>(
            &wc, &vuf_pp, &pks, &deltas, group, &mut rng,
        );

        wvuf_create_share_random::<WT, WVUF, ThreadRng>(&wc, &asks, group, &mut rng);
        wvuf_create_share_average::<WT, WVUF, ThreadRng>(&wc, &asks, group);

        let min_weight_player = wc.get_min_weight_player();
        wvuf_create_share_specific::<WT, WVUF, ThreadRng>(
            &wc,
            &asks,
            group,
            &min_weight_player,
            "min-weight",
        );
        let max_weight_player = wc.get_max_weight_player();
        wvuf_create_share_specific::<WT, WVUF, ThreadRng>(
            &wc,
            &asks,
            group,
            &max_weight_player,
            "max-weight",
        );

        // TODO: should change WVUF trait to support some kind of multi-threaded share verification,
        //  since in practice that's what we would do on the validators
        //  i.e., https://github.com/aptos-labs/aptos-core/blob/8ff40c8dd6505dea5e4b2a28cbbe7b97723b0ec2/consensus/src/rand/rand_gen/rand_manager.rs#L221
        wvuf_verify_share_random::<WT, WVUF, ThreadRng>(
            &wc, &vuf_pp, &asks, &apks, group, &mut rng,
        );
        wvuf_verify_share_average::<WT, WVUF, ThreadRng>(&wc, &vuf_pp, &asks, &apks, group);
        wvuf_verify_share_specific::<WT, WVUF, ThreadRng>(
            &wc,
            &vuf_pp,
            &asks,
            &apks,
            &min_weight_player,
            "min-weight",
            group,
        );
        wvuf_verify_share_specific::<WT, WVUF, ThreadRng>(
            &wc,
            &vuf_pp,
            &asks,
            &apks,
            &max_weight_player,
            "max-weight",
            group,
        );

        // benchmarks the sequence of WVUF::verify_share calls on shares from a specific subset of players
        let bc: Vec<(fn(&WeightedConfig, &mut ThreadRng) -> Vec<Player>, String)> = vec![
            (
                WeightedConfig::get_random_eligible_subset_of_players,
                "random".to_string(),
            ),
            (
                WeightedConfig::get_best_case_eligible_subset_of_players,
                "best_case".to_string(),
            ),
            (
                WeightedConfig::get_worst_case_eligible_subset_of_players,
                "worst_case".to_string(),
            ),
        ];

        for (pick_subset_fn, subset_type) in bc {
            wvuf_aggregate_shares::<WT, WVUF, ThreadRng>(
                &wc,
                &asks,
                &apks,
                group,
                &mut rng,
                pick_subset_fn,
                &subset_type,
            );

            wvuf_many_verify_shares::<WT, WVUF, ThreadRng>(
                &wc,
                &vuf_pp,
                &asks,
                &apks,
                pick_subset_fn,
                &subset_type,
                group,
                &mut rng,
            );

            wvuf_verify_proof::<WT, WVUF, ThreadRng>(
                &wc,
                &vuf_pp,
                &pk,
                &asks,
                &apks,
                group,
                &mut rng,
                pick_subset_fn,
                &subset_type,
            );

            for num_threads in [1, 2, 4, 8, 16, 32] {
                wvuf_derive_eval::<WT, WVUF, ThreadRng>(
                    &wc,
                    &vuf_pp,
                    &asks,
                    &apks,
                    group,
                    &mut rng,
                    pick_subset_fn,
                    &subset_type,
                    num_threads,
                );
            }
        }

        wvuf_eval::<WT, WVUF>(&wc, &sk, group);
    }

    bench_cases
}

fn wvuf_augment_random_keypair<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    sks: &Vec<WT::DealtSecretKeyShare>,
    pks: &Vec<WT::DealtPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_random_keypair/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(&mut thread_rng()).id;
                (sks[id].clone(), pks[id].clone())
            },
            |(sk, pk)| WVUF::augment_key_pair(vuf_pp, sk, pk, rng),
        )
    });
}

fn wvuf_augment_all_pubkeys<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    pks: &Vec<WVUF::PubKeyShare>,
    deltas: &Vec<WVUF::Delta>,
    group: &mut BenchmarkGroup<WallTime>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    assert_eq!(pks.len(), wc.get_total_num_players());
    assert_eq!(pks.len(), deltas.len());
    group.bench_function(format!("augment_all_pubkeys/{}", wc), move |b| {
        b.iter(|| {
            for (pk, delta) in zip(pks, deltas) {
                WVUF::augment_pubkey(vuf_pp, pk.clone(), delta.clone())
                    .expect("augmentation should have succeeded");
            }
        })
    });
}

fn wvuf_augment_random_pubkey<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    pks: &Vec<WVUF::PubKeyShare>,
    deltas: &Vec<WVUF::Delta>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("augment_random_pubkey/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                // Ugh, borrow checker...
                let id = wc.get_random_player(rng).id;
                let pk = pks[id].clone();
                let delta = deltas[id].clone();

                (pk, delta)
            },
            |(pk, delta)| WVUF::augment_pubkey(vuf_pp, pk, delta),
        )
    });
}

fn wvuf_create_share_random<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("create_share_random/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                &asks[player.id]
            },
            |ask| WVUF::create_share(ask, BENCH_MSG),
        )
    });
}

fn wvuf_create_share_specific<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    player: &Player,
    name: &str,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("create_share_specific/{}/{}", name, wc), move |b| {
        b.iter_with_setup(
            || &asks[player.id],
            |ask| WVUF::create_share(ask, BENCH_MSG),
        )
    });
}

fn wvuf_create_share_average<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("create_share_average/{}", wc), move |b| {
        let n = wc.get_total_num_players();
        b.iter_custom(|iters| {
            let shares: Vec<_> = (0..n)
                .map(|i| {
                    let player = wc.get_player(i);
                    &asks[player.id]
                })
                .collect();

            let start = Instant::now();
            for _i in 0..iters {
                black_box(
                    shares
                        .iter()
                        .map(|ask| {
                            WVUF::create_share(ask, BENCH_MSG);
                        })
                        .collect::<Vec<_>>(),
                );
            }
            let total_duration = start.elapsed();
            total_duration / (n as u32)
        })
    });
}

fn wvuf_verify_share_random<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("verify_share_random/{}", wc), move |b| {
        b.iter_with_setup(
            || {
                let player = wc.get_random_player(rng);
                let ask = &asks[player.id];

                (WVUF::create_share(ask, BENCH_MSG), &apks[player.id])
            },
            |(proof, apk)| WVUF::verify_share(vuf_pp, apk, BENCH_MSG, &proof),
        )
    });
}

fn wvuf_verify_share_average<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("verify_share_average/{}", wc), move |b| {
        let n = wc.get_total_num_players();
        b.iter_custom(|iters| {
            let shares: Vec<_> = (0..n)
                .map(|i| {
                    let player = wc.get_player(i);
                    let ask = &asks[player.id];
                    (WVUF::create_share(ask, BENCH_MSG), &apks[player.id])
                })
                .collect();

            let start = Instant::now();
            for _i in 0..iters {
                black_box(
                    shares
                        .iter()
                        .map(|(proof, apk)| WVUF::verify_share(vuf_pp, apk, BENCH_MSG, &proof))
                        .collect::<Vec<anyhow::Result<()>>>(),
                );
            }
            let total_duration = start.elapsed();
            total_duration / (n as u32)
        })
    });
}

fn wvuf_many_verify_shares<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
        PubKey = WT::DealtPubKey,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    name: &str,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("many_verify_shares/{}/{}", name, wc), move |b| {
        b.iter_with_setup(
            || get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn),
            |apks_and_proofs| {
                for (_, apk, proof) in apks_and_proofs {
                    WVUF::verify_share(vuf_pp, &apk, BENCH_MSG, &proof).unwrap();
                }
            },
        )
    });
}

fn wvuf_verify_share_specific<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    wc: &WeightedConfig,
    vuf_pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    player: &Player,
    name: &str,
    group: &mut BenchmarkGroup<WallTime>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    println!("Player weight: {:?}", wc.get_player_weight(player));
    let ask = &asks[player.id];
    let apk = &apks[player.id];
    group.bench_function(format!("verify_share_specific/{}/{}", name, wc), move |b| {
        b.iter_with_setup(
            || WVUF::create_share(ask, BENCH_MSG),
            |proof| WVUF::verify_share(vuf_pp, apk, BENCH_MSG, &proof),
        )
    });
}

fn wvuf_aggregate_shares<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(
        format!("aggregate_shares/{}-subset/{}", subset_type, wc),
        move |b| {
            b.iter_with_setup(
                || get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn),
                |apks_and_proofs| {
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice());
                },
            )
        },
    );
}

fn wvuf_verify_proof<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    pp: &WVUF::PublicParameters,
    pk: &WVUF::PubKey,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(
        format!("verify_proof/{}-subset/{}", subset_type, wc),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs =
                        get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn);
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice())
                },
                |proof| {
                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();
                    assert!(WVUF::verify_proof(pp, pk, apks.as_slice(), BENCH_MSG, &proof).is_ok())
                },
            )
        },
    );
}

fn wvuf_derive_eval<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    pp: &WVUF::PublicParameters,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &String,
    num_threads: usize,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    let pool = spawn_rayon_thread_pool("bench-wvuf".to_string(), Some(num_threads));

    group.bench_function(
        format!(
            "derive_eval/{}-subset/{}-thread/{}",
            subset_type, num_threads, wc
        ),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs =
                        get_apks_and_proofs::<WT, WVUF, R>(&wc, &asks, apks, rng, pick_subset_fn);
                    WVUF::aggregate_shares(&wc, apks_and_proofs.as_slice())
                },
                |proof| {
                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<WVUF::AugmentedPubKeyShare>>>();
                    assert!(
                        WVUF::derive_eval(wc, pp, BENCH_MSG, apks.as_slice(), &proof, &pool)
                            .is_ok()
                    )
                },
            )
        },
    );
}

fn pinkas_wvuf_derive_eval_collect_lagrange_shares_and_rks<
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<<PinkasWUF as WeightedVUF>::AugmentedSecretKeyShare>,
    apks: &Vec<<PinkasWUF as WeightedVUF>::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &str,
) {
    group.bench_function(
        format!("derive_eval_lagr/{}-subset/{}", subset_type, wc),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs = get_apks_and_proofs::<
                        das::WeightedTranscript,
                        PinkasWUF,
                        R,
                    >(
                        &wc, &asks, apks, rng, pick_subset_fn
                    );
                    PinkasWUF::aggregate_shares(&wc, apks_and_proofs.as_slice())
                },
                |proof| {
                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<<PinkasWUF as WeightedVUF>::AugmentedPubKeyShare>>>();

                    assert!(
                        PinkasWUF::collect_lagrange_coeffs_shares_and_rks(wc, &apks, &proof)
                            .is_ok()
                    );
                },
            )
        },
    );
}

fn pinkas_wvuf_derive_eval_rks_multiexps<R: rand_core::RngCore + rand_core::CryptoRng>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<<PinkasWUF as WeightedVUF>::AugmentedSecretKeyShare>,
    apks: &Vec<<PinkasWUF as WeightedVUF>::AugmentedPubKeyShare>,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &str,
    num_threads: usize,
) {
    let pool = spawn_rayon_thread_pool("bench-wvuf".to_string(), Some(num_threads));

    group.bench_function(
        format!(
            "derive_eval_multiexps/{}-subset/{}-thread/{}",
            subset_type, num_threads, wc
        ),
        move |b| {
            b.iter_with_setup(
                || {
                    let apks_and_proofs = get_apks_and_proofs::<
                        das::WeightedTranscript,
                        PinkasWUF,
                        R,
                    >(
                        &wc, &asks, apks, rng, pick_subset_fn
                    );

                    let apks = apks
                        .iter()
                        .map(|apk| Some(apk.clone()))
                        .collect::<Vec<Option<<PinkasWUF as WeightedVUF>::AugmentedPubKeyShare>>>();

                    let proof = PinkasWUF::aggregate_shares(&wc, apks_and_proofs.as_slice());
                    let (_, rks, lagr, ranges) = PinkasWUF::collect_lagrange_coeffs_shares_and_rks(
                        wc,
                        apks.as_slice(),
                        &proof,
                    )
                    .unwrap();

                    (
                        proof.clone(),
                        rks.into_iter().cloned().collect::<Vec<Vec<G1Projective>>>(),
                        lagr,
                        ranges,
                    )
                },
                |(proof, rks, lagr, ranges)| {
                    let rks = rks.iter().collect::<Vec<&Vec<G1Projective>>>();
                    let _ = PinkasWUF::rk_multiexps(&proof, rks, &lagr, &ranges, &pool);
                },
            )
        },
    );
}

fn pinkas_wvuf_derive_eval_multipairing<R: rand_core::RngCore + rand_core::CryptoRng>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    group: &mut BenchmarkGroup<WallTime>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
    subset_type: &str,
    num_threads: usize,
) {
    let pool = spawn_rayon_thread_pool("bench-wvuf".to_string(), Some(num_threads));
    let n = pick_subset_fn(wc, rng).len();

    group.bench_function(
        format!(
            "derive_eval_multipairing/{}-subset/{}-thread/{}",
            subset_type, num_threads, wc
        ),
        move |b| {
            b.iter_with_setup(
                || {
                    let g1 = random_g1_points(n, rng);
                    let g2 = random_g2_points(n, rng);

                    (g1, g2)
                },
                |(vec_g1, vec_g2)| {
                    let vec_ref_g2 = vec_g2.iter().collect::<Vec<&G2Projective>>();
                    let _ = PinkasWUF::multi_pairing(vec_g1, vec_ref_g2, &pool);
                },
            )
        },
    );
}

fn get_apks_and_proofs<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKey = WT::DealtPubKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
    R: rand_core::RngCore + rand_core::CryptoRng,
>(
    // For efficiency, we re-use the PVSS transcript
    wc: &WeightedConfig,
    asks: &Vec<WVUF::AugmentedSecretKeyShare>,
    apks: &Vec<WVUF::AugmentedPubKeyShare>,
    rng: &mut R,
    pick_subset_fn: fn(&WeightedConfig, &mut R) -> Vec<Player>,
) -> Vec<(Player, WVUF::AugmentedPubKeyShare, WVUF::ProofShare)> {
    let players = pick_subset_fn(wc, rng);

    players
        .iter()
        .map(|p| {
            (
                *p,
                apks[p.id].clone(),
                WVUF::create_share(&asks[p.id], BENCH_MSG),
            )
        })
        .collect::<Vec<(Player, WVUF::AugmentedPubKeyShare, WVUF::ProofShare)>>()
}

fn wvuf_eval<
    WT: Transcript<SecretSharingConfig = WeightedConfig>,
    WVUF: WeightedVUF<
        SecretKey = WT::DealtSecretKey,
        PubKeyShare = WT::DealtPubKeyShare,
        SecretKeyShare = WT::DealtSecretKeyShare,
    >,
>(
    wc: &WeightedConfig,
    sk: &WVUF::SecretKey,
    group: &mut BenchmarkGroup<WallTime>,
) where
    WVUF::PublicParameters: for<'a> From<&'a WT::PublicParameters>,
{
    group.bench_function(format!("eval/{}", wc), move |b| {
        b.iter_with_setup(|| {}, |_| WVUF::eval(sk, BENCH_MSG))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    //config = Criterion::default();
    targets = all_groups);
criterion_main!(benches);
