// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Signature-of-knowledge (SoK) context for the PVSS transcript.
//! This context is bound to the Fiat–Shamir transcript so that proofs are tied to
//! the dealer’s signing key, session, and domain-separation tag.

use aptos_crypto::bls12381;
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
