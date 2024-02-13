// Copyright © Aptos Foundation

use crate::{
    jwks::rsa::RSA_JWK,
    move_utils::as_move_value::AsMoveValue,
    zkid::{Configuration, IdCommitment, ZkIdPublicKey, ZkIdSignature, ZkpOrOpenIdSig},
};
use anyhow::bail;
use aptos_crypto::{poseidon_bn254, CryptoMaterialError};
use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G1Projective, G2Affine, G2Projective};
use ark_ff::PrimeField;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::fmt::{Display, Formatter};

// TODO(zkid): Some of this stuff, if not all, belongs to the aptos-crypto crate

pub const G1_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 32;
pub const G2_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 64;

/// Useful macro for arkworks serialization!
macro_rules! serialize {
    ($obj:expr) => {{
        let mut buf = vec![];
        $obj.serialize_compressed(&mut buf).unwrap();
        buf
    }};
}

/// Reflection of aptos_framework::zkid::Groth16PreparedVerificationKey
#[derive(Serialize, Deserialize, Debug)]
pub struct Groth16VerificationKey {
    pub alpha_g1: Vec<u8>,
    pub beta_g2: Vec<u8>,
    pub gamma_g2: Vec<u8>,
    pub delta_g2: Vec<u8>,
    pub gamma_abc_g1: Vec<Vec<u8>>,
}

impl AsMoveValue for Groth16VerificationKey {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.alpha_g1.as_move_value(),
            self.beta_g2.as_move_value(),
            self.gamma_g2.as_move_value(),
            self.delta_g2.as_move_value(),
            self.gamma_abc_g1.as_move_value(),
        ]))
    }
}

/// WARNING: This struct uses resource groups on the Move side. Do NOT implement OnChainConfig
/// for it, since `OnChainConfig::fetch_config` does not work with resource groups (yet).
impl MoveStructType for Groth16VerificationKey {
    const MODULE_NAME: &'static IdentStr = ident_str!("zkid");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Groth16VerificationKey");
}

impl TryFrom<Groth16VerificationKey> for PreparedVerifyingKey<Bn254> {
    type Error = CryptoMaterialError;

    fn try_from(vk: Groth16VerificationKey) -> Result<Self, Self::Error> {
        if vk.gamma_abc_g1.len() != 2 {
            return Err(CryptoMaterialError::DeserializationError);
        }

        Ok(Self::from(VerifyingKey {
            alpha_g1: G1Affine::deserialize_compressed(vk.alpha_g1.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            beta_g2: G2Affine::deserialize_compressed(vk.beta_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            gamma_g2: G2Affine::deserialize_compressed(vk.gamma_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            delta_g2: G2Affine::deserialize_compressed(vk.delta_g2.as_slice())
                .map_err(|_| CryptoMaterialError::DeserializationError)?,
            gamma_abc_g1: vec![
                G1Affine::deserialize_compressed(vk.gamma_abc_g1[0].as_slice())
                    .map_err(|_| CryptoMaterialError::DeserializationError)?,
                G1Affine::deserialize_compressed(vk.gamma_abc_g1[1].as_slice())
                    .map_err(|_| CryptoMaterialError::DeserializationError)?,
            ],
        }))
    }
}

impl From<PreparedVerifyingKey<Bn254>> for Groth16VerificationKey {
    fn from(pvk: PreparedVerifyingKey<Bn254>) -> Self {
        let PreparedVerifyingKey {
            vk:
                VerifyingKey {
                    alpha_g1,
                    beta_g2,
                    gamma_g2,
                    delta_g2,
                    gamma_abc_g1,
                },
            alpha_g1_beta_g2: _alpha_g1_beta_g2, // unnecessary for Move
            gamma_g2_neg_pc: _gamma_g2_neg_pc,   // unnecessary for Move
            delta_g2_neg_pc: _delta_g2_neg_pc,   // unnecessary for Move
        } = pvk;

        let mut gamma_abc_g1_bytes = Vec::with_capacity(gamma_abc_g1.len());
        for e in gamma_abc_g1.iter() {
            gamma_abc_g1_bytes.push(serialize!(e));
        }

        Groth16VerificationKey {
            alpha_g1: serialize!(alpha_g1),
            beta_g2: serialize!(beta_g2),
            gamma_g2: serialize!(gamma_g2),
            delta_g2: serialize!(delta_g2),
            gamma_abc_g1: gamma_abc_g1_bytes,
        }
    }
}

impl Display for Groth16VerificationKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "alpha_g1: {}", hex::encode(&self.alpha_g1))?;
        write!(f, "beta_g2: {}", hex::encode(&self.beta_g2))?;
        write!(f, "gamma_g2: {}", hex::encode(&self.gamma_g2))?;
        write!(f, "delta_g2: {}", hex::encode(&self.delta_g2))?;
        for (i, e) in self.gamma_abc_g1.iter().enumerate() {
            write!(f, "gamma_abc_g1[{i}]: {}", hex::encode(serialize!(e)))?;
        }
        Ok(())
    }
}

pub static DEVNET_VERIFYING_KEY: Lazy<PreparedVerifyingKey<Bn254>> = Lazy::new(devnet_pvk);

/// This will do the proper subgroup membership checks.
fn g1_projective_str_to_affine(x: &str, y: &str) -> anyhow::Result<G1Affine> {
    let g1_affine = G1Bytes::new_unchecked(x, y)?.deserialize_into_affine()?;
    Ok(g1_affine)
}

/// This will do the proper subgroup membership checks.
fn g2_projective_str_to_affine(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<G2Affine> {
    let g2_affine = G2Bytes::new_unchecked(x, y)?.to_affine()?;
    Ok(g2_affine)
}

/// This function uses the decimal uncompressed point serialization which is outputted by circom.
fn devnet_pvk() -> PreparedVerifyingKey<Bn254> {
    // Convert the projective points to affine.
    let alpha_g1 = g1_projective_str_to_affine(
        "16672231080302629756836614130913173861541009360974119524782950408048375831661",
        "1076145001163048025135533382088266750240489485046298539187659509488738517245",
    )
    .unwrap();

    let beta_g2 = g2_projective_str_to_affine(
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

    let gamma_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let delta_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        g1_projective_str_to_affine(
            "709845293616032000883655261014820428774807602111296273992483611119383326362",
            "645961711055726048875381920095150798755926517220714963239815637182963128467",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "9703775855460452449287141941638080366156266996878046656846622159120386001635",
            "1903615495723998350630869740881559921229604803173196414121492346747062004184",
        )
        .unwrap(),
    ] {
        gamma_abc_g1.push(points);
    }

    let vk = VerifyingKey {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    };

    PreparedVerifyingKey::from(vk)
}

fn parse_field_element(s: &str) -> Result<Fq, CryptoMaterialError> {
    s.parse::<Fq>()
        .map_err(|_e| CryptoMaterialError::DeserializationError)
}

macro_rules! serialize {
    ($obj:expr, $method:ident) => {{
        let mut buf = vec![];
        $obj.$method(&mut buf)?;
        buf
    }};
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct G1Bytes(pub(crate) [u8; G1_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G1Bytes {
    pub fn new_unchecked(x: &str, y: &str) -> anyhow::Result<Self> {
        let g1 = G1Projective::new_unchecked(
            parse_field_element(x)?,
            parse_field_element(y)?,
            parse_field_element("1")?,
        );

        let bytes: Vec<u8> = serialize!(g1, serialize_compressed);
        Self::new_from_vec(bytes)
    }

    /// Used internall or for testing.
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct G2Bytes(#[serde(with = "BigArray")] pub(crate) [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G2Bytes {
    pub fn new_unchecked(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<Self> {
        let g2 = G2Projective::new_unchecked(
            Fq2::new(parse_field_element(x[0])?, parse_field_element(x[1])?),
            Fq2::new(parse_field_element(y[0])?, parse_field_element(y[1])?),
            Fq2::new(parse_field_element("1")?, parse_field_element("0")?),
        );

        let bytes: Vec<u8> = serialize!(g2, serialize_compressed);
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

    pub fn to_affine(&self) -> Result<G2Affine, CryptoMaterialError> {
        self.try_into()
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

pub fn get_public_inputs_hash(
    sig: &ZkIdSignature,
    pk: &ZkIdPublicKey,
    jwk: &RSA_JWK,
    exp_horizon_secs: u64,
    config: &Configuration,
) -> anyhow::Result<Fr> {
    let extra_field_hashed;
    let override_aud_val_hashed;
    let use_override_aud;
    if let ZkpOrOpenIdSig::Groth16Zkp(proof) = &sig.sig {
        extra_field_hashed = poseidon_bn254::pad_and_hash_string(
            &proof.extra_field,
            config.max_extra_field_bytes as usize,
        )?;
        if let Some(override_aud_val) = &proof.override_aud_val {
            use_override_aud = ark_bn254::Fr::from(1);
            override_aud_val_hashed = poseidon_bn254::pad_and_hash_string(
                override_aud_val,
                IdCommitment::MAX_AUD_VAL_BYTES,
            )?;
        } else {
            use_override_aud = ark_bn254::Fr::from(0);
            override_aud_val_hashed =
                poseidon_bn254::pad_and_hash_string("", IdCommitment::MAX_AUD_VAL_BYTES)?;
        }
    } else {
        bail!("Cannot get_public_inputs_hash for ZkIdSignature")
    }

    // Add the epk as padded and packed scalars
    let mut frs = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
        sig.ephemeral_pubkey.to_bytes().as_slice(),
        config.max_commited_epk_bytes as usize,
    )?;

    // Add the id_commitment as a scalar
    frs.push(Fr::from_le_bytes_mod_order(&pk.idc.0));

    // Add the exp_timestamp_secs as a scalar
    frs.push(Fr::from(sig.exp_timestamp_secs));

    // Add the epk lifespan as a scalar
    frs.push(Fr::from(exp_horizon_secs));

    // Add the hash of the iss (formatted key-value pair string).
    let formatted_iss = format!("\"iss\":\"{}\",", pk.iss);
    frs.push(poseidon_bn254::pad_and_hash_string(
        &formatted_iss,
        config.max_iss_field_bytes as usize,
    )?);

    frs.push(extra_field_hashed);

    // Add the hash of the jwt_header with the "." separator appended
    let jwt_header_with_separator = format!("{}.", sig.jwt_header);
    frs.push(poseidon_bn254::pad_and_hash_string(
        &jwt_header_with_separator,
        config.max_jwt_header_b64_bytes as usize,
    )?);

    frs.push(jwk.to_poseidon_scalar()?);

    frs.push(override_aud_val_hashed);

    frs.push(use_override_aud);

    poseidon_bn254::hash_scalars(frs)
}

#[cfg(test)]
mod test {
    use crate::bn254_circom::{
        devnet_pvk, G1Bytes, G2Bytes, Groth16VerificationKey, G1_PROJECTIVE_COMPRESSED_NUM_BYTES,
        G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
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
        let groth16_vk: Groth16VerificationKey = devnet_pvk().into();
        let same_pvk: PreparedVerifyingKey<Bn254> = groth16_vk.try_into().unwrap();

        assert_eq!(same_pvk, devnet_pvk());
    }
}
