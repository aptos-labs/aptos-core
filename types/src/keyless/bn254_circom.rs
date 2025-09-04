// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::circuit_constants::MAX_EXTRA_FIELD_BYTES;
use crate::{
    jwks::rsa::RSA_JWK,
    keyless::{
        base64url_encode_str, Configuration, EphemeralCertificate, IdCommitment, KeylessPublicKey,
        KeylessSignature,
    },
    serialize,
    transaction::authenticator::EphemeralPublicKey,
};
use anyhow::bail;
use velor_crypto::{poseidon_bn254, poseidon_bn254::pad_and_hash_string, CryptoMaterialError};
use ark_bn254::{Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use num_traits::{One, Zero};
use once_cell::sync::Lazy;
use quick_cache::sync::Cache;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_big_array::BigArray;

// TODO(keyless): Some of this stuff, if not all, belongs to the velor-crypto crate

pub const G1_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 32;
pub const G2_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 64;

// When the extra_field is none, use this hash value which is equal to the hash of a single space string.
static EMPTY_EXTRA_FIELD_HASH: Lazy<Fr> = Lazy::new(|| {
    poseidon_bn254::keyless::pad_and_hash_string(" ", MAX_EXTRA_FIELD_BYTES as usize).unwrap()
});

static EMPTY_OVERRIDE_AUD_FIELD_HASH: Lazy<Fr> = Lazy::new(|| {
    poseidon_bn254::keyless::pad_and_hash_string("", IdCommitment::MAX_AUD_VAL_BYTES).unwrap()
});

/// This will do the proper subgroup membership checks.
pub fn g1_projective_str_to_affine(x: &str, y: &str) -> anyhow::Result<G1Affine> {
    let g1_affine = G1Bytes::new_unchecked(x, y)?.deserialize_into_affine()?;
    Ok(g1_affine)
}

/// This will do the proper subgroup membership checks.
pub fn g2_projective_str_to_affine(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<G2Affine> {
    let g2_affine = G2Bytes::new_unchecked(x, y)?.deserialize_into_affine()?;
    Ok(g2_affine)
}

/// Converts a decimal string to an Fq
fn parse_fq_element(s: &str) -> Result<Fq, CryptoMaterialError> {
    s.parse::<Fq>()
        .map_err(|_e| CryptoMaterialError::DeserializationError)
}

#[allow(unused)]
/// Converts a decimal string to an Fr
pub fn parse_fr_element(s: &str) -> Result<Fr, CryptoMaterialError> {
    s.parse::<Fr>()
        .map_err(|_e| CryptoMaterialError::DeserializationError)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct G1Bytes(pub(crate) [u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G1Bytes {
    pub fn new_unchecked(x: &str, y: &str) -> anyhow::Result<Self> {
        let g1 = G1Projective::new_unchecked(
            parse_fq_element(x)?,
            parse_fq_element(y)?,
            parse_fq_element("1")?,
        );

        let bytes: Vec<u8> = serialize!(g1);
        Self::new_from_vec(bytes)
    }

    /// Used internally or for testing.
    pub fn new_from_vec(vec: Vec<u8>) -> anyhow::Result<Self> {
        if vec.len() == G1_PROJECTIVE_COMPRESSED_NUM_BYTES {
            let mut bytes = [0; G1_PROJECTIVE_COMPRESSED_NUM_BYTES];
            bytes.copy_from_slice(&vec);
            Ok(Self(bytes))
        } else {
            bail!(
                "Serialized BN254 G1 must have exactly {} bytes",
                G1_PROJECTIVE_COMPRESSED_NUM_BYTES
            )
        }
    }

    pub fn deserialize_into_affine(&self) -> Result<G1Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl<'de> Deserialize<'de> for G1Bytes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String>::deserialize(deserializer)?;
            let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
            G1Bytes::new_from_vec(bytes).map_err(serde::de::Error::custom)
        } else {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "G1Bytes")]
            struct Value([u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]);

            let value = Value::deserialize(deserializer)?;
            Ok(G1Bytes(value.0))
        }
    }
}

impl Serialize for G1Bytes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            hex::encode(self.0).serialize(serializer)
        } else {
            // See comment in deserialize.
            serializer.serialize_newtype_struct("G1Bytes", &self.0)
        }
    }
}

impl TryInto<G1Projective> for &G1Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G1Projective, CryptoMaterialError> {
        G1Projective::deserialize_compressed(self.0.as_slice())
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl TryInto<G1Affine> for &G1Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G1Affine, CryptoMaterialError> {
        let g1_projective: G1Projective = self.try_into()?;
        Ok(g1_projective.into())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct G2Bytes(pub(crate) [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G2Bytes {
    pub fn new_unchecked(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<Self> {
        let g2 = G2Projective::new_unchecked(
            Fq2::new(parse_fq_element(x[0])?, parse_fq_element(x[1])?),
            Fq2::new(parse_fq_element(y[0])?, parse_fq_element(y[1])?),
            Fq2::new(parse_fq_element("1")?, parse_fq_element("0")?),
        );

        let bytes: Vec<u8> = serialize!(g2);
        Self::new_from_vec(bytes)
    }

    pub fn new_from_vec(vec: Vec<u8>) -> anyhow::Result<Self> {
        if vec.len() == G2_PROJECTIVE_COMPRESSED_NUM_BYTES {
            let mut bytes = [0; G2_PROJECTIVE_COMPRESSED_NUM_BYTES];
            bytes.copy_from_slice(&vec);
            Ok(Self(bytes))
        } else {
            bail!(
                "Serialized BN254 G2 must have exactly {} bytes",
                G2_PROJECTIVE_COMPRESSED_NUM_BYTES
            )
        }
    }

    pub fn deserialize_into_affine(&self) -> Result<G2Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl<'de> Deserialize<'de> for G2Bytes {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = <String>::deserialize(deserializer)?;
            let bytes = hex::decode(s).map_err(serde::de::Error::custom)?;
            G2Bytes::new_from_vec(bytes).map_err(serde::de::Error::custom)
        } else {
            // In order to preserve the Serde data model and help analysis tools,
            // make sure to wrap our value in a container with the same name
            // as the original type.
            #[derive(::serde::Deserialize)]
            #[serde(rename = "G2Bytes")]
            struct Value(#[serde(with = "BigArray")] [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

            let value = Value::deserialize(deserializer)?;
            Ok(G2Bytes(value.0))
        }
    }
}

impl Serialize for G2Bytes {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            hex::encode(self.0).serialize(serializer)
        } else {
            // Doing this differently than G1Bytes in order to use serde(with = "BigArray"). This
            // apparently is needed to correctly deserialize arrays with size greater than 32.
            #[derive(::serde::Serialize)]
            #[serde(rename = "G2Bytes")]
            struct Value(#[serde(with = "BigArray")] [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

            let value = Value(self.0);

            // See comment in deserialize.
            value.serialize(serializer)
        }
    }
}

impl TryInto<G2Projective> for &G2Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G2Projective, CryptoMaterialError> {
        G2Projective::deserialize_compressed(self.0.as_slice())
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl TryInto<G2Affine> for &G2Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G2Affine, CryptoMaterialError> {
        let g2_projective: G2Projective = self.try_into()?;
        Ok(g2_projective.into())
    }
}

static PAD_AND_HASH_STRING_CACHE: Lazy<Cache<(String, usize), Fr>> =
    Lazy::new(|| Cache::new(1_000));

static JWK_HASH_CACHE: Lazy<Cache<RSA_JWK, Fr>> = Lazy::new(|| Cache::new(100));

pub fn cached_pad_and_hash_string(str: &str, max_bytes: usize) -> anyhow::Result<Fr> {
    let key = (str.to_string(), max_bytes);
    match PAD_AND_HASH_STRING_CACHE.get(&key) {
        None => {
            let hash = pad_and_hash_string(str, max_bytes)?;
            PAD_AND_HASH_STRING_CACHE.insert(key, hash);
            Ok(hash)
        },
        Some(hash) => Ok(hash),
    }
}

pub fn cached_jwk_hash(jwk: &RSA_JWK) -> anyhow::Result<Fr> {
    match JWK_HASH_CACHE.get(jwk) {
        None => {
            let hash = jwk.to_poseidon_scalar()?;
            JWK_HASH_CACHE.insert(jwk.clone(), hash);
            Ok(hash)
        },
        Some(hash) => Ok(hash),
    }
}

pub fn hash_public_inputs(
    config: &Configuration,
    epk: &EphemeralPublicKey,
    idc: &IdCommitment,
    exp_timestamp_secs: u64,
    exp_horizon_secs: u64,
    iss: &str,
    extra_field: Option<&str>,
    jwt_header_json: &str,
    jwk: &RSA_JWK,
    override_aud_val: Option<&str>,
) -> anyhow::Result<Fr> {
    let (has_extra_field, extra_field_hash) = match extra_field {
        None => (Fr::zero(), *EMPTY_EXTRA_FIELD_HASH),
        Some(extra_field) => (
            Fr::one(),
            poseidon_bn254::keyless::pad_and_hash_string(
                extra_field,
                config.max_extra_field_bytes as usize,
            )?,
        ),
    };

    let (override_aud_val_hash, use_override_aud) = match override_aud_val {
        Some(override_aud_val) => (
            cached_pad_and_hash_string(override_aud_val, IdCommitment::MAX_AUD_VAL_BYTES)?,
            ark_bn254::Fr::from(1),
        ),
        None => (*EMPTY_OVERRIDE_AUD_FIELD_HASH, ark_bn254::Fr::from(0)),
    };

    // Add the hash of the jwt_header with the "." separator appended
    let jwt_header_b64_with_separator = format!("{}.", base64url_encode_str(jwt_header_json));
    let jwt_header_hash = cached_pad_and_hash_string(
        &jwt_header_b64_with_separator,
        config.max_jwt_header_b64_bytes as usize,
    )?;

    let jwk_hash = cached_jwk_hash(jwk)?;

    // Add the hash of the value of the `iss` field
    let iss_field_hash = cached_pad_and_hash_string(iss, config.max_iss_val_bytes as usize)?;

    // Add the id_commitment as a scalar
    let idc = Fr::from_le_bytes_mod_order(&idc.0);

    // Add the exp_timestamp_secs as a scalar
    let exp_timestamp_secs = Fr::from(exp_timestamp_secs);

    // Add the epk lifespan as a scalar
    let exp_horizon_secs = Fr::from(exp_horizon_secs);

    let mut epk_frs = poseidon_bn254::keyless::pad_and_pack_bytes_to_scalars_with_len(
        epk.to_bytes().as_slice(),
        config.max_commited_epk_bytes as usize,
    )?;

    // println!("Num EPK scalars:    {}", epk_frs.len());
    // for (i, e) in epk_frs.iter().enumerate() {
    //     println!("EPK Fr[{}]:          {}", i, e.to_string())
    // }
    // println!("IDC:                {}", idc);
    // println!("exp_timestamp_secs: {}", exp_timestamp_secs);
    // println!("exp_horizon_secs:   {}", exp_horizon_secs);
    // println!("iss field:          {}", pk.iss_val);
    // println!("iss field hash:     {}", iss_field_hash);
    // println!("Has extra field:    {}", has_extra_field);
    // println!("Extra field val:    {:?}", proof.extra_field);
    // println!("Extra field hash:   {}", extra_field_hash);
    // println!("JWT header val:     {}", jwt_header_b64_with_separator);
    // println!("JWT header hash:    {}", jwt_header_hash);
    // println!("JWK hash:           {}", jwk_hash);
    // println!("Override aud hash:  {}", override_aud_val_hash);
    // println!("Use override aud:   {}", use_override_aud.to_string());

    let mut frs = vec![];
    frs.append(&mut epk_frs);
    frs.push(idc);
    frs.push(exp_timestamp_secs);
    frs.push(exp_horizon_secs);
    frs.push(iss_field_hash);
    frs.push(has_extra_field);
    frs.push(extra_field_hash);
    frs.push(jwt_header_hash);
    frs.push(jwk_hash);
    frs.push(override_aud_val_hash);
    frs.push(use_override_aud);
    // TODO(keyless): If we plan on avoiding verifying the same PIH twice, there should be no
    //  need for caching here. If we do not, we should cache the result here too.
    poseidon_bn254::hash_scalars(frs)
}

pub fn get_public_inputs_hash(
    sig: &KeylessSignature,
    pk: &KeylessPublicKey,
    jwk: &RSA_JWK,
    config: &Configuration,
) -> anyhow::Result<Fr> {
    if let EphemeralCertificate::ZeroKnowledgeSig(proof) = &sig.cert {
        hash_public_inputs(
            config,
            &sig.ephemeral_pubkey,
            &pk.idc,
            sig.exp_date_secs,
            proof.exp_horizon_secs,
            &pk.iss_val,
            proof.extra_field.as_deref(),
            &sig.jwt_header_json,
            jwk,
            proof.override_aud_val.as_deref(),
        )
    } else {
        bail!("Can only call `get_public_inputs_hash` on keyless::Signature with Groth16 ZK proof")
    }
}

#[cfg(test)]
mod test {
    use crate::keyless::{
        bn254_circom::{
            G1Bytes, G2Bytes, G1_PROJECTIVE_COMPRESSED_NUM_BYTES,
            G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
        },
        circuit_constants::prepared_vk_for_testing,
        Groth16VerificationKey,
    };
    use ark_bn254::Bn254;
    use ark_groth16::PreparedVerifyingKey;

    #[test]
    pub fn test_bn254_serialized_sizes() {
        let g1 = G1Bytes::new_unchecked(
            "16672231080302629756836614130913173861541009360974119524782950408048375831661",
            "1076145001163048025135533382088266750240489485046298539187659509488738517245",
        )
        .unwrap();

        let g2 = G2Bytes::new_unchecked(
            [
                "1125365732643211423779651913319958385653115422366520671538751860820509133538",
                "10055196097002324305342942912758079446356594743098794928675544207400347950287",
            ],
            [
                "10879716754714953827605171295191459580695363989155343984818520267224463075503",
                "440220374146936557739765173414663598678359360031905981547938788314460390904",
            ],
        )
        .unwrap();

        let g1_bytes = bcs::to_bytes(&g1).unwrap();
        assert_eq!(g1_bytes.len(), G1_PROJECTIVE_COMPRESSED_NUM_BYTES);

        let g2_bytes = bcs::to_bytes(&g2).unwrap();
        assert_eq!(g2_bytes.len(), G2_PROJECTIVE_COMPRESSED_NUM_BYTES);
    }

    #[test]
    // Tests conversion between the devnet ark_groth16::PreparedVerificationKey and our Move
    // representation of it.
    fn print_groth16_pvk() {
        let groth16_vk: Groth16VerificationKey = prepared_vk_for_testing().into();

        println!("alpha_g1: {:?}", hex::encode(groth16_vk.alpha_g1.clone()));
        println!("beta_g2: {:?}", hex::encode(groth16_vk.beta_g2.clone()));
        println!("gamma_g2: {:?}", hex::encode(groth16_vk.gamma_g2.clone()));
        println!("delta_g2: {:?}", hex::encode(groth16_vk.delta_g2.clone()));
        let gamma_abc_g1 = groth16_vk.gamma_abc_g1.clone();
        println!("gamma_abc_g1_0: {:?}", hex::encode(gamma_abc_g1[0].clone()));
        println!("gamma_abc_g1_1: {:?}", hex::encode(gamma_abc_g1[1].clone()));

        let same_pvk: PreparedVerifyingKey<Bn254> = groth16_vk.try_into().unwrap();

        assert_eq!(same_pvk, prepared_vk_for_testing());
    }
}
