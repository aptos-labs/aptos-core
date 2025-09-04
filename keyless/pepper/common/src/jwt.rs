// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use jsonwebtoken::{DecodingKey, TokenData, Validation};
use serde::{Deserialize, Serialize};

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
    let empty_decoding_key = DecodingKey::from_secret(&[]);
    let parse_only_validation = {
        let mut config = Validation::default();
        config.insecure_disable_signature_validation();
        config.validate_exp = false;
        config
    };

    jsonwebtoken::decode::<Claims>(jwt, &empty_decoding_key, &parse_only_validation)
        .map_err(|e| anyhow!("jwt decoding error: {}", e))
}
