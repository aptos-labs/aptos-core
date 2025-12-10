// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::{Id, IdSet, OssifiedIdSet};
use crate::{
    group::{Fr, G1Affine, G1Projective},
    shared::{
        algebra::mult_tree::{compute_mult_tree, quotient},
        serialize::arkworks::*,
    },
};
use ark_ec::VariableBaseMSM;
use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
use ark_poly::univariate::DensePolynomial;
use ed25519_dalek::VerifyingKey;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;

/// An ID in an [`ArbXIdSet`].
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct FreeRootId {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    root_x: Fr,
}

impl FreeRootId {
    pub fn new(root_x: Fr) -> Self {
        Self { root_x }
    }
}

impl Id for FreeRootId {
    type OssifiedSet = FreeRootIdSet<ComputedCoeffs>;
    type Set = FreeRootIdSet<UncomputedCoeffs>;

    fn x(&self) -> Fr {
        self.root_x
    }

    fn y(&self) -> Fr {
        Fr::zero()
    }

    fn from_verifying_key(vk: &VerifyingKey) -> Self {
        // using empty domain separator b/c this is a test implementation
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fr>>::new(&[]);
        let field_element: [Fr; 1] = field_hasher.hash_to_field::<1>(&vk.to_bytes());
        FreeRootId::new(field_element[0])
    }
}

/// A set of IDs that is encoded via arbitrary points. Evaluation proof computation is
/// slower than [`FFTDomainIdSet`], but allows for creating IDs over a large space with
/// low probability of collision.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FreeRootIdSet<Coeffs> {
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

impl IdSet for FreeRootIdSet<UncomputedCoeffs> {
    type Id = FreeRootId;
    type OssifiedSet = FreeRootIdSet<ComputedCoeffs>;

    fn with_capacity(capacity: usize) -> Option<Self> {
        let capacity = capacity.next_power_of_two();
        Some(Self {
            poly_roots: Vec::new(),
            capacity,
            poly_coeffs: UncomputedCoeffs,
        })
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    fn add(&mut self, id: &FreeRootId) {
        if self.poly_roots.len() >= self.capacity {
            panic!("Number of ids must be less than capacity");
        }
        self.poly_roots.push(id.root_x);
    }

    fn compute_poly_coeffs(&self) -> FreeRootIdSet<ComputedCoeffs> {
        let mult_tree = compute_mult_tree(&self.poly_roots);

        FreeRootIdSet {
            poly_roots: self.poly_roots.clone(),
            capacity: self.capacity,
            poly_coeffs: ComputedCoeffs {
                coeffs: mult_tree[mult_tree.len() - 1][0].coeffs.clone(),
                mult_tree,
            },
        }
    }

    fn as_vec(&self) -> Vec<Self::Id> {
        self.poly_roots
            .iter()
            .map(|root_x| FreeRootId::new(*root_x))
            .collect()
    }
}

impl OssifiedIdSet for FreeRootIdSet<ComputedCoeffs> {
    type Id = FreeRootId;

    fn as_vec(&self) -> Vec<Self::Id> {
        self.poly_roots
            .iter()
            .map(|root_x| FreeRootId::new(*root_x))
            .collect()
    }

    fn poly_coeffs(&self) -> Vec<Fr> {
        self.poly_coeffs.coeffs.clone()
    }

    fn compute_all_eval_proofs_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        round: usize,
    ) -> HashMap<Self::Id, G1Affine> {
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
                .collect::<Vec<(Self::Id, G1Affine)>>(),
        )
    }

    fn compute_all_eval_proofs_with_setup_vzgg_multi_point_eval(
        &self,
        setup: &crate::shared::digest::DigestKey,
        round: usize,
    ) -> HashMap<Self::Id, G1Affine> {
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
                .collect::<Vec<(Self::Id, G1Affine)>>(),
        )
    }

    fn compute_eval_proofs_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        ids: &[Self::Id],
        round: usize,
    ) -> HashMap<Self::Id, G1Affine> {
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
                .collect::<Vec<(Self::Id, G1Affine)>>(),
        )
    }

    fn compute_eval_proof_with_setup(
        &self,
        setup: &crate::shared::digest::DigestKey,
        id: Self::Id,
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
