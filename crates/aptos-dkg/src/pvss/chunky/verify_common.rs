// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! SoK context and shared verification helpers for weighted chunky PVSS (v1 and v2).
//!
//! The SoK context is bound to the Fiat–Shamir transcript so that proofs are tied to
//! the dealer's signing key, session, and domain-separation tag.

use super::{subtranscript::Subtranscript, EncryptPubKey};
use crate::pvss::Player;
use anyhow::bail;
use aptos_crypto::{bls12381, weighted_config::WeightedConfigArkworks, TSecretSharingConfig as _};
use ark_ec::pairing::Pairing;
use serde::Serialize;

/// Context hashed into the SoK Fiat–Shamir transcript (dealer key, session, DST).
#[derive(Serialize, Clone, Debug)]
pub struct SokContext<'a, A: Serialize + Clone> {
    pub signing_pubkey: bls12381::PublicKey,
    pub session_id: &'a A,
    pub dealer_id: usize,
    pub dst: Vec<u8>,
}

impl<'a, A: Serialize + Clone> SokContext<'a, A> {
    /// Builds a SoK context for the Fiat–Shamir transcript.
    ///
    /// This context is hashed into the transcript so that proofs are bound to the dealer's
    /// signing key, the session, and the domain-separation tag. It is used when verifying
    /// weighted chunky PVSS transcripts (v1 and v2).
    ///
    /// # Arguments
    /// * `signing_pubkey` - The dealer's BLS12-381 public key used for signing.
    /// * `session_id` - Session identifier; serialized and bound into the transcript.
    /// * `dealer_id` - Index of the dealer in the weighted config.
    /// * `dst` - Domain-separation tag (DST) for the proof system.
    pub fn new(
        signing_pubkey: bls12381::PublicKey,
        session_id: &'a A,
        dealer_id: usize,
        dst: Vec<u8>,
    ) -> Self {
        Self {
            signing_pubkey,
            session_id,
            dealer_id,
            dst,
        }
    }
}

/// Checks that `eks`, `subtrs.Cs`, and `subtrs.Vs` lengths match the weighted config,
/// then builds and returns the SoK context for the dealer.
/// Call this at the start of `verify` for both weighted transcript v1 and v2.
pub fn verify_weighted_preamble<'a, A: Serialize + Clone, E: Pairing>(
    sc: &WeightedConfigArkworks<E::ScalarField>,
    subtrs: &Subtranscript<E>,
    dealer: &Player,
    spks: &[bls12381::PublicKey],
    eks: &[EncryptPubKey<E>],
    sid: &'a A,
    dst: Vec<u8>,
) -> anyhow::Result<SokContext<'a, A>> {
    if eks.len() != sc.get_total_num_players() {
        bail!(
            "Expected {} encryption keys, but got {}",
            sc.get_total_num_players(),
            eks.len()
        );
    }
    if subtrs.Cs.len() != sc.get_total_num_players() {
        bail!(
            "Expected {} arrays of chunked ciphertexts, but got {}",
            sc.get_total_num_players(),
            subtrs.Cs.len()
        );
    }
    if subtrs.Vs.len() != sc.get_total_num_players() {
        bail!(
            "Expected {} arrays of commitment elements, but got {}",
            sc.get_total_num_players(),
            subtrs.Vs.len()
        );
    }
    Ok(SokContext::new(
        spks[dealer.id].clone(),
        sid,
        dealer.id,
        dst,
    ))
}
