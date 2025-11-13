use ark_ec::{short_weierstrass::{Affine, SWCurveConfig}, AffineRepr, VariableBaseMSM as _};
use ark_ff::{Fp, FpConfig, PrimeField};



pub trait WeightedSum: Copy {
    type Scalar: PrimeField;

    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self;
}

impl<const N: usize, P: FpConfig<N>> WeightedSum for Fp<P,N> {
    type Scalar = Fp<P,N>;

    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self {
        assert_eq!(bases.len(), scalars.len());

        bases.into_iter()
            .zip(scalars)
            .map(|(b,s)| b*s)
            .sum()
            
    }
}

impl<P: SWCurveConfig> WeightedSum for Affine<P> {
    type Scalar = P::ScalarField;

    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self {
        <Self as AffineRepr>::Group::msm(&bases, &scalars)
            .expect("MSM failed weighted_sum()")
            .into()
    }
}
