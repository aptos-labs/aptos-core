use std::collections::HashMap;

use crate::group::{PairingSetting, Fr, G1Affine, G1Projective, G2Affine};
use ark_ec::{pairing::Pairing, AffineRepr, ScalarMul, VariableBaseMSM};
use ed25519_dalek::VerifyingKey;
use num_traits::{Zero, One};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_ff::{field_hashers::{DefaultFieldHasher, HashToField}, Field as _, PrimeField as _};
use sha2::Sha256;

use crate::{shared::algebra::fk_algorithm::FKDomain, shared::algebra::interpolate::interpolate};

use super::{Id, IdSet};

/// An ID in a [`RootsOfUnityIdSet`].
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub struct FFTDomainId<const N: usize> {
    eval_domain: Radix2EvaluationDomain<Fr>,
    x_index: usize,
    y: Fr
}

impl<const N: usize> FFTDomainId<N> {
    pub fn new(id_set: &FFTDomainIdSet<N>, x_index: usize, y: Fr) -> Self {
        Self {
            eval_domain: id_set.eval_domain.clone(),
            x_index: x_index % id_set.eval_domain.size(),
            y,
        }
    }

    pub fn new_with_id_set_capacty(capacity: usize, x_index: usize, y: Fr) -> Self {
        Self {
            // using unwrap here b/c not going to use this in production
            eval_domain: Radix2EvaluationDomain::new(capacity).unwrap(),
            x_index: x_index % capacity,
            y,
        }
    }

    pub fn x_index(&self) -> usize { self.x_index }
}

impl<const N: usize> Id for FFTDomainId<N> {
    type Set = FFTDomainIdSet<N>;

    fn x(&self) -> Fr { self.eval_domain.group_gen().pow(&[self.x_index as u64]) }
    fn y(&self) -> Fr { self.y }


    fn from_verifying_key(vk: &VerifyingKey) -> Self {
        // using empty domain separator b/c this is a test implementation
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fr>>::new(&[]);
        let field_elements : [Fr; 2] = field_hasher.hash_to_field::<2>(&vk.to_bytes());
        let x_index = field_elements[0].into_bigint().as_ref()[0] as usize;
        Self::new_with_id_set_capacty(N, x_index, field_elements[1])
    }
}


/// A set of IDs that is encoded via points on some FFT domain. Allows for very fast evaluation
/// proof computation, at the cost of high probability of ID collision.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct FFTDomainIdSet<const N: usize> {
    eval_domain: Radix2EvaluationDomain<Fr>,
    poly_evals: Vec<Fr>,
    cached_poly_coeffs: Option<Vec<Fr>>,
}

impl<const N: usize> FFTDomainIdSet<N> {

    pub fn set(&mut self, x: usize, y: Fr) {
        self.poly_evals[x] = y;
        self.cached_poly_coeffs = None;
    }



    pub fn field_to_x_element(&self, i: Fr) -> Fr {
        self.eval_domain.group_gen().pow(&i.into_bigint())
    }

    pub fn ith_x_element(&self, i: usize) -> Fr {
        self.eval_domain.group_gen().pow(&[i as u64])
    }
}

impl<const N: usize> IdSet for FFTDomainIdSet<N> {
    type Id = FFTDomainId<N>;

    fn with_capacity(capacity: usize) -> Option<Self> {
        let capacity = capacity.next_power_of_two();
        if capacity != N {
            None
        } else {
            Some(Self {
                eval_domain: Radix2EvaluationDomain::new(capacity)?,
                poly_evals: vec![Fr::zero(); capacity],
                cached_poly_coeffs: None,
            })
        }
    }

    fn capacity(&self) -> usize { 
        self.poly_evals.len()
    }

    fn add(&mut self, id: &Self::Id) {
        self.poly_evals[id.x_index] = id.y();
        self.cached_poly_coeffs = None;
    }

    fn compute_poly_coeffs(&mut self) {
        let mut coeffs = self.eval_domain.ifft(&self.poly_evals);
        coeffs.push(Fr::zero());
        self.cached_poly_coeffs = Some(coeffs);
    }

    fn poly_coeffs(&self) -> Vec<Fr> {
        if self.cached_poly_coeffs.is_none() {
            panic!("Need to compute first");
        }
        self.cached_poly_coeffs.clone().unwrap()
    }

    fn compute_all_eval_proofs_with_setup(&self, setup: &crate::shared::digest::DigestKey, round: usize) -> HashMap<Self::Id, G1Affine> {
        let pfs: Vec<(Self::Id, G1Affine)> = 
            self.as_vec().into_iter().zip(
            setup.fk_domain.eval_proofs_at_roots_of_unity(&self.poly_coeffs(), round)
            .iter().map(|g| G1Affine::from(*g))).collect();

        HashMap::from_iter(pfs.into_iter())
    }

    fn as_vec(&self) -> Vec<Self::Id> {
        (0..self.eval_domain.size()).zip(self.poly_evals.clone())
            // .filter(|(_x, y)| *y != Fr::zero()) //forgot why I added this
            .map(move |(x,y)| FFTDomainId::new(&self, x, y)).collect()
    }

    fn compute_eval_proof_with_setup(&self, setup: &crate::shared::digest::DigestKey, id: Self::Id, round: usize) -> G1Affine {
        unimplemented!()
    }

    fn compute_eval_proofs_with_setup(&self, setup: &crate::shared::digest::DigestKey, ids: &[Self::Id], round: usize) -> HashMap<Self::Id, G1Affine> {
        unimplemented!()
    }
}
