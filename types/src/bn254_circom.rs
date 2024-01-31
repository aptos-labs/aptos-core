// Copyright © Aptos Foundation

use crate::{
    jwks::rsa::RSA_JWK,
    zkid::{
        ZkIdPublicKey, ZkIdSignature, MAX_EPK_BYTES, MAX_EXPIRY_HORIZON_SECS, MAX_ISS_BYTES,
        MAX_JWT_HEADER_BYTES,
    },
};
use anyhow::bail;
use aptos_crypto::{poseidon_bn254, CryptoMaterialError};
use ark_bn254::{Fq, Fq2};
use ark_ff::PrimeField;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

// TODO(zkid): Some of this stuff, if not all, belongs to the aptos-crypto crate

pub const G1_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 32;
pub const G2_PROJECTIVE_COMPRESSED_NUM_BYTES: usize = 64;

pub static DEVNET_VERIFYING_KEY: Lazy<PreparedVerifyingKey<ark_bn254::Bn254>> =
    Lazy::new(devnet_pvk);

/// This will do the proper subgroup membership checks.
fn g1_projective_str_to_affine(x: &str, y: &str) -> anyhow::Result<ark_bn254::G1Affine> {
    let g1_affine = G1Bytes::new_unchecked(x, y)?.deserialize_into_affine()?;
    Ok(g1_affine)
}

/// This will do the proper subgroup membership checks.
fn g2_projective_str_to_affine(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<ark_bn254::G2Affine> {
    let g2_affine = G2Bytes::new_unchecked(x, y)?.to_affine()?;
    Ok(g2_affine)
}

fn devnet_pvk() -> PreparedVerifyingKey<ark_bn254::Bn254> {
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
            "19799867077440075892798570892827678991452882191483986973420950266983588147526",
            "7261406229996412667156189606964369006242293247396567701023787052439810543589",
        ],
        [
            "15618356441847575237880159451782511420373837463064250522093342825487687558812",
            "20490123502151072560031041764173142979409281632225526952209676367033524880945",
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        g1_projective_str_to_affine(
            "16119992548622948701752093197035559180088659648245261797962160821523395857787",
            "10895012769720065848112628781322097989082134121307195027616506940584635557433",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "12743680909720798417558674763081930985009983383780261525309863653205478749832",
            "10808093222645961212778297519773755506856954740368509958745099866520706196565",
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
        let g1 = ark_bn254::G1Projective::new_unchecked(
            parse_field_element(x)?,
            parse_field_element(y)?,
            parse_field_element("1")?,
        );

        let bytes: Vec<u8> = serialize!(g1, serialize_compressed);
        Self::new_from_vec(bytes)
    }

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

    pub fn deserialize_into_affine(&self) -> Result<ark_bn254::G1Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl TryInto<ark_bn254::G1Projective> for &G1Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<ark_bn254::G1Projective, CryptoMaterialError> {
        ark_bn254::G1Projective::deserialize_compressed(self.0.as_slice())
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl TryInto<ark_bn254::G1Affine> for &G1Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<ark_bn254::G1Affine, CryptoMaterialError> {
        let g1_projective: ark_bn254::G1Projective = self.try_into()?;
        Ok(g1_projective.into())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct G2Bytes(#[serde(with = "BigArray")] pub(crate) [u8; G2_PROJECTIVE_COMPRESSED_NUM_BYTES]);

impl G2Bytes {
    pub fn new_unchecked(x: [&str; 2], y: [&str; 2]) -> anyhow::Result<Self> {
        let g2 = ark_bn254::G2Projective::new_unchecked(
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

    pub fn to_affine(&self) -> Result<ark_bn254::G2Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl TryInto<ark_bn254::G2Projective> for &G2Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<ark_bn254::G2Projective, CryptoMaterialError> {
        ark_bn254::G2Projective::deserialize_compressed(self.0.as_slice())
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

impl TryInto<ark_bn254::G2Affine> for &G2Bytes {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<ark_bn254::G2Affine, CryptoMaterialError> {
        let g2_projective: ark_bn254::G2Projective = self.try_into()?;
        Ok(g2_projective.into())
    }
}

pub fn get_public_inputs_hash(
    sig: &ZkIdSignature,
    pk: &ZkIdPublicKey,
    jwk: &RSA_JWK,
) -> anyhow::Result<ark_bn254::Fr> {
    // Add the epk as padded and packed scalars
    let mut frs = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
        sig.ephemeral_pubkey.to_bytes().as_slice(),
        MAX_EPK_BYTES,
    )?;

    // Add the id_commitment as a scalar
    frs.push(ark_bn254::Fr::from_le_bytes_mod_order(&pk.idc.0));

    // Add the exp_timestamp_secs as a scalar
    frs.push(ark_bn254::Fr::from(sig.exp_timestamp_secs));

    // Add the epk lifespan as a scalar
    frs.push(ark_bn254::Fr::from(MAX_EXPIRY_HORIZON_SECS));

    // Add the hash of the iss (formatted key-value pair string).
    let formatted_iss = format!("\"iss\":\"{}\",", pk.iss);
    frs.push(poseidon_bn254::pad_and_hash_string(
        &formatted_iss,
        MAX_ISS_BYTES,
    )?);

    // Add the hash of the jwt_header with the "." separator appended
    let jwt_header_with_separator = format!("{}.", sig.jwt_header);
    frs.push(poseidon_bn254::pad_and_hash_string(
        &jwt_header_with_separator,
        MAX_JWT_HEADER_BYTES,
    )?);

    frs.push(jwk.to_poseidon_scalar()?);

    poseidon_bn254::hash_scalars(frs)
}

#[cfg(test)]
mod test {
    use crate::bn254_circom::{
        G1Bytes, G2Bytes, G1_PROJECTIVE_COMPRESSED_NUM_BYTES, G2_PROJECTIVE_COMPRESSED_NUM_BYTES,
    };

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
}
