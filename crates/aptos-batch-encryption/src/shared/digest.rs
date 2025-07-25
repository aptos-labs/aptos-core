//! This module deals with computing and generating opening proofs for "digests",
//! which are KZG polynomial commitments which commit to a set of IDs.

use std::{collections::{HashMap, HashSet}, num};

use crate::{errors::BatchEncryptionError, group::{Fr, G1Affine, G1Projective, G2Affine, PairingSetting}};
use ark_ec::{pairing::Pairing, AffineRepr, ScalarMul, VariableBaseMSM};
use rand_core::{CryptoRng, RngCore};
use ark_std::UniformRand;
use num_traits::{Zero, One};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_ff::{Field as _, PrimeField as _};
use serde::{Deserialize, Serialize};
use super::ids::{Id, IdSet, OssifiedIdSet};
use anyhow::{Result, anyhow};
use crate::shared::ark_serialize::*;


use crate::{shared::algebra::fk_algorithm::FKDomain, shared::algebra::interpolate::interpolate};



/// The digest public parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestKey {
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub tau_g2: G2Affine,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub tau_powers_g1: Vec<Vec<G1Affine>>,
    pub fk_domain: FKDomain<Fr, G1Projective>,
}

/// A succinct commitment to a set of IDs. Internally, a KZG commitment over a shifted
/// powers-of-tau.
#[derive(Clone)]
pub struct Digest {
    digest_g1: G1Affine,
    round: usize,
}

impl Digest {
    pub fn as_g1(&self) -> G1Affine {
        self.digest_g1
    }

    pub(super) fn new_for_testing<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        Self {
            digest_g1: G1Affine::rand(rng),
            round: 0
        }
    }
}

impl DigestKey {
    pub fn new(rng: &mut impl RngCore, batch_size: usize, num_rounds: usize) -> Option<Self> {
        let tau = Fr::rand(rng);

        let mut tau_powers_fr = vec![Fr::one()];
        let mut cur = tau.clone();
        for _ in 0..batch_size {
            tau_powers_fr.push(cur);
            cur *= &tau;
        }

        let rs : Vec<Fr> = (0..num_rounds).map(|_| Fr::rand(rng)).collect();

        let tau_powers_randomized_fr = rs.into_iter().map(|r| 
            tau_powers_fr.iter().map(|tau_power| r * tau_power).collect::<Vec<Fr>>()).collect::<Vec<Vec<Fr>>>();

        let tau_powers_g1 : Vec<Vec<G1Affine>> = tau_powers_randomized_fr.into_iter().map(|powers_for_r|
            G1Projective::from(G1Affine::generator()).batch_mul(
                &powers_for_r
                )).collect();


        let tau_powers_g1_projective : Vec<Vec<G1Projective>> = tau_powers_g1.iter()
            .map(|gs| 
                gs.iter().map(|g|
                    G1Projective::from(*g)
                ).collect()
            ).collect();


        let tau_g2 : G2Affine = (G2Affine::generator() * tau).into();

        let fk_domain = FKDomain::new(batch_size, batch_size, tau_powers_g1_projective)?;


        Some(DigestKey {
            tau_g2,
            tau_powers_g1,
            fk_domain,
        })
    }

    pub fn capacity(&self) -> usize {
        self.tau_powers_g1[0].len() - 1
    }

    
    pub fn digest<IS: IdSet>(&self, ids: &mut IS, round: usize) -> Result<(Digest, EvalProofs<IS::OssifiedSet>)> {
        if round >= self.tau_powers_g1.len() {
            Err(anyhow!("Tried to compute digest with round greater than setup length."))
        } else if ids.capacity() > self.tau_powers_g1[round].len() - 1 {
            Err(anyhow!("Tried to compute a batch digest with size {}, where setup supports up to size {}", 
                ids.capacity(), 
                self.tau_powers_g1[round].len() - 1
            ))?
        } else {

            let ids = ids.compute_poly_coeffs();
            let mut coeffs = ids.poly_coeffs();
            coeffs.resize(self.tau_powers_g1[round].len(), Fr::zero());

            let digest = Digest { digest_g1: G1Projective::msm(&self.tau_powers_g1[round], &coeffs).unwrap().into(), round };

            Ok((digest.clone(),
            EvalProofs::new(&self, digest, ids)
        ))
        }
    }

}


#[derive(Clone)]
pub struct EvalProofs<'a, IS: OssifiedIdSet> {
    pub digest_key: &'a DigestKey,
    pub digest: Digest,
    pub ids: IS,
    pub computed_proofs: HashMap<IS::Id, G1Affine>,
}

impl<'a, IS: OssifiedIdSet> EvalProofs<'a, IS> {
    pub fn new(digest_key: &'a DigestKey, digest: Digest, ids: IS) -> Self {
        Self {
            digest_key,
            digest,
            ids,
            computed_proofs: HashMap::new(),
        }
    }

    pub fn get(&self, i: &IS::Id) -> Option<G1Affine> {
        self.computed_proofs.get(i)
            .map(|g1| *g1)
    }

    pub fn compute_all(&mut self) {
        self.computed_proofs = self.ids.compute_all_eval_proofs_with_setup(self.digest_key, self.digest.round);
    }

    pub fn compute(&mut self, ids: &[IS::Id]) {
        self.computed_proofs = self.ids.compute_eval_proofs_with_setup(self.digest_key, ids, self.digest.round);
    }

    pub fn compute_single(&mut self, id : IS::Id) {
        let pf = self.ids.compute_eval_proof_with_setup(self.digest_key, id, self.digest.round);
        self.computed_proofs.insert(id, pf);
    }

    fn verify_pf(&self, id: IS::Id, pf: G1Affine) -> Result<()> {
        // TODO use multipairing here?
        Ok((PairingSetting::pairing(pf, self.digest_key.tau_g2 - G2Affine::generator() * id.x()) 
            == 
            PairingSetting::pairing(self.digest.as_g1() - G1Affine::generator() * id.y(), G2Affine::generator())).then_some(()).ok_or(BatchEncryptionError::EvalProofVerifyError)?)


    }

    pub fn uncomputed_ids(&self) -> Vec<IS::Id> {
        let ids : HashSet<<IS as OssifiedIdSet>::Id> = HashSet::from_iter(self.ids.as_vec().into_iter());
        let computed_ids : HashSet<<IS as OssifiedIdSet>::Id> = HashSet::from_iter(self.computed_proofs.keys().cloned());

        ids.difference(&computed_ids).cloned().collect()
    }

    pub fn verify(&self, id: IS::Id) -> Result<()> {
        let pf = self.computed_proofs[&id];
        self.verify_pf(id, pf)
    }

    pub fn verify_all(&self) -> Result<()> {
        self.computed_proofs
            .iter()
            .map(|(id, pf)| self.verify_pf(*id, *pf))
            .collect()
    }

    pub fn merge(&mut self, other_pfs: &Self) {
    }
}




#[cfg(test)]
pub(crate) mod tests {
    use ark_poly::DenseUVPolynomial;
    use ark_poly::{univariate::DensePolynomial, Polynomial};
    use ark_std::rand::thread_rng;
    use itertools::Itertools;
    use super::*;
    use crate::shared::ids::free_roots::ComputedCoeffs;
    use crate::shared::ids::{FFTDomainIdSet, FreeRootId, FreeRootIdSet};


    pub(crate) fn digest_and_pfs_for_testing<'a>(dk: &'a DigestKey) 
        -> (Digest, EvalProofs<'a, FreeRootIdSet<ComputedCoeffs>>)
    {
        let mut ids = FreeRootIdSet::with_capacity(dk.capacity()).unwrap();
        let mut counter = Fr::zero();

        for _ in 0..dk.capacity() {
            ids.add(&FreeRootId::new(counter));
            counter += Fr::one();
        }

        ids.compute_poly_coeffs();
        dk.digest(&mut ids, 0).unwrap()
    }


    #[test]
    fn compute_and_verify_all_opening_proofs() {

        let batch_capacity = 8;
        let num_rounds = 4;
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, batch_capacity, num_rounds * batch_capacity).unwrap();

        for current_batch_size in 1..=batch_capacity {

            let mut ids = FreeRootIdSet::with_capacity(batch_capacity).unwrap();
            let mut counter = Fr::zero();

            for _ in 0..current_batch_size {
                ids.add(&FreeRootId::new(counter));
                counter += Fr::one();
            }

            ids.compute_poly_coeffs();

            for round in 0..num_rounds {
                let (d, mut pfs) = setup.digest(&mut ids, round).unwrap();
                pfs.compute_all();
                pfs.verify_all().unwrap();
            }
        }
    }


    #[test]
    fn compute_and_verify_individual_opening_proofs() {
        let batch_size = 4;
        let num_rounds = 4;
        let mut rng = thread_rng();
        let setup = DigestKey::new(&mut rng, batch_size, num_rounds).unwrap();
        let mut ids = FreeRootIdSet::with_capacity(batch_size).unwrap();
        let mut counter = Fr::zero();
        for _x in 0..batch_size {
            ids.add(&FreeRootId::new(counter));
            counter += Fr::one();
        }

        for round in 0..num_rounds {
            println!("{}", round);
            let (d, mut pfs) = setup.digest(&mut ids, round).unwrap();
            for id in ids.as_vec() {
                pfs.compute_single(id);
            }
            pfs.verify_all().unwrap();
        }
    }

    #[test]
    fn test_digest_key_capacity() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 1).unwrap();
        assert_eq!(dk.capacity(), 8);
    }



}
