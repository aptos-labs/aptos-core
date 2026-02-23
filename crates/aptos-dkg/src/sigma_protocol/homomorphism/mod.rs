// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Write,
};
use std::fmt::Debug;

pub mod fixed_base_msms;
pub mod tuple;

/// A `Homomorphism` represents a structure-preserving map between algebraic objects.
///
/// Formally, it is a function from a `Domain` to a `Codomain` that preserves
/// the relevant algebraic structure (e.g. group, ring, or module operations).
///
/// In the context of sigma protocols, homomorphisms are the key building blocks:
/// they capture the algebraic relations that proofs are designed to demonstrate.
///
/// The `Codomain` type represents the output of the homomorphism, which may admit
/// multiple equivalent representations (e.g. projective vs. affine group elements).
///
/// The associated type `CodomainNormalized` represents a *canonical* or
/// *normalized* form of the codomain element. This form is intended to be:
///
/// - uniquely determined,
/// - suitable for deterministic serialization, and
/// - stable for use as input to Fiat–Shamir transcripts and challenge
///   derivation.
///
/// The `normalize` method converts a `Codomain` value into this canonical
/// representation.
///
/// CanonicalSerialize is added here so the parameters of the homomorphism (which will
/// be MSM bases) can be used for Fiat-Shamir challenges.
pub trait Trait: CanonicalSerialize {
    type Domain;
    type Codomain;
    type CodomainNormalized;

    fn apply(&self, element: &Self::Domain) -> Self::Codomain;
    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized;
}

/// `LiftHomomorphism` adapts a homomorphism `H` defined on some `Domain`
/// so that it can act on a larger `LargerDomain` by precomposing `H`
/// with a natural projection map `π`, which should also be a homomorphism.
///
/// In other words, given:
/// - a homomorphism `h: Domain -> Codomain`
/// - another homomorphism `π: LargerDomain -> Domain`
///
/// `LiftHomomorphism` represents the composed homomorphism:
/// `h ∘ π : LargerDomain -> Codomain`.
///
/// # Example
///
/// A common case is when `LargerDomain` is a Cartesian product type like `X × Y`
/// and the projection is `(x, y) ↦ x`. Then `LiftHomomorphism`
/// lets `h` act on the first component of the pair, so `(h ∘ π)(x,y) = h(x)`.
///
/// Naturally this method immediately extends to composing arbitrary homomorphisms,
/// but we don't need that formalism for now. We are not deriving Eq here because
/// function pointer comparisons do not seem useful in this context.
#[derive(Debug, Clone)]
pub struct LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    pub hom: H,
    pub projection: fn(&LargerDomain) -> H::Domain,
}

// We only care about the "bases" that are being used to define the homomorphism, so in serializing
// we ignore `projection` entirely
impl<H, LargerDomain> CanonicalSerialize for LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.hom.serialize_with_mode(&mut writer, compress)
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        self.hom.serialized_size(compress)
    }
}

/// Implements `Homomorphism` for `LiftHomomorphism` by composing
/// the original homomorphism with the projection.
///
/// That is, applying `LiftHomomorphism` to an input `x` is equivalent to
/// first computing `projection(x)` and then applying `hom` to the result.
impl<H, LargerDomain> Trait for LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    type Codomain = H::Codomain;
    type CodomainNormalized = H::CodomainNormalized;
    type Domain = LargerDomain;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let projected = (self.projection)(input);
        self.hom.apply(&projected)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        H::normalize(&self.hom, value)
    }
}

/// A trait for types that support **entrywise mapping** over their contents.
///
/// Given a value of this type, you can apply a function to each "entry" independently,
/// producing a new value of the same shape but possibly with a different inner type.
pub trait EntrywiseMap<T> {
    /// The resulting type after mapping the inner elements to type `U`.
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
}

// ===============================================================================
// ============================= BEGIN: TRIVIAL SHAPE ============================
// ===============================================================================

/// A trivial wrapper type for a single value. Should be used to wrap when the codomain of a homomorphism is something like E::G1
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct TrivialShape<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq>(pub T);

/// Implements `EntrywiseMap` for `TrivialShape`, mapping the inner value.
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> EntrywiseMap<T>
    for TrivialShape<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        TrivialShape<U>;

    fn map<U, F>(self, mut f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        TrivialShape(f(self.0))
    }
}

/// Implements `IntoIterator` for `TrivialShape`, producing a single-element iterator.
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> IntoIterator
    for TrivialShape<T>
{
    type IntoIter = std::iter::Once<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0)
    }
}

// ===============================================================================
// ============================= END: TRIVIAL SHAPE ==============================
// ===============================================================================

/// Builds a domain-separation tag for a composite homomorphism by prefixing each
/// sub-DST with its length (big-endian), so e.g. `[a|b]` and `[ab|]` do not collide.
#[inline]
pub fn domain_separate_dsts(prefix: &[u8], parts: &[Vec<u8>], suffix: &[u8]) -> Vec<u8> {
    let mut out = Vec::from(prefix);
    for p in parts {
        out.extend_from_slice(&(p.len() as u32).to_be_bytes());
        out.extend_from_slice(p);
    }
    out.extend_from_slice(suffix);
    out
}
