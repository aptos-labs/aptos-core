// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{hash_to_scalar, random::random_scalar, HasMultiExp};
use anyhow::bail;
use velor_crypto::signing_message;
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::Scalar;
use ff::Field;
use group::Group;
use serde::{Deserialize, Serialize};
use std::ops::{Mul, Neg};

const SCHNORR_POK_DST: &[u8; 21] = b"VELOR_SCHNORR_POK_DST";

/// A Schnorr PoK for (g, g^a) is a tuple:
///
///   $$(R = g^r, s = r + H(g^r, g^a, g) a)$$
pub type PoK<Gr> = (Gr, Scalar);

/// This is the Schnorr prover transcript that is hashed to obtain a Fiat-Shamir challenge.
/// TODO(TechDebt): Cannot have references here because CryptoHasher doesn't work with lifetimes.
#[derive(Serialize, Deserialize, BCSCryptoHash, CryptoHasher)]
#[allow(non_snake_case)]
struct Challenge<Gr> {
    R: Gr,  // g^r
    pk: Gr, // g^a
    g: Gr,
}

#[allow(non_snake_case)]
pub fn pok_prove<Gr, R>(a: &Scalar, g: &Gr, pk: &Gr, rng: &mut R) -> PoK<Gr>
where
    Gr: Serialize + Group + for<'a> Mul<&'a Scalar, Output = Gr>,
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    debug_assert!(g.mul(a).eq(pk));

    let r = random_scalar(rng);
    let R = g.mul(&r);
    let e = schnorr_hash(Challenge::<Gr> { R, pk: *pk, g: *g });
    let s = r + e * a;

    (R, s)
}

/// Computes the Fiat-Shamir challenge in the Schnorr PoK protocol given an instance $(g, pk = g^a)$
/// and the commitment $R = g^r$.
#[allow(non_snake_case)]
fn schnorr_hash<Gr>(c: Challenge<Gr>) -> Scalar
where
    Gr: Serialize,
{
    let c = signing_message(&c)
        .expect("unexpected error during Schnorr challenge struct serialization");

    hash_to_scalar(&c, SCHNORR_POK_DST)
}

/// Verifies all the $n$ Schnorr PoKs by taking a random linear combination of the verification
/// equations using $(1, \alpha, \alpha^2, \ldots, \alpha^{n-1})$ as the randomness.
///
/// The equation is:
///
///    $$g^{\sum_i s_i \gamma_i} = \prod_i R_i^{\gamma_i} \pk_i^{e_i \gamma_i}$$
///
/// where $e_i$ is the Fiat-Shamir challenge derived by hashing the PK and the generator $g$.
#[allow(non_snake_case)]
pub fn pok_batch_verify<'a, Gr>(
    poks: &Vec<(Gr, PoK<Gr>)>,
    g: &Gr,
    gamma: &Scalar,
) -> anyhow::Result<()>
where
    Gr: Serialize + Group + Mul<&'a Scalar> + HasMultiExp,
{
    let n = poks.len();
    let mut exps = Vec::with_capacity(2 * n + 1);
    let mut bases = Vec::with_capacity(2 * n + 1);

    // Compute \gamma_i = \gamma^i, for all i \in [0, n]
    let mut gammas = Vec::with_capacity(n);
    gammas.push(Scalar::ONE);
    for _ in 0..(n - 1) {
        gammas.push(gammas.last().unwrap().mul(gamma));
    }

    let mut last_exp = Scalar::ZERO;
    for i in 0..n {
        let (pk, (R, s)) = poks[i];

        bases.push(R);
        exps.push(gammas[i]);

        bases.push(pk);
        exps.push(schnorr_hash(Challenge::<Gr> { R, pk, g: *g }) * gammas[i]);

        last_exp += s * gammas[i];
    }

    bases.push(*g);
    exps.push(last_exp.neg());

    if Gr::multi_exp_iter(bases.iter(), exps.iter()) != Gr::identity() {
        bail!("Schnorr PoK batch verification failed");
    }

    Ok(())
}
