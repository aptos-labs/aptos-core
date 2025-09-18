// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::authenticator::{AnyPublicKey, AnySignature, EphemeralPublicKey};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub mod account_recovery_db;
pub mod jwt;
pub mod vuf;

/// Custom serialization function to convert Vec<u8> into a hex string.
fn serialize_bytes_to_hex<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_string = hex::encode(bytes);
    serializer.serialize_str(&hex_string)
}

/// Custom deserialization function to convert a hex string back into Vec<u8>.
fn deserialize_bytes_from_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    hex::decode(s).map_err(D::Error::custom)
}

/// Custom serialization function to convert Vec<u8> into a hex string with the 0x prefix.
fn serialize_bytes_to_hex_with_0x<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex_string = format!("0x{}", hex::encode(bytes));
    serializer.serialize_str(&hex_string)
}

/// Custom deserialization function to convert a hex string with the 0x prefix back into Vec<u8>.
fn deserialize_bytes_from_hex_with_0x<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if !s.starts_with("0x") {
        return Err(D::Error::custom("String is not prefixed by '0x'"));
    }
    hex::decode(&s[2..]).map_err(D::Error::custom)
}

/// Custom serialization function to convert `EphemeralPublicKey` into a hex string.
fn serialize_epk_to_hex<S>(epk: &EphemeralPublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let epk_bytes = epk.to_bytes();
    serialize_bytes_to_hex(&epk_bytes, serializer)
}

fn deserialize_epk_from_hex<'de, D>(deserializer: D) -> Result<EphemeralPublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = deserialize_bytes_from_hex(deserializer)?;
    let pk = EphemeralPublicKey::try_from(bytes.as_slice()).map_err(D::Error::custom)?;
    Ok(pk)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BadPepperRequestError {
    pub message: String,
}

/// A pepper scheme where:
/// - The pepper input contains `JWT, epk, blinder, expiry_time, uid_key`, wrapped in type `PepperRequest`.
/// - The pepper output is the `BLS12381_G1_BLS` VUF output of the input, wrapped in type `PepperResponse`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperRequest {
    #[serde(rename = "jwt_b64")]
    pub jwt: String,
    #[serde(
        serialize_with = "serialize_epk_to_hex",
        deserialize_with = "deserialize_epk_from_hex"
    )]
    pub epk: EphemeralPublicKey,
    pub exp_date_secs: u64,
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub epk_blinder: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
}

/// The response to `PepperRequest`, which contains either the pepper or a processing error.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperResponse {
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub pepper: Vec<u8>,
    #[serde(
        serialize_with = "serialize_bytes_to_hex_with_0x",
        deserialize_with = "deserialize_bytes_from_hex_with_0x"
    )]
    pub address: Vec<u8>,
}

fn serialize_anypk_to_hex<S>(pk: &AnyPublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let pk_bytes = pk.to_bytes();
    serialize_bytes_to_hex(&pk_bytes, serializer)
}

fn deserialize_anypk_from_hex<'de, D>(deserializer: D) -> Result<AnyPublicKey, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = deserialize_bytes_from_hex(deserializer)?;
    let pk = AnyPublicKey::try_from(bytes.as_slice()).map_err(D::Error::custom)?;
    Ok(pk)
}

fn serialize_anysig_to_hex<S>(sig: &AnySignature, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let sig_bytes = sig.to_bytes();
    serialize_bytes_to_hex(&sig_bytes, serializer)
}

fn deserialize_anysig_from_hex<'de, D>(deserializer: D) -> Result<AnySignature, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes = deserialize_bytes_from_hex(deserializer)?;
    let sig = AnySignature::try_from(bytes.as_slice()).map_err(D::Error::custom)?;
    Ok(sig)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyRequest {
    #[serde(
        serialize_with = "serialize_anypk_to_hex",
        deserialize_with = "deserialize_anypk_from_hex"
    )]
    pub public_key: AnyPublicKey,
    #[serde(
        serialize_with = "serialize_anysig_to_hex",
        deserialize_with = "deserialize_anysig_from_hex"
    )]
    pub signature: AnySignature,
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub message: Vec<u8>,
    #[serde(
        serialize_with = "serialize_bytes_to_hex_with_0x",
        deserialize_with = "deserialize_bytes_from_hex_with_0x"
    )]
    pub address: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyResponse {
    pub success: bool,
}

/// The response to /signature, which contains the VUF signature.
#[derive(Debug, Deserialize, Serialize)]
pub struct SignatureResponse {
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub signature: Vec<u8>, // unique BLS signature
}

/// The response to `/v0/vuf-pub-key`.
/// NOTE that in pepper v0, VUF is fixed to be `BLS12381_G1_BLS`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperV0VufPubKey {
    #[serde(
        serialize_with = "serialize_bytes_to_hex",
        deserialize_with = "deserialize_bytes_from_hex"
    )]
    pub public_key: Vec<u8>,
}

impl PepperV0VufPubKey {
    pub fn new(public_key: Vec<u8>) -> Self {
        Self { public_key }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PepperInput {
    pub iss: String,
    pub aud: String,
    pub uid_val: String,
    pub uid_key: String,
}
