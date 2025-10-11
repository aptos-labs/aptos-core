// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A `Map` represents a general function from a `Domain` to a `Codomain`.
///
/// In the context of **constructing** a sigma proof, the essential ingredient is a **homomorphism**:
/// a map that preserves some algebraic structure.
pub trait Map {
    type Domain;
    type Codomain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain;
}

/// Given a set with an obvious projection map to the `Domain` of a `Map`,
/// `LiftMap` produces a new map from `LargerDomain` by composing the projection with the original map.
pub struct LiftMap<M, LargerDomain>
where
    M: Map,
{
    pub map: M,
    pub projection_map: fn(&LargerDomain) -> M::Domain,
}

impl<M, LargerDomain> Map for LiftMap<M, LargerDomain>
where
    M: Map,
{
    type Codomain = M::Codomain;
    type Domain = LargerDomain;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let smaller = (self.projection_map)(input);
        self.map.apply(&smaller)
    }
}

/// Given two maps with the same domain, `DiagonalProductMap` produces a new map
/// from that domain to a pair of codomains by applying both maps to the same input.
///
/// Conceptually, this is the “diagonal product”: for an input `x` in the domain,
/// the resulting map produces `(map1(x), map2(x))`.
pub struct DiagonalProductMap<M1, M2>
where
    M1: Map,
    M2: Map<Domain = M1::Domain>,
{
    pub map1: M1,
    pub map2: M2,
}

impl<M1, M2> Map for DiagonalProductMap<M1, M2>
where
    M1: Map,
    M2: Map<Domain = M1::Domain>,
{
    type Codomain = (M1::Codomain, M2::Codomain);
    type Domain = M1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        (self.map1.apply(x), self.map2.apply(x))
    }
}

/// A `FixedBaseMSM` represents a map whose codomain consists of one of more fixed-base
/// multi-scalar multiplications (MSMs).
///
/// In the context of a sigma protocol, when the homomorphism only computes MSMs,
/// the resulting equations can be verified efficiently in a “batch” using a variant of
/// the Schwartz-Zippel lemma. Doing so requires iterating over both the MSMs
/// and the protocol’s proof and public statement in a uniform way.
pub trait FixedBaseMSM: Map {
    type Scalar;
    type Base;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)>;

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base>;
}

impl<M, LargerDomain> FixedBaseMSM for LiftMap<M, LargerDomain>
where
    M: FixedBaseMSM,
{
    type Base = M::Base;
    type Scalar = M::Scalar;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        let smaller = (self.projection_map)(input);
        self.map.msm_rows(&smaller)
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        self.map.flatten_codomain(output)
    }
}

impl<M1, M2> FixedBaseMSM for DiagonalProductMap<M1, M2>
where
    M1: FixedBaseMSM,
    M2: FixedBaseMSM<Domain = M1::Domain, Scalar = M1::Scalar, Base = M1::Base>,
{
    type Base = M1::Base;
    type Scalar = M1::Scalar;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        let mut rows = self.map1.msm_rows(input);
        rows.extend(self.map2.msm_rows(input));
        rows
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        let (c1, c2) = output; // output: &(M1::Codomain, M2::Codomain)
        let mut flat = self.map1.flatten_codomain(c1);
        flat.extend(self.map2.flatten_codomain(c2));
        flat
    }
}
