use ark_ff::FftField;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};

pub trait DifferentiableFn {
    fn differentiate(&self) -> Self;
}

impl<F: FftField> DifferentiableFn for DensePolynomial<F> {
    fn differentiate(&self) -> Self {
        let result_coeffs: Vec<F> = self
            .coeffs()
            .into_iter()
            .skip(1)
            .enumerate()
            .map(|(i, x)| *x * F::from(i as u64 + 1))
            .collect();

        Self::from_coefficients_vec(result_coeffs)
    }
}

#[cfg(test)]
mod tests {
    use super::DifferentiableFn;
    use crate::group::Fr;
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
    use ark_std::One;

    #[test]
    fn test_differentiate() {
        let p = DensePolynomial::from_coefficients_vec(vec![Fr::one(), Fr::one()]);
        let d = p.differentiate();
        assert_eq!(d.coeffs, vec![Fr::one()]);
    }

    #[test]
    fn test_differentiate_2() {
        let p = DensePolynomial::from_coefficients_vec(vec![Fr::one(), Fr::from(2), Fr::from(3)]);
        let d = p.differentiate();
        assert_eq!(d.coeffs, vec![Fr::from(2), Fr::from(6)]);
    }
}
