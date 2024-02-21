// Copyright © Aptos Foundation

#[cfg(test)]
use crate::oidb::bn254_circom::{
    G1_PROJECTIVE_COMPRESSED_NUM_BYTES, G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
};
use crate::{
    oidb::bn254_circom::{G1Bytes, G2Bytes},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
};
use anyhow::bail;
use aptos_crypto::CryptoMaterialError;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_bn254::{Bn254, Fr};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
use serde::{Deserialize, Serialize};

#[derive(
    Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize, CryptoHasher, BCSCryptoHash,
)]
pub struct Groth16Zkp {
    a: G1Bytes,
    b: G2Bytes,
    c: G1Bytes,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct SignedGroth16Zkp {
    pub proof: Groth16Zkp,
    /// A signature on the proof (via the ephemeral SK) to prevent malleability attacks.
    pub non_malleability_signature: EphemeralSignature,
    /// The expiration horizon that the circuit should enforce on the expiration date committed in the nonce.
    /// This must be <= `Configuration::max_expiration_horizon_secs`.
    pub exp_horizon_secs: u64,
    /// An optional extra field (e.g., `"<name>":"<val>") that will be matched publicly in the JWT
    pub extra_field: Option<String>,
    /// Will be set to the override `aud` value that the circuit should match, instead of the `aud` in the IDC.
    /// This will allow users to recover their OIDB accounts derived by an application that is no longer online.
    pub override_aud_val: Option<String>,
    /// A signature on the proof (via the training wheels SK) to mitigate against flaws in our circuit
    pub training_wheels_signature: Option<EphemeralSignature>,
}

impl SignedGroth16Zkp {
    pub fn verify_non_malleability_sig(&self, pub_key: &EphemeralPublicKey) -> anyhow::Result<()> {
        self.non_malleability_signature.verify(&self.proof, pub_key)
    }

    pub fn verify_training_wheels_sig(&self, pub_key: &EphemeralPublicKey) -> anyhow::Result<()> {
        if let Some(training_wheels_signature) = &self.training_wheels_signature {
            training_wheels_signature.verify(&self.proof, pub_key)
        } else {
            bail!("No training_wheels_signature found")
        }
    }

    pub fn verify_proof(
        &self,
        public_inputs_hash: Fr,
        pvk: &PreparedVerifyingKey<Bn254>,
    ) -> anyhow::Result<()> {
        self.proof.verify_proof(public_inputs_hash, pvk)
    }
}

impl TryFrom<&[u8]> for Groth16Zkp {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<Groth16Zkp>(bytes).map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl Groth16Zkp {
    pub fn new(a: G1Bytes, b: G2Bytes, c: G1Bytes) -> Self {
        Groth16Zkp { a, b, c }
    }

    #[cfg(test)]
    pub fn dummy_proof() -> Self {
        Groth16Zkp {
            a: G1Bytes::new_from_vec(vec![0u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
            b: G2Bytes::new_from_vec(vec![1u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
            c: G1Bytes::new_from_vec(vec![2u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]).unwrap(),
        }
    }

    pub fn verify_proof(
        &self,
        public_inputs_hash: Fr,
        pvk: &PreparedVerifyingKey<Bn254>,
    ) -> anyhow::Result<()> {
        let proof: Proof<Bn254> = Proof {
            a: self.a.deserialize_into_affine()?,
            b: self.b.as_affine()?,
            c: self.c.deserialize_into_affine()?,
        };
        let result = Groth16::<Bn254>::verify_proof(pvk, &proof, &[public_inputs_hash])?;
        if !result {
            bail!("groth16 proof verification failed")
        }
        Ok(())
    }
}
