// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::sigma_protocol::{homomorphism, Statement, Witness};
use ark_ff::PrimeField;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
    Write,
};

#[derive(CanonicalSerialize, Debug, CanonicalDeserialize, Clone)]
pub struct Proof<F: PrimeField, H: homomorphism::Trait>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    /// The “first item” recorded in the proof, which can be either:
    /// - the prover's commitment (H::Codomain)
    /// - the verifier's challenge (E::ScalarField)
    pub first_proof_item: FirstProofItem<F, H>,
    /// Prover's second message (response)
    pub z: H::Domain,
}

impl<F: PrimeField, H: homomorphism::Trait> Proof<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    /// Reference to the prover's first message (commitment) when the proof stores a commitment.
    /// Returns `None` when the first proof item is the challenge (non-batchable proof). (Obviously
    /// it can be recomputed in this setting)
    pub fn prover_commitment(&self) -> Option<&H::CodomainNormalized> {
        match &self.first_proof_item {
            FirstProofItem::Commitment(a) => Some(a),
            FirstProofItem::Challenge(_) => None,
        }
    }

    /// No-op (semantically): circumvents the fact that proofs inherit the homomorphism's lifetime. This method should do nothing at runtime.
    #[allow(non_snake_case)]
    pub fn change_lifetime<H2>(self) -> Proof<F, H2>
    where
        H2: homomorphism::Trait<Domain = H::Domain, CodomainNormalized = H::CodomainNormalized>,
    {
        let first = match self.first_proof_item {
            FirstProofItem::Commitment(A) => FirstProofItem::Commitment(A),
            FirstProofItem::Challenge(c) => FirstProofItem::Challenge(c),
        };

        Proof {
            first_proof_item: first,
            z: self.z,
        }
    }
}

// Manual implementation of PartialEq and Eq is required here because deriving PartialEq/Eq would
// automatically require `H` itself to implement PartialEq and Eq, which is undesirable.
// Workaround would be to make `Proof` generic over `H::Domain` and `H::Codomain` instead of `H`
impl<F: PrimeField, H: homomorphism::Trait> PartialEq for Proof<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    fn eq(&self, other: &Self) -> bool {
        self.first_proof_item == other.first_proof_item && self.z == other.z
    }
}

// Empty because it simply asserts reflexivity
impl<F: PrimeField, H: homomorphism::Trait> Eq for Proof<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
}

/// The “first item” recorded in a Σ-proof, which is one of:
/// - The first message of the protocol, which is the commitment from the prover. This leads to a more compact proof.
/// - The second message of the protocol, which is the challenge from the verifier. This leads to a proof which is amenable to batch verification.
/// TODO: Better name? In https://github.com/sigma-rs/sigma-proofs these would be called "compact" and "batchable" proofs
#[derive(Clone, Debug, Eq)]
pub enum FirstProofItem<F: PrimeField, H: homomorphism::Trait>
where
    H::CodomainNormalized: Statement,
{
    Commitment(H::CodomainNormalized),
    Challenge(F), // In more generality, this should be H::Domain::Scalar
}

// Manual implementation of PartialEq is required here because deriving PartialEq would
// automatically require `H` itself to implement PartialEq, which is undesirable.
impl<F: PrimeField, H: homomorphism::Trait> PartialEq for FirstProofItem<F, H>
where
    H::CodomainNormalized: Statement,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FirstProofItem::Commitment(a), FirstProofItem::Commitment(b)) => a == b,
            (FirstProofItem::Challenge(a), FirstProofItem::Challenge(b)) => a == b,
            _ => false,
        }
    }
}

// The natural CanonicalSerialize/Deserialize implementations for `FirstProofItem`; we follow the usual approach for enums.
// CanonicalDeserialize needs Valid.
impl<F: PrimeField, H: homomorphism::Trait> Valid for FirstProofItem<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement + Valid,
{
    fn check(&self) -> Result<(), SerializationError> {
        match self {
            FirstProofItem::Commitment(c) => c.check(),
            FirstProofItem::Challenge(f) => f.check(),
        }
    }
}

impl<F: PrimeField, H: homomorphism::Trait> CanonicalSerialize for FirstProofItem<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement + CanonicalSerialize,
{
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        match self {
            FirstProofItem::Commitment(c) => {
                0u8.serialize_with_mode(writer.by_ref(), compress)?;
                c.serialize_with_mode(writer, compress)
            },
            FirstProofItem::Challenge(f) => {
                1u8.serialize_with_mode(writer.by_ref(), compress)?;
                f.serialize_with_mode(writer, compress)
            },
        }
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        1 + match self {
            FirstProofItem::Commitment(c) => c.serialized_size(compress),
            FirstProofItem::Challenge(f) => f.serialized_size(compress),
        }
    }
}

impl<F: PrimeField, H: homomorphism::Trait> CanonicalDeserialize for FirstProofItem<F, H>
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement + CanonicalDeserialize + Valid,
{
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        // Read the discriminant tag
        let tag = u8::deserialize_with_mode(&mut reader, compress, validate)?;

        let item = match tag {
            0 => {
                let c = H::CodomainNormalized::deserialize_with_mode(reader, compress, validate)?;
                FirstProofItem::Commitment(c)
            },
            1 => {
                let f = F::deserialize_with_mode(reader, compress, validate)?;
                FirstProofItem::Challenge(f)
            },
            _ => return Err(SerializationError::InvalidData),
        };

        // Run validity check if requested
        if validate == Validate::Yes {
            item.check()?;
        }

        Ok(item)
    }
}
