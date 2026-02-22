// use crate::ml_sumcheck::BinaryConstraintPolynomial;
// use ark_bn254::Fr;
// use ark_ff::{One, Zero};
// use ark_poly::{DenseMultilinearExtension, MultilinearExtension};
// use ark_std::{test_rng, UniformRand};
// use ark_ff::Field;
// use crate::ark_std::rand::Rng;

// #[test]
// fn test_binary_constraint_sumcheck() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();
//     let nv = 10;

//     // Create random eq_point
//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();
//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//     let mut expected_sum = Fr::zero();

//     // Add several constraints
//     for _ in 0..5 {
//         let coefficient = Fr::rand(&mut rng);
//         let p = DenseMultilinearExtension::rand(nv, &mut rng);

//         // Compute actual sum with eq masking
//         for b in 0..(1 << nv) {
//             let p_val = p[b];
//             let binary_val = p_val * (Fr::one() - p_val);

//             // Build point from b
//             let mut point = Vec::new();
//             for j in 0..nv {
//                 if (b >> j) & 1 == 1 {
//                     point.push(Fr::one());
//                 } else {
//                     point.push(Fr::zero());
//                 }
//             }

//             // Compute eq_t
//             let mut eq_t = Fr::one();
//             for j in 0..nv {
//                 let tj = eq_point[j];
//                 let xj = point[j];
//                 eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//             }

//             // Compute eq_{1,...,1}
//             let mut eq_ones = Fr::one();
//             for &xj in &point {
//                 eq_ones *= xj;
//             }

//             expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//         }

//         poly.add_constraint(coefficient, p);
//     }

//     // Run protocol
//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     // Verify subclaim
//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation,
//         "subclaim evaluation mismatch"
//     );
// }

// #[test]
// fn test_single_constraint() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();
//     let nv = 8;

//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();
//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//     let coefficient = Fr::rand(&mut rng);
//     let p = DenseMultilinearExtension::rand(nv, &mut rng);

//     // Compute expected sum with eq masking
//     let mut expected_sum = Fr::zero();
//     for b in 0..(1 << nv) {
//         let p_val = p[b];
//         let binary_val = p_val * (Fr::one() - p_val);

//         let mut point = Vec::new();
//         for j in 0..nv {
//             if (b >> j) & 1 == 1 {
//                 point.push(Fr::one());
//             } else {
//                 point.push(Fr::zero());
//             }
//         }

//         let mut eq_t = Fr::one();
//         for j in 0..nv {
//             let tj = eq_point[j];
//             let xj = point[j];
//             eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//         }

//         let mut eq_ones = Fr::one();
//         for &xj in &point {
//             eq_ones *= xj;
//         }

//         expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//     }

//     poly.add_constraint(coefficient, p);

//     // Run protocol
//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation
//     );
// }

// #[test]
// fn test_boolean_inputs() {
//     use crate::ml_sumcheck::MLSumcheck;

//     // Test that P(1-P) = 0 when P ∈ {0,1}
//     let nv = 4;
//     let eq_point: Vec<Fr> = vec![Fr::zero(); nv];
//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());

//     // Create a polynomial that evaluates to only 0s and 1s
//     let boolean_evals: Vec<Fr> = (0..(1 << nv))
//         .map(|i| if i % 2 == 0 { Fr::zero() } else { Fr::one() })
//         .collect();

//     let p = DenseMultilinearExtension::from_evaluations_vec(nv, boolean_evals.clone());
//     poly.add_constraint(Fr::one(), p);

//     // Compute expected sum (should be zero since P(1-P) = 0 for boolean)
//     let mut expected_sum = Fr::zero();
//     for b in 0..(1 << nv) {
//         let p_val = boolean_evals[b];
//         let binary_val = p_val * (Fr::one() - p_val); // This is 0

//         let mut point = Vec::new();
//         for j in 0..nv {
//             if (b >> j) & 1 == 1 {
//                 point.push(Fr::one());
//             } else {
//                 point.push(Fr::zero());
//             }
//         }

//         let mut eq_t = Fr::one();
//         for j in 0..nv {
//             let tj = eq_point[j];
//             let xj = point[j];
//             eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//         }

//         let mut eq_ones = Fr::one();
//         for &xj in &point {
//             eq_ones *= xj;
//         }

//         expected_sum += binary_val * eq_t * (Fr::one() - eq_ones);
//     }

//     assert_eq!(expected_sum, Fr::zero(), "Boolean inputs should give zero sum");

//     // Run protocol
//     let rng = test_rng();
//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation
//     );
// }

// #[test]
// fn test_multiple_rounds() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();

//     // Test with different numbers of variables
//     for nv in [1, 2, 5, 8, 12] {
//         let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();
//         let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//         let mut expected_sum = Fr::zero();

//         // Add random constraints
//         for _ in 0..3 {
//             let coefficient = Fr::rand(&mut rng);
//             let p = DenseMultilinearExtension::rand(nv, &mut rng);

//             for b in 0..(1 << nv) {
//                 let p_val = p[b];
//                 let binary_val = p_val * (Fr::one() - p_val);

//                 let mut point = Vec::new();
//                 for j in 0..nv {
//                     if (b >> j) & 1 == 1 {
//                         point.push(Fr::one());
//                     } else {
//                         point.push(Fr::zero());
//                     }
//                 }

//                 let mut eq_t = Fr::one();
//                 for j in 0..nv {
//                     let tj = eq_point[j];
//                     let xj = point[j];
//                     eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//                 }

//                 let mut eq_ones = Fr::one();
//                 for &xj in &point {
//                     eq_ones *= xj;
//                 }

//                 expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//             }

//             poly.add_constraint(coefficient, p);
//         }

//         let poly_info = poly.info();

//         // Non-interactive prove
//         let proof = MLSumcheck::prove(&poly).expect("prove failed");

//         // Non-interactive verify
//         let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//             .expect(&format!("verification failed for nv={}", nv));

//         assert_eq!(
//             poly.evaluate(&subclaim.point),
//             subclaim.expected_evaluation,
//             "failed for nv={}",
//             nv
//         );
//     }
// }

// #[test]
// #[should_panic(expected = "Attempt to prove a constant")]
// fn test_zero_variables_panics() {
//     use crate::ml_sumcheck::protocol::IPForMLSumcheck;

//     let poly = BinaryConstraintPolynomial::<Fr>::new_without_g(0, vec![]);
//     IPForMLSumcheck::prover_init(&poly);
// }

// #[test]
// #[should_panic(expected = "Polynomial has wrong number of variables")]
// fn test_mismatched_variables_panics() {
//     let mut rng = test_rng();
//     let eq_point: Vec<Fr> = (0..5).map(|_| Fr::rand(&mut rng)).collect();
//     let mut poly = BinaryConstraintPolynomial::new_without_g(5, eq_point);
//     let p = DenseMultilinearExtension::rand(6, &mut rng); // Wrong size!
//     poly.add_constraint(Fr::one(), p);
// }

// #[test]
// fn test_evaluation_correctness() {
//     let mut rng = test_rng();
//     let nv = 6;

//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();
//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());

//     // Add constraints
//     let c1 = Fr::from(3u64);
//     let p1 = DenseMultilinearExtension::rand(nv, &mut rng);
//     poly.add_constraint(c1, p1.clone());

//     let c2 = Fr::from(7u64);
//     let p2 = DenseMultilinearExtension::rand(nv, &mut rng);
//     poly.add_constraint(c2, p2.clone());

//     // Test evaluation at random point
//     let point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();

//     let poly_eval = poly.evaluate(&point);

//     // Manual computation
//     let p1_val = p1.evaluate(&point).unwrap();
//     let p2_val = p2.evaluate(&point).unwrap();
//     let binary_sum = c1 * p1_val * (Fr::one() - p1_val)
//                    + c2 * p2_val * (Fr::one() - p2_val);

//     // Compute eq_t
//     let mut eq_t = Fr::one();
//     for i in 0..nv {
//         let ti = eq_point[i];
//         let xi = point[i];
//         eq_t *= (Fr::one() - ti) + xi * (ti + ti - Fr::one());
//     }

//     // Compute eq_{1,...,1}
//     let mut eq_ones = Fr::one();
//     for &xi in &point {
//         eq_ones *= xi;
//     }

//     let expected = binary_sum * eq_t * (Fr::one() - eq_ones);

//     assert_eq!(poly_eval, expected, "evaluation mismatch");
// }

// #[test]
// fn test_with_eq_masking() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();
//     let nv = 8;

//     // Create random eq_point
//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();

//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//     let mut expected_sum = Fr::zero();

//     // Add several constraints
//     for _ in 0..3 {
//         let coefficient = Fr::rand(&mut rng);
//         let p = DenseMultilinearExtension::rand(nv, &mut rng);

//         // Compute actual sum manually
//         for b in 0..(1 << nv) {
//             let p_val = p[b];
//             let binary_val = p_val * (Fr::one() - p_val);

//             // Compute point from b
//             let mut point = Vec::new();
//             for j in 0..nv {
//                 if (b >> j) & 1 == 1 {
//                     point.push(Fr::one());
//                 } else {
//                     point.push(Fr::zero());
//                 }
//             }

//             // Compute eq_t
//             let mut eq_t = Fr::one();
//             for j in 0..nv {
//                 let tj = eq_point[j];
//                 let xj = point[j];
//                 eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//             }

//             // Compute eq_{1,...,1}
//             let mut eq_ones = Fr::one();
//             for &xj in &point {
//                 eq_ones *= xj;
//             }

//             expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//         }

//         poly.add_constraint(coefficient, p);
//     }

//     println!("Expected sum with eq masking: {:?}", expected_sum);

//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation,
//         "Subclaim verification failed"
//     );
// }

// #[test]
// fn test_eq_masking_zeros_out_all_ones() {
//     let mut rng = test_rng();
//     let nv = 6;

//     // Create random eq_point
//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();

//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());

//     // Add a constraint
//     let coefficient = Fr::rand(&mut rng);
//     let p = DenseMultilinearExtension::rand(nv, &mut rng);
//     poly.add_constraint(coefficient, p.clone());

//     // Evaluate at (1,1,...,1)
//     let all_ones: Vec<Fr> = vec![Fr::one(); nv];
//     let eval_at_ones = poly.evaluate(&all_ones);

//     // Should be zero because (1 - eq_{1,...,1}(1,...,1)) = (1 - 1) = 0
//     assert_eq!(eval_at_ones, Fr::zero(), "Polynomial should be zero at (1,...,1)");

//     println!("Eval at (1,...,1): {:?}", eval_at_ones);
// }

// #[test]
// fn test_eq_point_at_origin() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();
//     let nv = 6;

//     // Set eq_point to (0,0,...,0) so eq_t(0,...,0) = 1
//     let eq_point: Vec<Fr> = vec![Fr::zero(); nv];

//     let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//     let mut expected_sum = Fr::zero();

//     // Add constraint
//     let coefficient = Fr::rand(&mut rng);
//     let p = DenseMultilinearExtension::rand(nv, &mut rng);

//     // Compute expected sum
//     for b in 0..(1 << nv) {
//         let p_val = p[b];
//         let binary_val = p_val * (Fr::one() - p_val);

//         // Build point
//         let mut point = Vec::new();
//         for j in 0..nv {
//             if (b >> j) & 1 == 1 {
//                 point.push(Fr::one());
//             } else {
//                 point.push(Fr::zero());
//             }
//         }

//         // eq_t with t = (0,...,0)
//         // eq_t(x) = ∏ᵢ (1-0)·(1-xᵢ) = ∏ᵢ (1-xᵢ)
//         let mut eq_t = Fr::one();
//         for &xj in &point {
//             eq_t *= Fr::one() - xj;
//         }

//         // eq_{1,...,1}
//         let mut eq_ones = Fr::one();
//         for &xj in &point {
//             eq_ones *= xj;
//         }

//         expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//     }

//     poly.add_constraint(coefficient, p);

//     println!("Expected sum with eq_point at origin: {:?}", expected_sum);

//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation,
//         "Subclaim verification failed"
//     );
// }

// #[test]
// fn test_multiple_constraints_with_eq_masking() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();

//     for nv in [3, 5, 10] {
//         let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();
//         let mut poly = BinaryConstraintPolynomial::new_without_g(nv, eq_point.clone());
//         let mut expected_sum = Fr::zero();

//         // All-ones point is at index 2^nv - 1
//         let all_ones_index = (1 << nv) - 1;

//         // Add multiple constraints
//         for _ in 0..5 {
//             let coefficient = Fr::rand(&mut rng);

//             // Create polynomial that is 0 or 1 everywhere except at (1,...,1)
//             let mut evaluations = Vec::with_capacity(1 << nv);
//             for idx in 0..(1 << nv) {
//                 if idx == all_ones_index {
//                     // Random value at (1,1,...,1)
//                     evaluations.push(Fr::rand(&mut rng));
//                 } else {
//                     // Random boolean (0 or 1) at other points
//                     if rng.gen_bool(0.5) {
//                         evaluations.push(Fr::one());
//                     } else {
//                         evaluations.push(Fr::zero());
//                     }
//                 }
//             }

//             let p = DenseMultilinearExtension::from_evaluations_vec(nv, evaluations.clone());

//             // Compute expected sum
//             for b in 0..(1 << nv) {
//                 let p_val = evaluations[b];
//                 let binary_val = p_val * (Fr::one() - p_val);

//                 let mut point = Vec::new();
//                 for j in 0..nv {
//                     if (b >> j) & 1 == 1 {
//                         point.push(Fr::one());
//                     } else {
//                         point.push(Fr::zero());
//                     }
//                 }

//                 let mut eq_t = Fr::one();
//                 for j in 0..nv {
//                     let tj = eq_point[j];
//                     let xj = point[j];
//                     eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//                 }

//                 let mut eq_ones = Fr::one();
//                 for &xj in &point {
//                     eq_ones *= xj;
//                 }

//                 expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//             }

//             poly.add_constraint(coefficient, p);
//         }

//         let poly_info = poly.info();
//         let proof = MLSumcheck::prove(&poly).expect(&format!("prove failed for nv={}", nv));
//         let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//             .expect(&format!("verification failed for nv={}", nv));

//         assert_eq!(
//             poly.evaluate(&subclaim.point),
//             subclaim.expected_evaluation,
//             "failed for nv={}",
//             nv
//         );

//         // Additional check: verify that the sum is actually zero
//         // because (1 - eq_{1,...,1}(1,...,1)) = 0
//         println!("  Expected sum should be 0 due to (1 - eq_{{1,...,1}}) masking: {}",
//                  expected_sum == Fr::zero());
//     }
// }

// #[test]
// fn test_with_g_polynomial() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();
//     let nv = 4;

//     // Create random eq_point
//     let eq_point: Vec<Fr> = (0..nv).map(|_| Fr::rand(&mut rng)).collect();

//     // Create random α
//     let alpha = Fr::rand(&mut rng);

//     // Create random g polynomials (one per variable)
//     let g_polys: Vec<Vec<Fr>> = (0..nv)
//         .map(|_| (0..5).map(|_| Fr::rand(&mut rng)).collect())
//         .collect();

//     let mut poly = BinaryConstraintPolynomial::new(nv, eq_point.clone(), alpha, g_polys.clone());
//     let mut expected_sum = Fr::zero();

//     // Add several constraints
//     for _ in 0..3 {
//         let coefficient = Fr::rand(&mut rng);
//         let p = DenseMultilinearExtension::rand(nv, &mut rng);

//         // Compute sum of binary constraint term
//         for b in 0..(1 << nv) {
//             let p_val = p[b];
//             let binary_val = p_val * (Fr::one() - p_val);

//             // Build point from b
//             let mut point = Vec::new();
//             for j in 0..nv {
//                 if (b >> j) & 1 == 1 {
//                     point.push(Fr::one());
//                 } else {
//                     point.push(Fr::zero());
//                 }
//             }

//             // Compute eq_t
//             let mut eq_t = Fr::one();
//             for j in 0..nv {
//                 let tj = eq_point[j];
//                 let xj = point[j];
//                 eq_t *= (Fr::one() - tj) + xj * (tj + tj - Fr::one());
//             }

//             // Compute eq_{1,...,1}
//             let mut eq_ones = Fr::one();
//             for &xj in &point {
//                 eq_ones *= xj;
//             }

//             expected_sum += coefficient * binary_val * eq_t * (Fr::one() - eq_ones);
//         }

//         poly.add_constraint(coefficient, p);
//     }

//     // Add α·g term
//     // Σ_{x∈{0,1}ⁿ} α·g(x) = α · Σᵢ 2^(n-1) · [gᵢ(0) + gᵢ(1)]
//     let half_hypercube = Fr::from(1u64 << (nv - 1));
//     for i in 0..nv {
//         let coeffs = &g_polys[i];
//         let g_i_at_0 = coeffs[0]; // r₀
//         let g_i_at_1 = coeffs[0] + coeffs[1] + coeffs[2] + coeffs[3] + coeffs[4]; // r₀ + r₁ + r₂ + r₃ + r₄

//         expected_sum += alpha * half_hypercube * (g_i_at_0 + g_i_at_1);
//     }

//     println!("Expected sum with g polynomial: {:?}", expected_sum);

//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation,
//         "Subclaim verification failed"
//     );
// }

// /// Example: Convert sum_{j=1}^{log(n)} γ_j · f̂_j(b⃗) · (f̂_j(b⃗) - 1) into sumcheck format
// ///
// /// This demonstrates how to convert the expression:
// ///   Σ_{j=1}^{log(n)} γ_j · f̂_j(b⃗) · (f̂_j(b⃗) - 1)
// ///
// /// into a BinaryConstraintPolynomial that can be used with MLSumcheck.
// ///
// /// Key conversion: The library uses P(1-P) format, but the expression is P(P-1).
// /// Since P(P-1) = -(P(1-P)), we use coefficient -γ_j instead of γ_j.
// #[test]
// fn example_convert_sumcheck_expression() {
//     use crate::ml_sumcheck::MLSumcheck;

//     let mut rng = test_rng();

//     // Parameters
//     let _n = 16;  // Example: n = 16
//     let log_n = 4;  // log₂(16) = 4
//     let num_variables = 4;  // Dimension of b⃗ (this is the number of variables for multilinear extensions)

//     // The point b⃗ at which we want to evaluate (this becomes eq_point)
//     let b_vec: Vec<Fr> = (0..num_variables).map(|_| Fr::rand(&mut rng)).collect();

//     // Create the polynomial
//     // Note: If you don't need the g term, use new_without_g
//     let mut poly = BinaryConstraintPolynomial::new_without_g(num_variables, b_vec.clone());

//     // For each j from 1 to log(n), add the constraint
//     let mut expected_sum = Fr::zero();

//     for _j in 1..=log_n {
//         // γ_j is a scalar coefficient
//         let gamma_j = Fr::rand(&mut rng);

//         // f̂_j is a multilinear extension created from a list of values
//         // The list should have 2^num_variables elements (one for each point in {0,1}^num_variables)
//         // Example: create random values for f̂_j
//         let f_j_values: Vec<Fr> = (0..(1 << num_variables))
//             .map(|_| Fr::rand(&mut rng))
//             .collect();

//         // Create the multilinear extension from the list of values
//         let f_hat_j = DenseMultilinearExtension::from_evaluations_vec(
//             num_variables,
//             f_j_values.clone()
//         );

//         // IMPORTANT: The library uses P(1-P) format, but your expression is P(P-1)
//         // Since P(P-1) = -(P(1-P)), we use -γ_j as the coefficient
//         let coefficient = -gamma_j;

//         // Add the constraint
//         poly.add_constraint(coefficient, f_hat_j.clone());

//         // Compute expected sum for verification
//         // This computes: Σ_{x ∈ {0,1}^num_variables} γ_j · f̂_j(x) · (f̂_j(x) - 1) · eq_b(x) · (1 - eq_{1,...,1}(x))
//         for b in 0..(1 << num_variables) {
//             let f_j_val = f_j_values[b];
//             let binary_val = f_j_val * (f_j_val - Fr::one());  // f̂_j(x) · (f̂_j(x) - 1)

//             // Build point from b
//             let mut point = Vec::new();
//             for k in 0..num_variables {
//                 if (b >> k) & 1 == 1 {
//                     point.push(Fr::one());
//                 } else {
//                     point.push(Fr::zero());
//                 }
//             }

//             // Compute eq_b(x) where b is the eq_point
//             let mut eq_b = Fr::one();
//             for k in 0..num_variables {
//                 let bk = b_vec[k];
//                 let xk = point[k];
//                 eq_b *= (Fr::one() - bk) + xk * (bk + bk - Fr::one());
//             }

//             // Compute eq_{1,...,1}(x)
//             let mut eq_ones = Fr::one();
//             for &xk in &point {
//                 eq_ones *= xk;
//             }

//             // Add to expected sum: γ_j · f̂_j(x) · (f̂_j(x) - 1) · eq_b(x) · (1 - eq_{1,...,1}(x))
//             expected_sum += gamma_j * binary_val * eq_b * (Fr::one() - eq_ones);
//         }
//     }

//     // Run the sumcheck protocol
//     let poly_info = poly.info();
//     let proof = MLSumcheck::prove(&poly).expect("prove failed");
//     let subclaim = MLSumcheck::verify(&poly_info, expected_sum, &proof)
//         .expect("verification failed");

//     // Verify the subclaim
//     assert_eq!(
//         poly.evaluate(&subclaim.point),
//         subclaim.expected_evaluation,
//         "Subclaim verification failed"
//     );

//     println!("Successfully converted and verified sumcheck expression!");
//     println!("Number of constraints: {}", log_n);
//     println!("Number of variables: {}", num_variables);
//     println!("Expected sum: {:?}", expected_sum);
// }
