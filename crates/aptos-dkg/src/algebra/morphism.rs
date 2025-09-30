// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub trait Morphism {
    type Domain;
    type Codomain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain;
}

pub struct LiftMorphism<M, LargerDomain>
where
    M: Morphism,
{
    pub morphism: M,
    pub projection_map: fn(&LargerDomain) -> M::Domain,
}

impl<M, LargerDomain> Morphism for LiftMorphism<M, LargerDomain>
where
    M: Morphism,
{
    type Codomain = M::Codomain;
    type Domain = LargerDomain;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let smaller = (self.projection_map)(input);
        self.morphism.apply(&smaller)
    }
}

pub struct DiagonalProductMorphism<M1, M2>
where
    M1: Morphism,
    M2: Morphism<Domain = M1::Domain>,
{
    pub morphism1: M1,
    pub morphism2: M2,
}

impl<M1, M2> Morphism for DiagonalProductMorphism<M1, M2>
where
    M1: Morphism,
    M2: Morphism<Domain = M1::Domain>,
{
    type Codomain = (M1::Codomain, M2::Codomain);
    type Domain = M1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        (self.morphism1.apply(x), self.morphism2.apply(x))
    }
}

pub trait FixedBaseMSM: Morphism {
    type Scalar;
    type Base;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)>;

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base>;
}

impl<M, LargerDomain> FixedBaseMSM for LiftMorphism<M, LargerDomain>
where
    M: FixedBaseMSM,
{
    type Base = M::Base;
    type Scalar = M::Scalar;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        let smaller = (self.projection_map)(input);
        self.morphism.msm_rows(&smaller)
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        self.morphism.flatten_codomain(output)
    }
}

impl<M1, M2> FixedBaseMSM for DiagonalProductMorphism<M1, M2>
where
    M1: FixedBaseMSM,
    M2: FixedBaseMSM<Domain = M1::Domain, Scalar = M1::Scalar, Base = M1::Base>,
{
    type Base = M1::Base;
    type Scalar = M1::Scalar;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        let mut rows = self.morphism1.msm_rows(input);
        rows.extend(self.morphism2.msm_rows(input));
        rows
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        let (c1, c2) = output; // output: &(M1::Codomain, M2::Codomain)
        let mut flat = self.morphism1.flatten_codomain(c1);
        flat.extend(self.morphism2.flatten_codomain(c2));
        flat
    }
}
