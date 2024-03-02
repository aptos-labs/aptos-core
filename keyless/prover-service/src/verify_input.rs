use anyhow::Result;
use aptos_types::jwks::rsa::RSA_JWK;

use crate::{input_conversion::config, input_conversion::types::Input};

pub fn verify_input(
    input : Input,
    _config : &config::CircuitConfig,
    jwk: &RSA_JWK,
    ) -> Result<()> {
    jwk.verify_signature(&input.jwt_b64)?;
    Ok(())
}
