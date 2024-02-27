// Copyright © Aptos Foundation

use serde::{Deserialize, Serialize};

pub mod jwt;
pub mod vuf;

/// The spec of a request to this pepper service.
#[derive(Debug, Deserialize, Serialize)]
pub enum PepperRequest {
    V0(PepperRequestV0),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PepperResponse {
    Error(String),
    V0(PepperResponseV0),
}

/// A pepper scheme where:
/// - The pepper input contains `JWT, epk, blinder, expiry_time, uid_key`, wrapped in type `PepperRequestV0`.
/// - The pepper output is the `BLS12381_G1_BLS` VUF output of the input, wrapped in type `PepperResponseV0`.
#[derive(Debug, Deserialize, Serialize)]
pub struct PepperRequestV0 {
    pub jwt: String,
    pub epk_hex_string: String,
    pub epk_expiry_time_secs: u64,
    pub epk_blinder_hex_string: String,
    pub uid_key: Option<String>,
}

/// The response to `PepperRequestV0`, which contains either the pepper or a processing error.
pub type PepperResponseV0 = Result<Vec<u8>, String>;

#[derive(Debug, Deserialize, Serialize)]
pub struct VUFVerificationKey {
    pub scheme_name: String,
    pub vuf_public_key_hex_string: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PepperInput {
    pub iss: String,
    pub aud: String,
    pub uid_val: String,
    pub uid_key: String,
}
