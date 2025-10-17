use aptos_dkg::algebra::polynomials::barycentric_eval;
use ark_ff::{FftField, Field};
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, Polynomial};
use ark_std::UniformRand;

#[test]
fn test_barycentric_eval_() {
    type F = ark_bls12_381::Fr;
    let mut rng = ark_std::rand::thread_rng();

    let degree = 64;
    let poly_coeffs: Vec<F> = (0..=degree).map(|_| F::rand(&mut rng)).collect();
    let poly = DensePolynomial::from_coefficients_vec(poly_coeffs);

    let n = 128;
    let omega = F::get_root_of_unity(n).unwrap();
    let roots: Vec<F> = (0..n).map(|i| omega.pow([i as u64])).collect();
    let evals: Vec<F> = roots.iter().map(|&root| poly.evaluate(&root)).collect();
    let n_inv = F::from(n as u64).inverse().unwrap();

    for _ in 0..20 {
        let x = F::rand(&mut rng);
        let expected = poly.evaluate(&x);
        let val = barycentric_eval(&evals, &roots, x, n_inv);
        assert_eq!(val, expected);
    }
}
