// Copyright © Aptos Foundation

use crate::{
    jwks::rsa::RSA_JWK,
    zkid::{
        ZkIdPublicKey, ZkIdSignature, MAX_EPK_BYTES, MAX_EXPIRY_HORIZON_SECS, MAX_ISS_BYTES,
        MAX_JWT_HEADER_BYTES,
    },
};
use aptos_crypto::{poseidon_bn254, CryptoMaterialError};
use ark_bn254::{Fq, Fq2, G1Affine, G2Affine};
use ark_ff::PrimeField;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static DEVNET_VERIFYING_KEY: Lazy<PreparedVerifyingKey<ark_bn254::Bn254>> = Lazy::new(devnet_pvk);

fn devnet_pvk() -> PreparedVerifyingKey<ark_bn254::Bn254> {
    // Convert the projective points to affine.
    let alpha_g1 = G1Projective::new(
        "16672231080302629756836614130913173861541009360974119524782950408048375831661",
        "1076145001163048025135533382088266750240489485046298539187659509488738517245",
    )
    .to_affine()
    .unwrap();

    let beta_g2 = G2Projective::new(
        [
            "1125365732643211423779651913319958385653115422366520671538751860820509133538",
            "10055196097002324305342942912758079446356594743098794928675544207400347950287",
        ],
        [
            "10879716754714953827605171295191459580695363989155343984818520267224463075503",
            "440220374146936557739765173414663598678359360031905981547938788314460390904",
        ],
    )
    .to_affine()
    .unwrap();

    let gamma_g2 = G2Projective::new(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .to_affine()
    .unwrap();

    let delta_g2 = gamma_g2;

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        G1Projective::new(
            "10630119204695129176884860852234232187032863639334371023708138007302523646865",
            "8100947059469766601395165113187306282631271312167186605231839390439402060594",
        )
        .to_affine()
        .unwrap(),
        G1Projective::new(
            "18669717593291583006164561820680929698908561353625908867516300854867219058689",
            "8091804270019087529935049146021494025057159496668931947922664231857415567945",
        )
        .to_affine()
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

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct G1Projective {
    x: String,
    y: String,
    z: String,
}

impl G1Projective {
    pub fn new(x: &str, y: &str) -> Self {
        G1Projective {
            x: x.to_owned(),
            y: y.to_owned(),
            z: "1".to_string(),
        }
    }

    pub fn to_affine(&self) -> Result<G1Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl TryInto<G1Affine> for &G1Projective {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G1Affine, CryptoMaterialError> {
        let g1 = ark_bn254::G1Projective::new_unchecked(
            parse_field_element(&self.x)?,
            parse_field_element(&self.y)?,
            parse_field_element(&self.z)?,
        );
        Ok(g1.into())
    }
}

pub type Fq2Str = [String; 2];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
pub struct G2Projective {
    x: Fq2Str,
    y: Fq2Str,
    z: Fq2Str,
}

impl G2Projective {
    pub fn new(x: [&str; 2], y: [&str; 2]) -> Self {
        G2Projective {
            x: [x[0].to_owned(), x[1].to_owned()],
            y: [y[0].to_owned(), y[1].to_owned()],
            z: ["1".to_string(), "0".to_owned()],
        }
    }

    pub fn to_affine(&self) -> Result<G2Affine, CryptoMaterialError> {
        self.try_into()
    }
}

impl TryInto<G2Affine> for &G2Projective {
    type Error = CryptoMaterialError;

    fn try_into(self) -> Result<G2Affine, CryptoMaterialError> {
        let g2 = ark_bn254::G2Projective::new_unchecked(
            Fq2::new(
                parse_field_element(&self.x[0])?,
                parse_field_element(&self.x[1])?,
            ),
            Fq2::new(
                parse_field_element(&self.y[0])?,
                parse_field_element(&self.y[1])?,
            ),
            Fq2::new(
                parse_field_element(&self.z[0])?,
                parse_field_element(&self.z[1])?,
            ),
        );
        Ok(g2.into())
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

    let iat_val = 1700255944;
    // Add the iat val as a scalar TODO - update this when circuit updated
    frs.push(ark_bn254::Fr::from(iat_val));

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
