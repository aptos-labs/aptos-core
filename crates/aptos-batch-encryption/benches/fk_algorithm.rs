// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_batch_encryption::group::{Fr, G1Projective};
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
use aptos_batch_encryption::{shared::digest::DigestKey, shared::algebra::fk_algorithm::FKDomain};
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ark_std::{rand::thread_rng, UniformRand};




pub fn eval_proofs_at_roots_of_unity(c: &mut Criterion) {
    let mut group = c.benchmark_group("FKDomain::eval_proofs_at_roots_of_unity");

    for poly_degree in [8, 32, 128, 512 ] {
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, poly_degree, 1).unwrap();
        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
            vec![Fr::rand(&mut rng); poly_degree + 1 ]
        );
        let tau_powers_projective : Vec<Vec<G1Projective>> = setup.tau_powers_g1.iter().map(
            |gs| gs.into_iter().map(
                |g| G1Projective::from(*g)
            ).collect::<Vec<G1Projective>>()
        ).collect();
        let fk_domain = FKDomain::new(poly_degree, poly_degree, tau_powers_projective).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, poly), |b, input| {
            b.iter(||
                input.0.eval_proofs_at_roots_of_unity(&input.1.coeffs, 0)
                );
        });
    }
}

pub fn eval_proofs_at_x_coords(c: &mut Criterion) {
    let mut group = c.benchmark_group("FKDomain::eval_proofs_at_x_coords");

    for poly_degree in [8, 32, 128, 512 ] {
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, poly_degree, 1).unwrap();
        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
            vec![Fr::rand(&mut rng); poly_degree + 1 ]
        );
        let x_coords = vec![Fr::rand(&mut rng); poly_degree  ];

        let tau_powers_projective : Vec<Vec<G1Projective>> = setup.tau_powers_g1.iter().map(
            |gs| gs.into_iter().map(
                |g| G1Projective::from(*g)
            ).collect::<Vec<G1Projective>>()
        ).collect();
        let fk_domain = FKDomain::new(poly_degree, poly_degree, tau_powers_projective).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, poly, x_coords), |b, input| {
            b.iter(||
                input.0.eval_proofs_at_x_coords(&input.1.coeffs, &input.2, 0)
                );
        });
    }
}


criterion_group!(benches, eval_proofs_at_roots_of_unity, eval_proofs_at_x_coords);
criterion_main!(benches);

