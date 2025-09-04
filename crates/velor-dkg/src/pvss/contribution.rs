// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pvss::{schnorr, Player},
    utils::HasMultiExp,
};
use anyhow::bail;
use velor_crypto::bls12381;
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use blstrs::Scalar;
use group::Group;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, ops::Mul};

/// A PVSS contribution, which is signed as part of the PVSS transcript
/// TODO(TechDebt): CryptoHasher does not work with lifetimes. So we cannot make this struct store
///  references, which causes unnecessary cloning in the code.
#[derive(Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct Contribution<Gr, A> {
    pub comm: Gr,
    pub player: Player,
    pub aux: A,
}

pub type SoK<Gr> = (Player, Gr, bls12381::Signature, schnorr::PoK<Gr>);

pub fn batch_verify_soks<Gr, A>(
    soks: &[SoK<Gr>],
    pk_base: &Gr,
    pk: &Gr,
    spks: &Vec<bls12381::PublicKey>,
    aux: &Vec<A>,
    tau: &Scalar,
) -> anyhow::Result<()>
where
    Gr: Serialize + HasMultiExp + Display + Copy + Group + for<'a> Mul<&'a Scalar>,
    A: Serialize + Clone,
{
    if soks.len() != spks.len() {
        bail!(
            "Expected {} signing PKs, but got {}",
            soks.len(),
            spks.len()
        );
    }

    if soks.len() != aux.len() {
        bail!(
            "Expected {} auxiliary infos, but got {}",
            soks.len(),
            aux.len()
        );
    }

    // First, the PoKs
    let mut c = Gr::identity();
    for (_, c_i, _, _) in soks {
        c.add_assign(c_i)
    }

    if c.ne(pk) {
        bail!(
            "The PoK does not correspond to the dealt secret. Expected {} but got {}",
            pk,
            c
        );
    }

    let poks = soks
        .iter()
        .map(|(_, c, _, pok)| (*c, *pok))
        .collect::<Vec<(Gr, schnorr::PoK<Gr>)>>();

    // TODO(Performance): 128-bit exponents instead of powers of tau
    schnorr::pok_batch_verify::<Gr>(&poks, pk_base, &tau)?;

    // Second, the signatures
    let msgs = soks
        .iter()
        .zip(aux)
        .map(|((player, comm, _, _), aux)| Contribution::<Gr, A> {
            comm: *comm,
            player: *player,
            aux: aux.clone(),
        })
        .collect::<Vec<Contribution<Gr, A>>>();
    let msgs_refs = msgs
        .iter()
        .map(|c| c)
        .collect::<Vec<&Contribution<Gr, A>>>();
    let pks = spks
        .iter()
        .map(|pk| pk)
        .collect::<Vec<&bls12381::PublicKey>>();
    let sig = bls12381::Signature::aggregate(
        soks.iter()
            .map(|(_, _, sig, _)| sig.clone())
            .collect::<Vec<bls12381::Signature>>(),
    )?;

    sig.verify_aggregate(&msgs_refs[..], &pks[..])?;
    Ok(())
}
