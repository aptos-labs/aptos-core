// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

pub mod fixedbasemsms;
pub mod tuple;

/// A `Homomorphism` represents a structure-preserving map between algebraic objects.
///
/// Formally, it is a function from a `Domain` to a `Codomain` that preserves
/// the relevant algebraic structure (e.g. group, ring, or module operations).
///
/// In the context of sigma protocols, homomorphisms are the key building blocks:
/// they capture the algebraic relations that proofs are designed to demonstrate.
pub trait Trait {
    type Domain;
    type Codomain;

    fn apply(&self, element: &Self::Domain) -> Self::Codomain;
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
/// but we don't need that formalism for now.
pub struct LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    pub hom: H,
    pub projection: fn(&LargerDomain) -> H::Domain,
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
    type Domain = LargerDomain;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let projected = (self.projection)(input);
        self.hom.apply(&projected)
    }
}

/// A trait for types that support **entrywise mapping** over their contents.
///
/// Given a value of this type, you can apply a function to each "entry" independently,
/// producing a new value of the same shape but possibly with a different inner type.
pub trait EntrywiseMap<T> {
    /// The resulting type after mapping the inner elements to type `U`.
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone;
}

/// A trivial wrapper type for a single value. Typically used when the Codomain would've been E::G1
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone)]
pub struct TrivialShape<T: CanonicalSerialize + CanonicalDeserialize + Clone>(pub T);

/// Implements `EntrywiseMap` for `TrivialShape`, mapping the inner value.
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> EntrywiseMap<T> for TrivialShape<T> {
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone> = TrivialShape<U>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone,
    {
        TrivialShape(f(self.0))
    }
}

/// Implements `IntoIterator` for `TrivialShape`, producing a single-element iterator.
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> IntoIterator for TrivialShape<T> {
    type IntoIter = std::iter::Once<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0)
    }
}
