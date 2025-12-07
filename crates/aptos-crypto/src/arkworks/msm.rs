// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Utilities for representing inputs to multi-scalar multiplications (MSMs).
//!
//! An MSM takes a collection of bases and corresponding scalars and computes
//! their linear combination. This module defines a simple container for such
//! inputs, along with a small trait to abstract over concrete container types.

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// Input to a (not necessarily fixed-base) multi-scalar multiplication (MSM).
///
/// An MSM input consists of:
/// * a list of base elements, and
/// * a list of scalar elements,
/// which are interpreted pairwise.  
///
/// Implementations that construct an `MsmInput` should ensure that 
/// `bases.len() == scalars.len()`
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct MsmInput<
    B: CanonicalSerialize + CanonicalDeserialize,
    S: CanonicalSerialize + CanonicalDeserialize,
> {
    /// the bases of the MSM
    pub bases: Vec<B>,
    /// the scalars of the MSM
    pub scalars: Vec<S>,
}

/// Trait abstraction for types representing MSM inputs.
///
/// This exists as a workaround because stable Rust does not yet support default
/// associated types. (Now we have to do `type MsmInput: IsMsmInput` rather than `type MsmInput = IsMsmInput`)
pub trait IsMsmInput<B, S> {
    /// Returns a reference to the slice of base elements in this MSM input.
    fn bases(&self) -> &[B];

    /// Returns a reference to the slice of scalar elements in this MSM input.
    fn scalars(&self) -> &[S];
}

impl<
        B: CanonicalSerialize + CanonicalDeserialize,
        S: CanonicalSerialize + CanonicalDeserialize,
    > IsMsmInput<B, S> for MsmInput<B, S>
{
    fn bases(&self) -> &[B] {
        &self.bases
    }

    fn scalars(&self) -> &[S] {
        &self.scalars
    }
}