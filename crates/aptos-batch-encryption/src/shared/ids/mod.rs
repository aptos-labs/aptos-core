// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::group::{Fr, G1Affine};
use ed25519_dalek::VerifyingKey;
use std::{collections::HashMap, hash::Hash};

pub mod free_roots;

pub use free_roots::{FreeRootId, FreeRootIdSet};
use serde::Serialize;

pub trait Id: Send + Sync + Eq + PartialEq + Clone + Copy + Hash + Serialize {
    type Set: IdSet<Id = Self>;
    type OssifiedSet: OssifiedIdSet<Id = Self>;

    fn x(&self) -> Fr;
    fn y(&self) -> Fr;

    fn from_verifying_key(vk: &VerifyingKey) -> Self;
}

/// A set of IDs that can be committed to as a KZG polynomial commitment.
pub trait IdSet: Clone + Send + Sync {
    type Id: Id<Set = Self>;
    type OssifiedSet: OssifiedIdSet<Id = Self::Id>;
    fn with_capacity(capacity: usize) -> Option<Self>;
    /// Maximum number of IDs which can be added to this set. Each set is initialized with such
    /// a capacity that should mirror the KZG setup size.
    fn capacity(&self) -> usize;
    /// Add an id to this set.
    fn add(&mut self, id: &Self::Id);
    /// Compute the coefficients of the polynomial to be committed to which encodes this set. Must
    /// be called before [`IdSet::poly_coeffs`].
    fn from_slice(ids: &[Self::Id]) -> Option<Self> {
        let mut result = Self::with_capacity(ids.len())?;
        for id in ids {
            result.add(id);
        }
        Some(result)
    }
    fn compute_poly_coeffs(&self) -> Self::OssifiedSet;

    fn as_vec(&self) -> Vec<Self::Id>;
}

pub trait OssifiedIdSet {
    type Id: Id<OssifiedSet = Self>;
    fn as_vec(&self) -> Vec<Self::Id>;
    /// The coefficients of the polynomial to be committed to which encodes this set. Must call
    /// [`IdSet::compute_poly_coeffs`] before calling this.
    fn poly_coeffs(&self) -> Vec<Fr>;
    /// Given a [`DigestKey`], compute all KZG evaluation proofs for the polynomial that encodes
    /// this set with respect to this setup.
    fn compute_all_eval_proofs_with_setup(
        &self,
        setup: &super::digest::DigestKey,
        round: usize,
    ) -> HashMap<Self::Id, G1Affine>;
    /// Given a [`DigestKey`], compute all KZG evaluation proofs for the polynomial that encodes
    /// this set with respect to this setup. This version uses a different (slower for our
    /// parameter regime) multi-point-eval algorithm, from von zur Gathen and Gerhardt.
    fn compute_all_eval_proofs_with_setup_vzgg_multi_point_eval(
        &self,
        setup: &super::digest::DigestKey,
        round: usize,
    ) -> HashMap<Self::Id, G1Affine>;
    fn compute_eval_proofs_with_setup(
        &self,
        setup: &super::digest::DigestKey,
        ids: &[Self::Id],
        round: usize,
    ) -> HashMap<Self::Id, G1Affine>;
    fn compute_eval_proof_with_setup(
        &self,
        setup: &super::digest::DigestKey,
        id: Self::Id,
        round: usize,
    ) -> G1Affine;
}
