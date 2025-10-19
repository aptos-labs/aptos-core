use aptos_dkg::algebra::polynomials::barycentric_eval;
use ark_ec::{pairing::Pairing, AdditiveGroup};
use ark_ff::{FftField, Field, UniformRand};
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, Polynomial};
use ark_std::{rand::Rng, test_rng};

#[cfg(test)]
fn run_barycentric_case<E: Pairing>(degree: usize, n: usize, sample_points: usize) {
    let mut rng = test_rng();
    type Fr<E> = <E as Pairing>::ScalarField;

    // Generate coefficients for a random polynomial; if degree is 0, randomly sometimes replace with zero
    let poly_coeffs: Vec<Fr<E>> = if degree == 0 && rng.gen_bool(0.5) {
        vec![Fr::<E>::ZERO]
    } else {
        (0..=degree).map(|_| Fr::<E>::rand(&mut rng)).collect()
    };
    let poly = DensePolynomial::from_coefficients_vec(poly_coeffs);

    // Build domain and evaluations
    let omega = Fr::<E>::get_root_of_unity(n as u64)
        .unwrap_or_else(|| panic!("no root of unity of size {} for this field", n));
    let roots: Vec<Fr<E>> = (0..n).map(|i| omega.pow([i as u64])).collect();
    let evals: Vec<Fr<E>> = roots.iter().map(|&r| poly.evaluate(&r)).collect();
    let n_inv = Fr::<E>::from(n as u64).inverse().unwrap();

    // Random points
    for _ in 0..sample_points {
        let x = Fr::<E>::rand(&mut rng);
        let expected = poly.evaluate(&x);
        let val = barycentric_eval(&evals, &roots, x, n_inv);
        assert_eq!(
            val, expected,
            "Failed for degree {}, n = {} at x = {:?}",
            degree, n, x
        );
    }

    // Interpolation points (roots)
    for (root, &eval) in roots.iter().zip(evals.iter()) {
        let val = barycentric_eval(&evals, &roots, *root, n_inv);
        assert_eq!(
            val, eval,
            "Interpolation mismatch at root {:?} for degree {}, n = {}",
            root, degree, n
        );
    }
}

#[test]
fn test_barycentric_eval_bls12_381() {
    use ark_bls12_381::Bls12_381;
    use ark_bn254::Bn254;
    let cases = [
        (0, 1),
        (1, 2),
        (2, 4),
        (15, 16),
        (16, 32),
        (63, 64),
        (64, 128),
        (256, 512),
        (511, 512),
        (1023, 1024),
    ];

    for &(degree, n) in &cases {
        run_barycentric_case::<Bn254>(degree, n, 5);
    }
    for &(degree, n) in &cases {
        run_barycentric_case::<Bls12_381>(degree, n, 5);
    }
}
