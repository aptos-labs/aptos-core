// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    keyless::{
        bn254_circom::{
            G1Bytes, G2Bytes, G1_PROJECTIVE_COMPRESSED_NUM_BYTES,
            G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
        },
        zkp_sig::ZKP,
    },
    transaction::authenticator::EphemeralSignature,
};
use anyhow::bail;
use velor_crypto::CryptoMaterialError;
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use ark_bn254::{Bn254, Fr};
use ark_ff::{BigInteger, PrimeField};
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// NOTE: We do not deserialize these into affine points because we want to avoid doing unnecessary
/// work, since other validation steps might fail before we even get to the point of deserialization.
#[derive(
    Copy, Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize, CryptoHasher, BCSCryptoHash,
)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct Groth16Proof {
    a: G1Bytes,
    b: G2Bytes,
    c: G1Bytes,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct ZeroKnowledgeSig {
    pub proof: ZKP,
    /// The expiration horizon that the circuit should enforce on the expiration date committed in
    /// the nonce. This must be <= `Configuration::max_expiration_horizon_secs`.
    pub exp_horizon_secs: u64,
    /// An optional extra field (e.g., `"<name>":"<val>"`) that will be matched publicly in the JWT
    pub extra_field: Option<String>,
    /// Will be set to the override `aud` value that the circuit should match, instead of the `aud`
    /// in the IDC. This will allow users to recover keyless accounts bound to an application that
    /// is no longer online.
    pub override_aud_val: Option<String>,
    /// A signature on the proof and the statement (via the training wheels SK) to mitigate against
    /// flaws in our circuit.
    pub training_wheels_signature: Option<EphemeralSignature>,
}

/// This struct is used to wrap together the Groth16 ZKP and the statement it proves so that the
/// prover service can sign them together. It is only used during signature verification & never
/// sent over the network.
#[derive(Clone, Debug, CryptoHasher, BCSCryptoHash, Hash, PartialEq, Eq)]
pub struct Groth16ProofAndStatement {
    pub proof: Groth16Proof,
    // TODO(keyless): implement Serialize/Deserialize for Fr and use Fr here directly
    pub public_inputs_hash: [u8; 32],
}

impl Groth16ProofAndStatement {
    pub fn new(proof: Groth16Proof, public_inputs_hash: Fr) -> Self {
        let public_inputs_hash: [u8; 32] = public_inputs_hash
            .into_bigint()
            .to_bytes_le()
            .try_into()
            .expect("expected 32-byte public inputs hash");

        Groth16ProofAndStatement {
            proof,
            public_inputs_hash,
        }
    }
}

impl<'de> Deserialize<'de> for Groth16ProofAndStatement {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            //
            // In this case, we use the serde(with = "hex") macro, which causes public_inputs_hash
            // to deserialize from a hex string.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "Groth16ProofAndStatement")]
            struct Value {
                pub proof: Groth16Proof,
                #[serde(with = "hex")]
                pub public_inputs_hash: [u8; 32],
            }

            let value = Value::deserialize(deserializer)?;
            Ok(Groth16ProofAndStatement {
                proof: value.proof,
                public_inputs_hash: value.public_inputs_hash,
            })
        } else {
            // Same as above, except this time we don't use the serde(with = "hex") macro, so that
            // serde uses default behavior for serialization.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "Groth16ProofAndStatement")]
            struct Value {
                pub proof: Groth16Proof,
                pub public_inputs_hash: [u8; 32],
            }

            let value = Value::deserialize(deserializer)?;
            Ok(Groth16ProofAndStatement {
                proof: value.proof,
                public_inputs_hash: value.public_inputs_hash,
            })
        }
    }
}

impl Serialize for Groth16ProofAndStatement {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            #[derive(::serde::Serialize)]
            #[serde(rename = "Groth16ProofAndStatement")]
            struct Value {
                pub proof: Groth16Proof,
                #[serde(with = "hex")]
                pub public_inputs_hash: [u8; 32],
            }

            let value = Value {
                proof: self.proof,
                public_inputs_hash: self.public_inputs_hash,
            };

            value.serialize(serializer)
        } else {
            #[derive(::serde::Serialize)]
            #[serde(rename = "Groth16ProofAndStatement")]
            struct Value {
                pub proof: Groth16Proof,
                pub public_inputs_hash: [u8; 32],
            }

            let value = Value {
                proof: self.proof,
                public_inputs_hash: self.public_inputs_hash,
            };

            value.serialize(serializer)
        }
    }
}

impl ZeroKnowledgeSig {
    pub fn verify_groth16_proof(
        &self,
        public_inputs_hash: Fr,
        pvk: &PreparedVerifyingKey<Bn254>,
    ) -> anyhow::Result<()> {
        match self.proof {
            ZKP::Groth16(proof) => proof.verify_proof(public_inputs_hash, pvk),
        }
    }
}

impl TryFrom<&[u8]> for ZeroKnowledgeSig {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<ZeroKnowledgeSig>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl TryFrom<&[u8]> for Groth16Proof {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<Groth16Proof>(bytes)
            .map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

impl Groth16Proof {
    pub fn new(a: G1Bytes, b: G2Bytes, c: G1Bytes) -> Self {
        Groth16Proof { a, b, c }
    }

    pub fn get_a(&self) -> &G1Bytes {
        &self.a
    }

    pub fn get_b(&self) -> &G2Bytes {
        &self.b
    }

    pub fn get_c(&self) -> &G1Bytes {
        &self.c
    }

    /// NOTE: For testing only. (And used in `testsuite/generate-format`.)
    pub fn dummy_proof() -> Self {
        Groth16Proof {
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
        // let start = std::time::Instant::now();
        let proof: Proof<Bn254> = Proof {
            a: self.a.deserialize_into_affine()?,
            b: self.b.deserialize_into_affine()?,
            c: self.c.deserialize_into_affine()?,
        };
        // println!("Deserialization time: {:?}", start.elapsed());

        // let start = std::time::Instant::now();
        let verified = Groth16::<Bn254>::verify_proof(pvk, &proof, &[public_inputs_hash])?;
        // println!("Proof verification time: {:?}", start.elapsed());
        if !verified {
            bail!("groth16 proof verification failed")
        }
        Ok(())
    }
}
