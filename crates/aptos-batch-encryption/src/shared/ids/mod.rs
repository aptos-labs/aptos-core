use std::{collections::HashMap, hash::Hash};

use crate::group::{Fr, G1Affine};
use ed25519_dalek::VerifyingKey;
use anyhow::Result;

pub mod fft_domain;
pub mod free_roots;

pub use fft_domain::{FFTDomainId, FFTDomainIdSet};
pub use free_roots::{FreeRootIdSet, FreeRootId};


pub trait Id : Send + Sync + Eq + PartialEq + Clone + Copy + Hash {
    type Set : IdSet<Id = Self>;

    fn x(&self) -> Fr;
    fn y(&self) -> Fr;

    fn from_verifying_key(vk: &VerifyingKey) -> Self;
}

/// A set of IDs that can be committed to as a KZG polynomial commitment.
pub trait IdSet : Clone + Send + Sync {
    type Id : Id<Set = Self>;
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
    fn compute_poly_coeffs(&mut self);
    /// The coefficients of the polynomial to be committed to which encodes this set. Must call
    /// [`IdSet::compute_poly_coeffs`] before calling this.
    fn poly_coeffs(&self) -> Option<Vec<Fr>>;
    /// Given a [`DigestKey`], compute all KZG evaluation proofs for the polynomial that encodes
    /// this set with respect to this setup.
    fn compute_all_eval_proofs_with_setup(&self, setup: &super::digest::DigestKey, round: usize) -> Result<HashMap<Self::Id, G1Affine>>;
    fn compute_eval_proofs_with_setup(&self, setup: &super::digest::DigestKey, ids: &[Self::Id], round: usize) -> Result<HashMap<Self::Id, G1Affine>>;
    fn compute_eval_proof_with_setup(&self, setup: &super::digest::DigestKey, id: Self::Id, round: usize) -> Result<G1Affine>;
    //fn compute_single_eval_proof_with_setup(&self, id: Self::Id, setup: &super::digest::DigestKey, round: usize) -> HashMap<Self::Id, G1Affine>;
    // TODO start here next time. Think about interface here for computing 1) a single proof, 2)
    // multiple proofs, and 3) all proofs. Also think about how to make FK work well for
    // not-all-proofs.
    // Finally, think about changing to new variant w/ randomized setups. DONE


    fn as_vec(&self) -> Vec<Self::Id>;
} 

