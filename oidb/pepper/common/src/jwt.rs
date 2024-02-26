// Copyright © Aptos Foundation

use anyhow::anyhow;
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// The claims required in a JWT.
#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    pub nonce: String,
    pub iss: String,
    pub sub: String,
    pub email: Option<String>,
    pub azp: Option<String>,
    pub aud: String,
    pub iat: u64,
    pub exp: u64,
}

/// Simply parse the fields out without performing signature verification.
pub fn parse(jwt: &str) -> anyhow::Result<TokenData<Claims>> {
    jsonwebtoken::decode::<Claims>(
        jwt,
        DUMMY_DECODING_KEY.deref(),
        VALIDATION_CONFIG_NO_SIG_VRFY.deref(),
    )
    .map_err(|e| anyhow!("jwt decoding error: {}", e))
}

static DUMMY_DECODING_KEY: Lazy<DecodingKey> = Lazy::new(|| DecodingKey::from_secret(&[]));

static VALIDATION_CONFIG_NO_SIG_VRFY: Lazy<Validation> = Lazy::new(|| {
    let mut config = Validation::default();
    config.insecure_disable_signature_validation();
    config.validate_exp = false;
    config
});
