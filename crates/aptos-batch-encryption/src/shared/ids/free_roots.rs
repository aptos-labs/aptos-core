use std::collections::HashMap;

use crate::{group::{Fr, G1Affine, G1Projective, G2Affine, PairingSetting}, shared::algebra::{fk_algorithm::EPTest as _, interpolate::vanishing_poly, mult_tree::{compute_mult_tree, quotient}}};
use crate::shared::ark_serialize::*;
use ark_ec::{pairing::Pairing, AffineRepr, ScalarMul, VariableBaseMSM};
use ark_std::rand::seq::index;
use ed25519_dalek::VerifyingKey;
use num_traits::{Zero, One};
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, EvaluationDomain, Radix2EvaluationDomain};
use ark_ff::{field_hashers::{DefaultFieldHasher, HashToField}, Field as _, PrimeField as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{shared::algebra::fk_algorithm::FKDomain, shared::algebra::interpolate::interpolate};

use super::{Id, IdSet};


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
    type Set = FreeRootIdSet;

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
pub struct FreeRootIdSet {
    pub poly_roots: Vec<Fr>,
    capacity: usize,
    cached_poly_coeffs: Option<Vec<Fr>>,
    cached_mult_tree: Option<Vec<Vec<DensePolynomial<Fr>>>>,
}




impl IdSet for FreeRootIdSet {
    type Id = FreeRootId;

    fn with_capacity(capacity: usize) -> Option<Self> {
        let capacity = capacity.next_power_of_two();
        Some(Self {
            poly_roots: Vec::new(),
            capacity,
            cached_poly_coeffs: None,
            cached_mult_tree: None,
        })
    }

    fn capacity(&self) -> usize {
        self.capacity
    }


    fn add(&mut self, id: &FreeRootId) {
        if self.poly_roots.len() >= self.capacity {
            // TODO real error handling
            panic!("put a real error here.");
        }
        self.poly_roots.push(id.root_x);
        self.cached_poly_coeffs = None;
    }

    fn compute_poly_coeffs(&mut self) {
        let mult_tree = compute_mult_tree(&self.poly_roots);
        self.cached_poly_coeffs = Some(mult_tree[mult_tree.len()-1][0].coeffs.clone());
        self.cached_mult_tree = Some(mult_tree);
    }

    fn poly_coeffs(&self) -> Vec<Fr> {
        if self.cached_poly_coeffs.is_none() {
            panic!("Need to compute first");
        }
        self.cached_poly_coeffs.clone().unwrap()
    }

    fn compute_all_eval_proofs_with_setup(&self, setup: &crate::shared::digest::DigestKey, round: usize) -> HashMap<Self::Id, G1Affine> {
        let pfs : Vec<G1Affine> = setup.fk_domain
            .eval_proofs_at_x_coords_alt(&self.poly_coeffs(), &self.poly_roots, round)
            .iter()
            .map(|g| G1Affine::from(*g))
            .collect();
        HashMap::from_iter(self.as_vec().into_iter().zip(pfs).collect::<Vec<(Self::Id, G1Affine)>>().into_iter())
    }

    fn compute_eval_proofs_with_setup(&self, setup: &crate::shared::digest::DigestKey, ids: &[Self::Id], round: usize) -> HashMap<Self::Id, G1Affine> {
        let pfs : Vec<G1Affine> = setup.fk_domain
            .eval_proofs_at_x_coords_alt(&self.poly_coeffs(), &ids.into_iter().map(|id| id.x()).collect::<Vec<Fr>>(), round)
            .iter()
            .map(|g| G1Affine::from(*g))
            .collect();
        HashMap::from_iter(self.as_vec().into_iter().zip(pfs).collect::<Vec<(Self::Id, G1Affine)>>().into_iter())
    }

    fn as_vec(&self) -> Vec<Self::Id> {
        self.poly_roots.iter()
            .map(|root_x| FreeRootId::new(*root_x))
            .collect()
    }

    fn compute_eval_proof_with_setup(&self, setup: &crate::shared::digest::DigestKey, id: Self::Id, round: usize) -> G1Affine {
        if self.cached_mult_tree.is_none() {
            panic!("Need to compute first");
        }

        let index_of_id = self.poly_roots.iter().position(|x| id.x() == *x).unwrap();

        let mut q_coeffs = quotient(self.cached_mult_tree.as_ref().unwrap(), index_of_id).coeffs;
        q_coeffs.push(Fr::zero());


        G1Projective::msm(&setup.tau_powers_g1[round], &q_coeffs).unwrap().into()
    }
}
