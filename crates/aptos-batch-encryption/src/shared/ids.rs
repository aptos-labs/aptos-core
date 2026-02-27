// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    group::{Fr, G1Affine, G1Projective},
    shared::algebra::mult_tree::{compute_mult_tree, quotient},
};
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use ark_ec::VariableBaseMSM as _;
use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
use ark_poly::univariate::DensePolynomial;
use ark_std::{One, Zero};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{collections::HashMap, hash::Hash};

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct Id {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    root_x: Fr,
}

impl Id {
    pub fn one() -> Self {
        Self::new(Fr::one())
    }

    pub fn new(root_x: Fr) -> Self {
        Self { root_x }
    }

    pub fn x(&self) -> Fr {
        self.root_x
    }

    pub fn from_verifying_key(vk: &VerifyingKey) -> Self {
        // using empty domain separator b/c this is a test implementation
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fr>>::new(&[]);
        let field_element: [Fr; 1] = field_hasher.hash_to_field::<1>(&vk.to_bytes());
        Self::new(field_element[0])
    }
}

/// A set of IDs that is encoded via arbitrary roots.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct IdSet<Coeffs> {
    pub poly_roots: Vec<Fr>,
    capacity: usize,
    poly_coeffs: Coeffs,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct UncomputedCoeffs;
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct ComputedCoeffs {
    coeffs: Vec<Fr>,
    mult_tree: Vec<Vec<DensePolynomial<Fr>>>,
}

impl IdSet<UncomputedCoeffs> {
    pub fn from_slice(ids: &[Id]) -> Self {
        let mut result = Self::with_capacity(ids.len());
        for id in ids {
            // Note: although add() can panic, it never should here b/c we initialized Self
            // with capcity equal to ids.len().
            result.add(id);
        }
        result
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        Self {
            poly_roots: Vec::new(),
            capacity,
            poly_coeffs: UncomputedCoeffs,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn add(&mut self, id: &Id) {
        if self.poly_roots.len() >= self.capacity {
            panic!("Number of ids must be less than capacity");
        }
        self.poly_roots.push(id.root_x);
    }

    pub fn compute_poly_coeffs(&self) -> IdSet<ComputedCoeffs> {
        let mult_tree = compute_mult_tree(&self.poly_roots);

        IdSet {
            poly_roots: self.poly_roots.clone(),
            capacity: self.capacity,
            poly_coeffs: ComputedCoeffs {
                coeffs: mult_tree[mult_tree.len() - 1][0].coeffs.clone(),
                mult_tree,
            },
        }
    }

    pub fn as_vec(&self) -> Vec<Id> {
        self.poly_roots
            .iter()
            .map(|root_x| Id::new(*root_x))
            .collect()
    }
}

impl IdSet<ComputedCoeffs> {
    pub fn as_vec(&self) -> Vec<Id> {
        self.poly_roots
            .iter()
            .map(|root_x| Id::new(*root_x))
            .collect()
    }

    pub fn poly_coeffs(&self) -> Vec<Fr> {
        self.poly_coeffs.coeffs.clone()
    }

    pub fn compute_all_eval_proofs_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        round: usize,
    ) -> HashMap<Id, G1Affine> {
        let pfs: Vec<G1Affine> = setup
            .fk_domain
            .eval_proofs_at_x_coords_naive_multi_point_eval(
                &self.poly_coeffs(),
                &self.poly_roots,
                round,
            )
            .iter()
            .map(|g| G1Affine::from(*g))
            .collect();

        HashMap::from_iter(
            self.as_vec()
                .into_iter()
                .zip(pfs)
                .collect::<Vec<(Id, G1Affine)>>(),
        )
    }

    pub fn compute_all_eval_proofs_with_setup_vzgg_multi_point_eval(
        &self,
        setup: &crate::shared::digest::DigestKey,
        round: usize,
    ) -> HashMap<Id, G1Affine> {
        let pfs: Vec<G1Affine> = setup
            .fk_domain
            .eval_proofs_at_x_coords(&self.poly_coeffs(), &self.poly_roots, round)
            .iter()
            .map(|g| G1Affine::from(*g))
            .collect();

        HashMap::from_iter(
            self.as_vec()
                .into_iter()
                .zip(pfs)
                .collect::<Vec<(Id, G1Affine)>>(),
        )
    }

    pub fn compute_eval_proofs_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        ids: &[Id],
        round: usize,
    ) -> HashMap<Id, G1Affine> {
        let pfs: Vec<G1Affine> = setup
            .fk_domain
            .eval_proofs_at_x_coords_naive_multi_point_eval(
                &self.poly_coeffs(),
                &ids.iter().map(|id| id.x()).collect::<Vec<Fr>>(),
                round,
            )
            .iter()
            .map(|g| G1Affine::from(*g))
            .collect();

        HashMap::from_iter(
            ids.iter()
                .cloned()
                .zip(pfs)
                .collect::<Vec<(Id, G1Affine)>>(),
        )
    }

    pub fn compute_eval_proof_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        id: Id,
        round: usize,
    ) -> G1Affine {
        let index_of_id = self.poly_roots.iter().position(|x| id.x() == *x).unwrap();

        let mut q_coeffs = quotient(&self.poly_coeffs.mult_tree, index_of_id).coeffs;
        q_coeffs.push(Fr::zero());

        G1Projective::msm(&setup.tau_powers_g1[round], &q_coeffs)
            .unwrap()
            .into()
    }
}
