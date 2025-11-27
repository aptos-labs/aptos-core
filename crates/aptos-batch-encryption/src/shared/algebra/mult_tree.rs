use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};

pub fn compute_mult_tree<F: FftField>(roots: &[F]) -> Vec<Vec<DensePolynomial<F>>> {
    let mut bases: Vec<DensePolynomial<F>> = roots
        .into_iter()
        .cloned()
        .map(|u| DenseUVPolynomial::from_coefficients_vec(vec![-u, F::one()]))
        .collect();

    bases.resize(
        bases.len().next_power_of_two(),
        DenseUVPolynomial::from_coefficients_vec(vec![F::one()]),
    );

    let num_leaves = bases.len();
    let mut result = vec![bases];
    let depth = num_leaves.ilog2();
    assert_eq!(2usize.pow(depth), num_leaves);

    for i in 1..=(num_leaves.ilog2() as usize) {
        let len_at_i = 2usize.pow(depth as u32 - i as u32);
        let result_at_i = (0..len_at_i)
            .into_par_iter()
            .map(|j| result[i - 1][2 * j].clone() * &result[i - 1][2 * j + 1])
            .collect();
        result.push(result_at_i);
    }

    result
}

pub fn quotient<F: FftField>(
    mult_tree: &Vec<Vec<DensePolynomial<F>>>,
    divisor_index: usize,
) -> DensePolynomial<F> {
    let mut mult_tree = mult_tree.clone();
    mult_tree[0][divisor_index] = DenseUVPolynomial::from_coefficients_vec(vec![F::one()]);
    let depth = mult_tree.len();

    let mut subtree_with_divisor = divisor_index;

    for i in 1..depth {
        subtree_with_divisor /= 2;
        mult_tree[i][subtree_with_divisor] = mult_tree[i - 1][2 * subtree_with_divisor].clone()
            * &mult_tree[i - 1][2 * subtree_with_divisor + 1];
    }

    mult_tree[depth - 1][0].clone()
}

#[cfg(test)]
mod tests {
    use super::compute_mult_tree;
    use crate::{group::Fr, shared::algebra::mult_tree::quotient};
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
    use ark_std::{rand::thread_rng, One, UniformRand};

    #[test]
    fn test_mult_tree() {
        let mut rng = thread_rng();

        for num_roots in 1..=16 {
            let frs: Vec<Fr> = (0..num_roots).map(|_| Fr::rand(&mut rng)).collect();
            let mult_tree = compute_mult_tree(&frs);

            // naive computation of root of tree
            let result: DensePolynomial<Fr> = frs
                .into_iter()
                .map(|u| DenseUVPolynomial::from_coefficients_vec(vec![-u, Fr::one()]))
                .reduce(|acc, f| acc * f)
                .unwrap();

            assert_eq!(result, mult_tree.into_iter().last().unwrap()[0]);
        }
    }

    #[test]
    fn test_quotient() {
        let mut rng = thread_rng();

        for num_roots in 2..=16 {
            let mult_tree = compute_mult_tree(
                &(0..num_roots)
                    .map(|_| Fr::rand(&mut rng))
                    .collect::<Vec<Fr>>(),
            );

            let vanishing_poly = &mult_tree[mult_tree.len() - 1][0];

            for i in 0..num_roots {
                let divisor = &mult_tree[0][i];
                let quotient = quotient(&mult_tree, i);

                assert_eq!(quotient * divisor, *vanishing_poly);
            }
        }
    }
}
