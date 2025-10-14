use aptos_dkg::algebra::polynomials::barycentric_eval;
use ark_bls12_381::Fr;
use ark_ff::{FftField, Field};
use num_traits::{identities::One, Zero};

#[test]
fn test_barycentric_eval_linear() {
    // f(x) = x + 1 over 2nd roots of unity

    let roots = vec![Fr::one(), -Fr::one()]; // 1, -1
    let evals: Vec<Fr> = roots.iter().map(|&x| x + Fr::one()).collect();

    // Test at root points
    for (r, &f_r) in roots.iter().zip(evals.iter()) {
        let val = barycentric_eval(&evals, &roots, *r);
        assert_eq!(val, f_r);
    }

    // Test at z=0, should get 1
    let val = barycentric_eval(&evals, &roots, Fr::zero());
    assert_eq!(val, Fr::one());
}

#[test]
fn test_barycentric_eval_third_degree() {
    // f(x) = x^3 + x^2 + 1, 4th roots of unity
    let n = 4;

    let omega = Fr::get_root_of_unity(n).unwrap();
    let roots = [
        omega.pow([0u64]),
        omega.pow([1u64]),
        omega.pow([2u64]),
        omega.pow([3u64]),
    ]
    .to_vec();

    let evals: Vec<Fr> = roots
        .iter()
        .map(|&x| x * x * x + x * x + Fr::one())
        .collect();

    // Test at roots
    for (r, &f_r) in roots.iter().zip(evals.iter()) {
        let val = barycentric_eval(&evals, &roots, *r);
        assert_eq!(val, f_r);
    }
}
