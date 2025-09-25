// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Protocol (blueprint), generic over a homomorphism F.
pub struct Protocol<F: Homomorphism> {
    pub hom: F,
}

pub trait Homomorphism {
    type Domain;
    type Codomain;
}

/// The “first message” **stored** in a Sigma proof, which is one of:
/// - Commitment from the prover
/// - Challenge from the verifier
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum FirstStoredMessage<H: Homomorphism>
{
    Commitment(H::Codomain),
    Challenge(<H::Domain as Domain>::Scalar),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(bound(
    serialize = "
        H::Domain: Serialize,
        H::Codomain: Serialize,
        <H::Domain as Domain>::Scalar: Serialize
    ",
    deserialize = "
        H::Domain: Deserialize<'de>,
        H::Codomain: Deserialize<'de>,
        <H::Domain as Domain>::Scalar: Deserialize<'de>
    "
))]
// By default, #[derive(Serialize, Deserialize)] would force `H` itself to implement
// Serialize/Deserialize. That fails because `H` (e.g. ConsistencyHomomorphism)
// may contain references (like &PublicParameters) that only implement DeserializeOwned.
// 
// The #[serde(bound(...))] attribute overrides those defaults and tells serde
// to require serialization only of the *associated types* actually used here
// (Domain, Codomain, Scalar). This way, `SigmaProof` can be (de)serialized
// without `H` itself needing to be.
pub struct Proof<H: Homomorphism> {
    /// The “first message” stored in the proof: either the prover's commitment (H::Codomain)
    /// or the verifier's challenge (H::Domain::Scalar)
    pub first_stored_message: FirstStoredMessage<H>,
    /// Prover's second message (response)
    pub z: H::Domain,
}