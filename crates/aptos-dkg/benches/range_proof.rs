// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_dkg::{
    range_proofs::{
        dekart_univariate::{fiat_shamir_challenges, Proof as UnivariateDeKART, PublicStatement},
        dekart_univariate_v2::Proof as UnivariateDeKARTv2,
        traits::BatchedRangeProof,
    },
    utils::test_utils,
};
use ark_ec::{
    pairing::{Pairing, PairingOutput},
    AdditiveGroup, VariableBaseMSM,
};
use ark_std::rand::thread_rng;
use criterion::{criterion_group, criterion_main, Criterion};

/// Generic benchmark function over any pairing curve
fn bench_range_proof<E: Pairing, B: BatchedRangeProof<E>>(c: &mut Criterion, curve_name: &str) {
    let mut group = c.benchmark_group(format!("range_proof/{}", curve_name));

    let ell = std::env::var("L")
        .unwrap_or(std::env::var("ELL").unwrap_or_default())
        .parse::<usize>()
        .unwrap_or(16); // 48

    let n = std::env::var("N")
        .unwrap_or_default()
        .parse::<usize>()
        .unwrap_or(1023); // 2048 - 1

    group.bench_function(format!("prove/ell={ell}/n={n}").as_str(), move |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let (pk, _, values, comm, comm_r) =
                    test_utils::range_proof_random_instance::<_, B, _>(n, ell, &mut rng);
                (pk, values, comm, comm_r)
            },
            |(pk, values, comm, r)| {
                let mut fs_t = merlin::Transcript::new(B::DST);
                let mut rng = thread_rng();
                let _proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
            },
        )
    });

    group.bench_function(format!("verify/ell={ell}/n={n}").as_str(), |b| {
        b.iter_with_setup(
            || {
                let mut rng = thread_rng();
                let (pk, vk, values, comm, r) =
                    test_utils::range_proof_random_instance::<_, B, _>(n, ell, &mut rng);
                let mut fs_t = merlin::Transcript::new(B::DST);
                let proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
                (vk, n, ell, comm, proof)
            },
            |(vk, n, ell, comm, proof)| {
                let mut fs_t = merlin::Transcript::new(B::DST);
                proof.verify(&vk, n, ell, &comm, &mut fs_t).unwrap();
            },
        )
    });
}

// Specialize benchmark for a concrete pairing curve
fn bench_groups(c: &mut Criterion) {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;

    //    bench_range_proof::<Bn254, UnivariateDeKART<Bn254>>(c, "BN254");
    //    bench_range_proof::<Bls12_381, UnivariateDeKART<Bls12_381>>(c, "BLS12-381");

    bench_range_proof::<Bn254, UnivariateDeKARTv2<Bn254>>(c, "BN254");
    bench_range_proof::<Bls12_381, UnivariateDeKARTv2<Bls12_381>>(c, "BLS12-381");

    // bench_verify_components::<Bn254>(c, "BN254");
    // bench_verify_components::<Bls12_381>(c, "BLS12-381");

    //run_param_sweep::<Bn254, UnivariateDeKARTv2<Bn254>>("BN254");
    //run_param_sweep::<Bls12_381, UnivariateDeKARTv2<Bls12_381>>("BLS12-381");

    // Sweep parameters for BLS12-381, DeKART v2
    //    run_param_sweep::<Bls12_381, UnivariateDeKARTv2<Bls12_381>>("BLS12-381-v2");
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_groups
);
criterion_main!(benches);

use prettytable::{Cell, Row, Table};
use std::time::Instant;

fn run_param_sweep<E: Pairing, B: BatchedRangeProof<E>>(curve_name: &str) {
    let ells = [4, 8, 16, 32];
    let ns = [1, 3, 7, 15, 31, 63, 127, 255, 511, 1023, 2047];
    let num_runs = 10;

    // store results in microseconds (averaged)
    let mut prove_times = vec![vec![0u128; ns.len()]; ells.len()];
    let mut verify_times = vec![vec![0u128; ns.len()]; ells.len()];

    for (i, &ell) in ells.iter().enumerate() {
        for (j, &n) in ns.iter().enumerate() {
            let mut prove_sum = 0u128;
            let mut verify_sum = 0u128;

            for _ in 0..num_runs {
                let mut rng = thread_rng();
                let (pk, vk, values, comm, r) =
                    test_utils::range_proof_random_instance::<_, B, _>(n, ell, &mut rng);

                // --- Prove ---
                let mut fs_t = merlin::Transcript::new(B::DST);
                let start = Instant::now();
                let proof = B::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
                prove_sum += (start.elapsed().as_secs_f64() * 1_000_000.0) as u128; // µs

                // --- Verify ---
                let mut fs_t = merlin::Transcript::new(B::DST);
                let start = Instant::now();
                proof.verify(&vk, n, ell, &comm, &mut fs_t).unwrap();
                verify_sum += (start.elapsed().as_secs_f64() * 1_000_000.0) as u128;
                // µs
            }

            prove_times[i][j] = prove_sum / num_runs as u128;
            verify_times[i][j] = verify_sum / num_runs as u128;
        }
    }

    // Print tables
    print_time_table_microseconds(
        format!("{curve_name} - Prove (ms)"),
        &ells,
        &ns,
        &prove_times,
    );
    print_time_table_microseconds(
        format!("{curve_name} - Verify (ms)"),
        &ells,
        &ns,
        &verify_times,
    );

    // Save CSVs
    let base_name = curve_name.replace('/', "_").replace(' ', "_");
    save_table_to_csv(
        &format!("{}_prove.csv", base_name),
        &ells,
        &ns,
        &prove_times,
    )
    .unwrap();
    save_table_to_csv(
        &format!("{}_verify.csv", base_name),
        &ells,
        &ns,
        &verify_times,
    )
    .unwrap();
}

fn print_time_table_microseconds(title: String, ells: &[usize], ns: &[usize], data: &[Vec<u128>]) {
    let mut table = Table::new();
    let mut header = vec![Cell::new("ℓ \\ n")];
    header.extend(ns.iter().map(|n| Cell::new(&n.to_string())));
    table.add_row(Row::new(header));

    for (i, &ell) in ells.iter().enumerate() {
        let mut row = vec![Cell::new(&ell.to_string())];
        // divide µs by 1000.0 to print milliseconds with 2 decimals
        row.extend(
            data[i]
                .iter()
                .map(|v| Cell::new(&format!("{:.2}", *v as f64 / 1000.0))),
        );
        table.add_row(Row::new(row));
    }

    println!("\n=== {title} ===");
    table.printstd();
}

use csv::Writer;

fn save_table_to_csv(
    filename: &str,
    ells: &[usize],
    ns: &[usize],
    data: &[Vec<u128>],
) -> csv::Result<()> {
    let mut wtr = Writer::from_path(filename)?;

    // Write header: first column label + n values
    let mut header = vec!["ell/n".to_string()];
    header.extend(ns.iter().map(|n| n.to_string()));
    wtr.write_record(header)?;

    // Each row: ell value + timings for each n
    for (i, &ell) in ells.iter().enumerate() {
        let mut row = vec![ell.to_string()];
        row.extend(data[i].iter().map(|v| v.to_string())); // store µs
        wtr.write_record(row)?;
    }

    wtr.flush()?;
    println!("→ Saved CSV: {filename}");
    Ok(())
}

fn save_table_to_csv_ms(
    filename: &str,
    ells: &[usize],
    ns: &[usize],
    data: &[Vec<u128>],
) -> csv::Result<()> {
    let mut wtr = Writer::from_path(filename)?;

    // Header
    let mut header = vec!["ell/n".to_string()];
    header.extend(ns.iter().map(|n| n.to_string()));
    wtr.write_record(header)?;

    // Each row: ℓ followed by formatted milliseconds
    for (i, &ell) in ells.iter().enumerate() {
        let mut row = vec![ell.to_string()];
        row.extend(
            data[i].iter().map(|v| format!("{:.2}", *v as f64 / 1000.0)), // convert µs → ms with decimals
        );
        wtr.write_record(row)?;
    }

    wtr.flush()?;
    println!("→ Saved CSV: {filename}");
    Ok(())
}

// fn bench_verify_components<E: Pairing>(c: &mut Criterion, curve_name: &str) {
//     let mut group = c.benchmark_group(format!("range_proof_components/{}", curve_name));

//     let ell = std::env::var("L")
//         .unwrap_or(std::env::var("ELL").unwrap_or_default())
//         .parse::<usize>()
//         .unwrap_or(48);

//     let n = std::env::var("N")
//         .unwrap_or_default()
//         .parse::<usize>()
//         .unwrap_or(2048 - 1);

//     // --- Full verify benchmark ---
//     group.bench_function(format!("verify_components/ell={ell}/n={n}"), |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 (vk, comm, proof)
//             },
//             |(vk, comm, proof)| {
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 proof.verify(&vk, n, ell, &comm, &mut fs_t).unwrap();
//             },
//         )
//     });

//     group.finish();

//     // --- Sub-step benchmarks in separate group ---
//     let mut sub_group = c.benchmark_group(format!("verify_components_substeps/{}", curve_name));

//     // 1. Recompute commitment
//     sub_group.bench_function("recompute_commitment", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 (vk, comm, proof)
//             },
//             |(vk, comm, proof)| {
//                 let commitment_recomputed =
//                     VariableBaseMSM::msm(&proof.c, &vk.powers_of_two[..ell]).unwrap();
//                 assert_eq!(comm.0, commitment_recomputed);
//             },
//         )
//     });

//     // 2. Fiat–Shamir challenges
//     sub_group.bench_function("fiat_shamir_challenges", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 (vk, comm, proof, fs_t)
//             },
//             |(vk, comm, proof, mut fs_t)| {
//                 let public_statement = PublicStatement {
//                     n,
//                     ell,
//                     comm: comm.clone(),
//                 };
//                 let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
//                 let (_alphas, _betas) = fiat_shamir_challenges(
//                     &vk,
//                     public_statement,
//                     &bit_commitments,
//                     proof.c.len(),
//                     &mut fs_t,
//                 );
//             },
//         )
//     });

//     // 3. h(τ) pairing check
//     sub_group.bench_function("pairing_h_tau", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let public_statement = PublicStatement {
//                     n,
//                     ell,
//                     comm: comm.clone(),
//                 };
//                 let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
//                 let (_alphas, betas) = fiat_shamir_challenges(
//                     &vk,
//                     public_statement,
//                     &bit_commitments,
//                     proof.c.len(),
//                     &mut fs_t,
//                 );
//                 (vk, proof, betas)
//             },
//             |(vk, proof, betas)| {
//                 let h_check = E::multi_pairing(
//                     (0..ell)
//                         .map(|j| proof.c[j] * betas[j])
//                         .chain(std::iter::once(-proof.d))
//                         .collect::<Vec<_>>(),
//                     (0..ell)
//                         .map(|j| proof.c_hat[j] - vk.tau_2)
//                         .chain(std::iter::once(vk.vanishing_com))
//                         .collect::<Vec<_>>(),
//                 );
//                 assert_eq!(PairingOutput::<E>::ZERO, h_check);
//             },
//         )
//     });

//     // 4. Duality MSM checks
//     sub_group.bench_function("msm_g1_duality", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 let public_statement = PublicStatement {
//                     n,
//                     ell,
//                     comm: comm.clone(),
//                 };
//                 let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
//                 let (alphas, _betas) = fiat_shamir_challenges(
//                     &vk,
//                     public_statement,
//                     &bit_commitments,
//                     proof.c.len(),
//                     &mut fs_t,
//                 );
//                 (vk, proof, alphas)
//             },
//             |(_vk, proof, alphas)| {
//                 let _g1_comb: E::G1 = VariableBaseMSM::msm(&proof.c, &alphas).unwrap();
//             },
//         )
//     });

//     sub_group.bench_function("msm_g2_duality", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 let public_statement = PublicStatement {
//                     n,
//                     ell,
//                     comm: comm.clone(),
//                 };
//                 let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
//                 let (alphas, _betas) = fiat_shamir_challenges(
//                     &vk,
//                     public_statement,
//                     &bit_commitments,
//                     proof.c.len(),
//                     &mut fs_t,
//                 );
//                 (vk, proof, alphas)
//             },
//             |(_vk, proof, alphas)| {
//                 let _g2_comb: E::G2 = VariableBaseMSM::msm(&proof.c_hat, &alphas).unwrap();
//             },
//         )
//     });

//     sub_group.bench_function("pairing_c_check", |b| {
//         b.iter_with_setup(
//             || {
//                 let mut rng = thread_rng();
//                 let (pk, vk, values, comm, r) =
//                     test_utils::range_proof_random_instance::<_, UnivariateDeKART<E>, _>(
//                         n, ell, &mut rng,
//                     );
//                 let mut fs_t = merlin::Transcript::new(UnivariateDeKART::<E>::DST);
//                 let proof =
//                     UnivariateDeKART::<E>::prove(&pk, &values, ell, &comm, &r, &mut fs_t, &mut rng);
//                 let public_statement = PublicStatement {
//                     n,
//                     ell,
//                     comm: comm.clone(),
//                 };
//                 let bit_commitments = (&proof.c[..], &proof.c_hat[..]);
//                 let (alphas, _betas) = fiat_shamir_challenges(
//                     &vk,
//                     public_statement,
//                     &bit_commitments,
//                     proof.c.len(),
//                     &mut fs_t,
//                 );
//                 (vk, proof, alphas)
//             },
//             |(vk, proof, alphas)| {
//                 let _g1_comb = VariableBaseMSM::msm(&proof.c, &alphas).unwrap();
//                 let _g2_comb = VariableBaseMSM::msm(&proof.c_hat, &alphas).unwrap();
//                 let _c_check =
//                     E::multi_pairing(vec![_g1_comb, -vk.tau_1], vec![vk.tau_2, _g2_comb]);
//             },
//         )
//     });
// }
