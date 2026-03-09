//! Benchmarks for aptos-crypto sumcheck (Single MLE sum, product of 4 MLEs, booleanity-eq).
//!
//! Compare:
//! - **SingleMLE**: sum of one random MLE → degree-1 round polynomials.
//! - **Product4MLE**: sum of product of 4 random MLEs → degree-4 round polynomials.
//! - **Product4MLE_masking**: same as Product4MLE but with optional ZK masking (no mixed terms).
//! - **BooleanityEq**: sum_x [ sum_j c^j MLE[j](x)(1-MLE[j](x)) ] * eq_t(x) * (1-eq_0(x)) → degree-4 rounds.
//!
//! Run: `cargo bench -p aptos-crypto --bench sumcheck`
//! Filter: `cargo bench -p aptos-crypto --bench sumcheck Product4MLE` or `BooleanityEq`.

// #[macro_use]
// #[allow(unused_imports)]
// extern crate criterion;

// use aptos_crypto::sumcheck::{
//     BatchedSumcheck, BindingOrder, BooleanityEqSumcheckProver, BooleanityEqSumcheckVerifier,
//     DensePolynomial, MaskingPolynomial, MerlinSumcheckTranscript, ProverOpeningAccumulator,
//     SumcheckInstanceParams, SumcheckInstanceProver, SumcheckInstanceVerifier, UniPoly,
//     VerifierOpeningAccumulator,
// };
// use ark_bn254::Fr;
// use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
// use merlin::Transcript;
// use rand::{rngs::StdRng, RngCore, SeedableRng};

// /// Minimal sumcheck instance: proves sum_x MLE(x) = claim for one dense MLE (degree-1 rounds).
// mod simple_sumcheck_instance {
//     use super::*;
//     use std::marker::PhantomData;

//     pub struct SimpleMleSumcheckParams<F: aptos_crypto::sumcheck::SumcheckField> {
//         pub num_rounds: usize,
//         pub initial_claim: Option<F>,
//         _marker: PhantomData<F>,
//     }

//     impl<F: aptos_crypto::sumcheck::SumcheckField> SumcheckInstanceParams<F>
//         for SimpleMleSumcheckParams<F>
//     {
//         fn degree(&self) -> usize {
//             1
//         }

//         fn num_rounds(&self) -> usize {
//             self.num_rounds
//         }

//         fn input_claim(&self, _: &dyn aptos_crypto::sumcheck::OpeningAccumulator<F>) -> F {
//             self.initial_claim.unwrap_or(F::zero())
//         }
//     }

//     pub struct SimpleMleSumcheckProver<F: aptos_crypto::sumcheck::SumcheckField> {
//         params: SimpleMleSumcheckParams<F>,
//         poly: DensePolynomial<F>,
//     }

//     impl<F: aptos_crypto::sumcheck::SumcheckField> SimpleMleSumcheckProver<F> {
//         pub fn new(num_vars: usize, evals: Vec<F>) -> Self {
//             let poly = DensePolynomial::new(evals.clone());
//             assert_eq!(poly.get_num_vars(), num_vars);
//             let initial_claim = poly.Z.iter().fold(F::zero(), |a, &b| a + b);
//             Self {
//                 params: SimpleMleSumcheckParams {
//                     num_rounds: num_vars,
//                     initial_claim: Some(initial_claim),
//                     _marker: PhantomData,
//                 },
//                 poly,
//             }
//         }

//         fn half_sum(coeffs: &[F]) -> (F, F) {
//             let n = coeffs.len() / 2;
//             let h0 = coeffs[..n].iter().fold(F::zero(), |a, &b| a + b);
//             let h1 = coeffs[n..].iter().fold(F::zero(), |a, &b| a + b);
//             (h0, h1)
//         }
//     }

//     impl<F: aptos_crypto::sumcheck::SumcheckField> SumcheckInstanceProver<F>
//         for SimpleMleSumcheckProver<F>
//     {
//         fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
//             &self.params
//         }

//         fn input_claim(&self, _: &ProverOpeningAccumulator<F>) -> F {
//             self.poly.Z.iter().fold(F::zero(), |a, &b| a + b)
//         }

//         fn compute_message(&mut self, _round: usize, _previous_claim: F) -> UniPoly<F> {
//             let (h0, h1) = Self::half_sum(&self.poly.Z);
//             UniPoly::from_coeff(vec![h0, h1 - h0])
//         }

//         fn ingest_challenge(&mut self, r_j: F::Challenge, _round: usize) {
//             self.poly.bind(&r_j, BindingOrder::HighToLow);
//         }
//     }

//     pub struct SimpleMleSumcheckVerifier<F: aptos_crypto::sumcheck::SumcheckField> {
//         params: SimpleMleSumcheckParams<F>,
//         pub poly_evals: Option<Vec<F>>,
//     }

//     impl<F: aptos_crypto::sumcheck::SumcheckField> SimpleMleSumcheckVerifier<F> {
//         pub fn with_poly(num_rounds: usize, evals: Vec<F>) -> Self {
//             let initial_claim = evals.iter().fold(F::zero(), |a, &b| a + b);
//             Self {
//                 params: SimpleMleSumcheckParams {
//                     num_rounds,
//                     initial_claim: Some(initial_claim),
//                     _marker: PhantomData,
//                 },
//                 poly_evals: Some(evals),
//             }
//         }
//     }

//     impl<F: aptos_crypto::sumcheck::SumcheckField> SumcheckInstanceVerifier<F>
//         for SimpleMleSumcheckVerifier<F>
//     {
//         fn get_params(&self) -> &dyn SumcheckInstanceParams<F> {
//             &self.params
//         }

//         fn expected_output_claim(
//             &self,
//             _accumulator: &VerifierOpeningAccumulator<F>,
//             r: &[F::Challenge],
//         ) -> F {
//             match &self.poly_evals {
//                 None => F::zero(),
//                 Some(evals) => {
//                     let mut poly = DensePolynomial::new(evals.clone());
//                     for r_j in r {
//                         poly.bind(r_j, BindingOrder::HighToLow);
//                     }
//                     assert_eq!(poly.len(), 1);
//                     poly.Z[0]
//                 },
//             }
//         }
//     }
// }

// use aptos_crypto::sumcheck::{Product4SumcheckProver, Product4SumcheckVerifier};
// use simple_sumcheck_instance::{SimpleMleSumcheckProver, SimpleMleSumcheckVerifier};

// const NUM_VARS_RANGE: std::ops::Range<usize> = 10..18;

// fn random_evals(num_vars: usize, rng: &mut StdRng) -> Vec<Fr> {
//     let n = 1 << num_vars;
//     (0..n)
//         .map(|_| Fr::from(rng.next_u64() % (1 << 20)))
//         .collect()
// }

// fn random_evals_4(num_vars: usize, rng: &mut StdRng) -> [Vec<Fr>; 4] {
//     [
//         random_evals(num_vars, rng),
//         random_evals(num_vars, rng),
//         random_evals(num_vars, rng),
//         random_evals(num_vars, rng),
//     ]
// }

// /// Number of MLEs in the booleanity-eq sum (e.g. 16).
// const BOOLEANITY_EQ_M: usize = 16;

// fn random_mle_evals_m(num_vars: usize, m: usize, rng: &mut StdRng) -> Vec<Vec<Fr>> {
//     (0..m).map(|_| random_evals(num_vars, rng)).collect()
// }

// fn random_t(num_vars: usize, rng: &mut StdRng) -> Vec<Fr> {
//     (0..num_vars)
//         .map(|_| Fr::from(rng.next_u64() % (1 << 20)))
//         .collect()
// }

// fn aptos_sumcheck_prove_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_prove");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("SingleMLE", nv), &nv, |b, &nv| {
//             b.iter(|| {
//                 let evals = random_evals(nv, &mut rng);
//                 let mut prover = SimpleMleSumcheckProver::new(nv, evals);
//                 let mut opening = ProverOpeningAccumulator::new(0);
//                 let mut transcript_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript = MerlinSumcheckTranscript::new(&mut transcript_inner);
//                 let (proof, _challenges, _claim) =
//                     BatchedSumcheck::prove(vec![&mut prover], &mut opening, &mut transcript);
//                 black_box(proof);
//             });
//         });
//     }
// }

// fn aptos_sumcheck_verify_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_verify");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("SingleMLE", nv), &nv, |b, &nv| {
//             let evals = random_evals(nv, &mut rng);
//             let mut prover = SimpleMleSumcheckProver::new(nv, evals.clone());
//             let mut opening_p = ProverOpeningAccumulator::new(0);
//             let mut transcript_p_inner = Transcript::new(b"AptosSumcheckBench");
//             let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
//             let (proof, _challenges, _claim) =
//                 BatchedSumcheck::prove(vec![&mut prover], &mut opening_p, &mut transcript_p);
//             let verifier = SimpleMleSumcheckVerifier::with_poly(nv, evals);
//             b.iter(|| {
//                 let mut opening_v = VerifierOpeningAccumulator::new(0, false);
//                 let mut transcript_v_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);
//                 let result = BatchedSumcheck::verify_standard(
//                     &proof,
//                     vec![&verifier],
//                     &mut opening_v,
//                     &mut transcript_v,
//                 );
//                 let _ = black_box(result);
//             });
//         });
//     }
// }

// fn aptos_sumcheck_prove_degree4_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_prove_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("Product4MLE", nv), &nv, |b, &nv| {
//             b.iter(|| {
//                 let evals = random_evals_4(nv, &mut rng);
//                 let mut prover = Product4SumcheckProver::new(nv, evals);
//                 let mut opening = ProverOpeningAccumulator::new(0);
//                 let mut transcript_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript = MerlinSumcheckTranscript::new(&mut transcript_inner);
//                 let (proof, _challenges, _claim) =
//                     BatchedSumcheck::prove(vec![&mut prover], &mut opening, &mut transcript);
//                 black_box(proof);
//             });
//         });
//     }
// }

// fn aptos_sumcheck_verify_degree4_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_verify_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("Product4MLE", nv), &nv, |b, &nv| {
//             let evals = random_evals_4(nv, &mut rng);
//             let mut prover = Product4SumcheckProver::new(nv, evals.clone());
//             let mut opening_p = ProverOpeningAccumulator::new(0);
//             let mut transcript_p_inner = Transcript::new(b"AptosSumcheckBench");
//             let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
//             let (proof, _challenges, _claim) =
//                 BatchedSumcheck::prove(vec![&mut prover], &mut opening_p, &mut transcript_p);
//             let verifier = Product4SumcheckVerifier::with_polys(nv, evals);
//             b.iter(|| {
//                 let mut opening_v = VerifierOpeningAccumulator::new(0, false);
//                 let mut transcript_v_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);
//                 let result = BatchedSumcheck::verify_standard(
//                     &proof,
//                     vec![&verifier],
//                     &mut opening_v,
//                     &mut transcript_v,
//                 );
//                 let _ = black_box(result);
//             });
//         });
//     }
// }

// /// Product4MLE **with** masking (ZK): same workload + O(m*d) masking round messages.
// fn aptos_sumcheck_prove_degree4_masking_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     const DEGREE: usize = 4;
//     let seed = b"AptosSumcheckMaskingBench_seed!!!!!!!!!!!!!!";
//     let mut group = c.benchmark_group("aptos_sumcheck_prove_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(
//             BenchmarkId::new("Product4MLE_masking", nv),
//             &nv,
//             |b, &nv| {
//                 b.iter(|| {
//                     let evals = random_evals_4(nv, &mut rng);
//                     let g = MaskingPolynomial::<Fr>::from_seed(seed, nv, DEGREE);
//                     let mut prover = Product4SumcheckProver::new_with_masking(nv, evals, g);
//                     let mut opening = ProverOpeningAccumulator::new(0);
//                     let mut transcript_inner = Transcript::new(b"AptosSumcheckBench");
//                     let mut transcript = MerlinSumcheckTranscript::new(&mut transcript_inner);
//                     let (proof, _challenges, _claim) =
//                         BatchedSumcheck::prove(vec![&mut prover], &mut opening, &mut transcript);
//                     black_box(proof);
//                 });
//             },
//         );
//     }
// }

// fn aptos_sumcheck_verify_degree4_masking_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     const DEGREE: usize = 4;
//     let seed = b"AptosSumcheckMaskingBench_seed!!!!!!!!!!!!!!";
//     let mut group = c.benchmark_group("aptos_sumcheck_verify_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(
//             BenchmarkId::new("Product4MLE_masking", nv),
//             &nv,
//             |b, &nv| {
//                 let evals = random_evals_4(nv, &mut rng);
//                 let g = MaskingPolynomial::<Fr>::from_seed(seed, nv, DEGREE);
//                 let mut prover =
//                     Product4SumcheckProver::new_with_masking(nv, evals.clone(), g.clone());
//                 let mut opening_p = ProverOpeningAccumulator::new(0);
//                 let mut transcript_p_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
//                 let (proof, _challenges, _claim) =
//                     BatchedSumcheck::prove(vec![&mut prover], &mut opening_p, &mut transcript_p);
//                 let verifier = Product4SumcheckVerifier::with_polys_and_masking(nv, evals, g);
//                 b.iter(|| {
//                     let mut opening_v = VerifierOpeningAccumulator::new(0, false);
//                     let mut transcript_v_inner = Transcript::new(b"AptosSumcheckBench");
//                     let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);
//                     let result = BatchedSumcheck::verify_standard(
//                         &proof,
//                         vec![&verifier],
//                         &mut opening_v,
//                         &mut transcript_v,
//                     );
//                     let _ = black_box(result);
//                 });
//             },
//         );
//     }
// }

// // ---- BooleanityEq: sum_x [ sum_j c^j MLE[j](x)(1-MLE[j](x)) ] * eq_t(x) * (1-eq_0(x)) ----

// fn aptos_sumcheck_prove_booleanity_eq_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_prove_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("BooleanityEq_m16", nv), &nv, |b, &nv| {
//             b.iter(|| {
//                 let mle_evals = random_mle_evals_m(nv, BOOLEANITY_EQ_M, &mut rng);
//                 let c = Fr::from(rng.next_u64() % (1 << 20));
//                 let t = random_t(nv, &mut rng);
//                 let mut prover = BooleanityEqSumcheckProver::new(nv, mle_evals, c, t);
//                 let mut opening = ProverOpeningAccumulator::new(0);
//                 let mut transcript_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript = MerlinSumcheckTranscript::new(&mut transcript_inner);
//                 let (proof, _challenges, _claim) =
//                     BatchedSumcheck::prove(vec![&mut prover], &mut opening, &mut transcript);
//                 black_box(proof);
//             });
//         });
//     }
// }

// fn aptos_sumcheck_verify_booleanity_eq_bench(c: &mut Criterion) {
//     let mut rng = StdRng::seed_from_u64(0);
//     let mut group = c.benchmark_group("aptos_sumcheck_verify_degree4");
//     for nv in NUM_VARS_RANGE {
//         group.bench_with_input(BenchmarkId::new("BooleanityEq_m16", nv), &nv, |b, &nv| {
//             let mle_evals = random_mle_evals_m(nv, BOOLEANITY_EQ_M, &mut rng);
//             let c = Fr::from(rng.next_u64() % (1 << 20));
//             let t = random_t(nv, &mut rng);
//             let mut prover = BooleanityEqSumcheckProver::new(nv, mle_evals.clone(), c, t.clone());
//             let mut opening_p = ProverOpeningAccumulator::new(0);
//             let mut transcript_p_inner = Transcript::new(b"AptosSumcheckBench");
//             let mut transcript_p = MerlinSumcheckTranscript::new(&mut transcript_p_inner);
//             let (proof, _challenges, _claim) =
//                 BatchedSumcheck::prove(vec![&mut prover], &mut opening_p, &mut transcript_p);
//             let verifier = BooleanityEqSumcheckVerifier::new(nv, mle_evals, c, t);
//             b.iter(|| {
//                 let mut opening_v = VerifierOpeningAccumulator::new(0, false);
//                 let mut transcript_v_inner = Transcript::new(b"AptosSumcheckBench");
//                 let mut transcript_v = MerlinSumcheckTranscript::new(&mut transcript_v_inner);
//                 let result = BatchedSumcheck::verify_standard(
//                     &proof,
//                     vec![&verifier],
//                     &mut opening_v,
//                     &mut transcript_v,
//                 );
//                 let _ = black_box(result);
//             });
//         });
//     }
// }

// criterion_group!(
//     benches,
//     aptos_sumcheck_prove_bench,
//     aptos_sumcheck_verify_bench,
//     aptos_sumcheck_prove_degree4_bench,
//     aptos_sumcheck_verify_degree4_bench,
//     aptos_sumcheck_prove_degree4_masking_bench,
//     aptos_sumcheck_verify_degree4_masking_bench,
//     aptos_sumcheck_prove_booleanity_eq_bench,
//     aptos_sumcheck_verify_booleanity_eq_bench,
// );
// criterion_main!(benches);
