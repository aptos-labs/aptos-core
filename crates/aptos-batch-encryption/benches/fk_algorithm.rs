// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//use batch_ibe::group::{Fr, G1Projective};
//use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, EvaluationDomain, Radix2EvaluationDomain};
//use batch_ibe::{shared::digest::DigestKey, shared::algebra::fk_algorithm::FKDomain};
//use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
//use ark_std::{rand::thread_rng, UniformRand, Zero};
//use rayon::iter::{IntoParallelIterator, ParallelIterator};
//
//
//
//
//pub fn eval_proofs_at_roots_of_unity(c: &mut Criterion) {
//    let mut group = c.benchmark_group("FKDomain::eval_proofs_at_roots_of_unity");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, poly), |b, input| {
//            b.iter(||
//                input.0.eval_proofs_at_roots_of_unity(&input.1.coeffs)
//                );
//        });
//    }
//}
//
//pub fn eval_proofs_at_x_coords(c: &mut Criterion) {
//    let mut group = c.benchmark_group("FKDomain::eval_proofs_at_x_coords");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let x_coords = vec![Fr::rand(&mut rng); poly_degree  ];
//
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, poly, x_coords), |b, input| {
//            b.iter(||
//                input.0.eval_proofs_at_x_coords(&input.1.coeffs, &input.2)
//                );
//        });
//    }
//}
//
//pub fn toeplitz_for_poly(c: &mut Criterion) {
//    let mut group = c.benchmark_group("FKDomain::toeplitz_for_poly");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, f), |b, input| {
//            b.iter(|| {
//                input.0.toeplitz_for_poly(&input.1)
//            }
//            );
//        });
//    }
//}
//
//
//pub fn toeplitz_eval_prepared(c: &mut Criterion) {
//    let mut group = c.benchmark_group("ToeplitzDomain::eval_prepared");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//        let toeplitz = fk_domain.toeplitz_for_poly(&f);
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, toeplitz), |b, input| {
//            b.iter(|| {
//                input.0.toeplitz_domain.eval_prepared(&input.1, &input.0.prepared_toeplitz_input);
//            }
//            );
//        });
//    }
//}
//
//pub fn toeplitz_to_circulant(c: &mut Criterion) {
//    let mut group = c.benchmark_group("ToeplitzDomain::toeplitz_to_circulant");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//        let toeplitz = fk_domain.toeplitz_for_poly(&f);
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, toeplitz), |b, input| {
//            b.iter(|| {
//                input.0.toeplitz_domain.toeplitz_to_circulant(&input.1);
//            }
//            );
//        });
//    }
//}
//
//
//pub fn circulant_eval_prepared(c: &mut Criterion) {
//    let mut group = c.benchmark_group("CirculantDomain::eval_prepared");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//        let toeplitz = fk_domain.toeplitz_for_poly(&f);
//        let toeplitz_domain = fk_domain.toeplitz_domain;
//        let circulant = toeplitz_domain.toeplitz_to_circulant(&toeplitz);
//        let prepared_input = &fk_domain.prepared_toeplitz_input;
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(toeplitz_domain, circulant, prepared_input), |b, input| {
//            b.iter(|| {
//                input.0.circulant_domain.eval_prepared(&input.1, &input.2);
//            }
//            );
//        });
//    }
//}
//
//pub fn fft(c: &mut Criterion) {
//    let mut group = c.benchmark_group("Radix2EvaluationDomain::fft");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//        let toeplitz = fk_domain.toeplitz_for_poly(&f);
//        let h_term_commitments = fk_domain.toeplitz_domain.eval_prepared(&toeplitz, &fk_domain.prepared_toeplitz_input);
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fk_domain, h_term_commitments), |b, input| {
//            b.iter(|| {
//                input.0.fft_domain.fft(&input.1);
//            }
//            );
//        });
//    }
//}
//
//pub fn fft_tree(c: &mut Criterion) {
//    let mut group = c.benchmark_group("Radix2EvaluationDomain::fft_tree");
//
//    for poly_degree in [8, 32, 128, 512 ] {
//        //let mut rng = thread_rng();
//        //let random_group_elts = vec![ G1Projective::rand(&mut rng); poly_degree ];
//        let mut rng = thread_rng();
//        let setup = DigestKey::new(&mut rng, poly_degree).unwrap();
//        let poly : DensePolynomial<Fr> = DensePolynomial::from_coefficients_vec(
//            vec![Fr::rand(&mut rng); poly_degree + 1 ]
//        );
//        let tau_powers_projective : Vec<G1Projective> = setup.tau_powers_g1.iter().map(|g| G1Projective::from(*g)).collect();
//        let fk_domain = FKDomain::new(poly_degree, poly_degree, &tau_powers_projective).unwrap();
//
//        let mut f = Vec::from(poly.coeffs);
//        f.extend(std::iter::repeat(Fr::zero()).take(fk_domain.toeplitz_domain.dimension() + 1 - f.len()));
//        let toeplitz = fk_domain.toeplitz_for_poly(&f);
//        let h_term_commitments = fk_domain.toeplitz_domain.eval_prepared(&toeplitz, &fk_domain.prepared_toeplitz_input);
//
//        let fft_domains : Vec<Radix2EvaluationDomain<Fr>> = (0..poly_degree.ilog2()).map(|i| Radix2EvaluationDomain::new(2usize.pow(i)).unwrap()).collect();
//
//        group.bench_with_input(BenchmarkId::from_parameter(poly_degree), &(fft_domains, h_term_commitments), |b, input| {
//            b.iter(|| {
//                let l = poly_degree.ilog2() as usize;
//                for i in 0..l {
//                    (0..2usize.pow((l - i) as u32)).into_par_iter().for_each(|_j| {
//                        input.0[i].fft(&input.1[..2usize.pow(i as u32)]);
//                    });
//                }
//            }
//            );
//        });
//    }
//}
//
//criterion_group!(benches, eval_proofs_at_roots_of_unity, eval_proofs_at_x_coords, toeplitz_for_poly, toeplitz_eval_prepared, toeplitz_to_circulant, circulant_eval_prepared, fft, fft_tree);
//criterion_main!(benches);
//
