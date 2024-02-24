// Copyright © Aptos Foundation

use crate::vuf_keys::VUF_SCHEME0_SK;
use anyhow::{anyhow, bail, ensure};
use aptos_oidb_pepper_common::{jwt::Claims, nonce_derivation, nonce_derivation::NonceDerivationScheme, vuf, vuf::VUF, PepperRequest, PepperResponse, PepperRequestV0, PepperResponseV0, PepperInput, PepperInputV0};
use jsonwebtoken::{Algorithm::RS256, Validation};
use once_cell::sync::Lazy;
use std::collections::HashSet;

pub mod about;
pub mod jwk;
pub mod vuf_keys;

pub type Issuer = String;
pub type KeyID = String;

/// The core processing logic of this pepper service.
pub async fn process(request: PepperRequest) -> PepperResponse {
    match request {
        PepperRequest::V0(req) => {
            let response = match process_v0(req) {
                Ok(pepper_hexlified) => PepperResponseV0::Ok { pepper_hexlified },
                Err(e) => PepperResponseV0::Error(e.to_string()),
            };
            PepperResponse::V0(response)
        }
    }
}

fn process_v0(request: PepperRequestV0) -> anyhow::Result<String> {
    let PepperRequestV0 {
        jwt,
        overriding_aud,
        epk_serialized_hexlified,
        expiry_time_sec,
        blinder_hexlified,
        uid_key,
    } = request;

    let claims = aptos_oidb_pepper_common::jwt::parse(jwt.as_str()).map_err(|e| {
        anyhow!("aptos_oidb_pepper_service::process() failed with jwt decoding error: {e}")
    })?;

    let actual_uid_key = if let Some(uid_key) = uid_key.as_ref() {
        uid_key
    } else {
        "sub"
    };

    let uid_val = if actual_uid_key == "email" {
        claims.claims.email.clone().ok_or_else(||anyhow!("aptos_oidb_pepper_service::process() failed with `email` required but not found in jwt"))?
    } else if actual_uid_key == "sub" {
        claims.claims.sub.clone()
    } else {
        bail!(
            "aptos_oidb_pepper_service::process() failed with unsupported uid key: {}",
            actual_uid_key
        )
    };

    let blinder = hex::decode(blinder_hexlified).map_err(|e| {
        anyhow!("aptos_oidb_pepper_service::process() failed with blinder hex decoding error: {e}")
    })?;
    let epk = hex::decode(epk_serialized_hexlified).map_err(|e| {
        anyhow!("aptos_oidb_pepper_service::process() failed with epk hex decoding error: {e}")
    })?;
    // TODO: OpenIdSig::reconstruct_oauth_nonce. Hardcode the config for now.
    let nonce_pre_image = nonce_derivation::scheme1::PreImage {
        epk,
        expiry_time_sec,
        blinder,
    };
    let recalculated_nonce = nonce_derivation::scheme1::Scheme::derive_nonce(&nonce_pre_image);

    ensure!(
        claims.claims.nonce == hex::encode(recalculated_nonce),
        "aptos_oidb_pepper_service::process() failed with nonce mismatch"
    );

    let key_id = claims
        .header
        .kid
        .ok_or_else(|| anyhow!("aptos_oidb_pepper_service::process() failed with missing kid"))?;

    let sig_vrfy_key = jwk::cached_decoding_key(&claims.claims.iss, &key_id)?;
    let validation_with_sig_vrfy = Validation::new(RS256);
    let _claims = jsonwebtoken::decode::<Claims>(
        jwt.as_str(),
        sig_vrfy_key.as_ref(),
        &validation_with_sig_vrfy,
    ) // Signature verification happens here.
    .map_err(|e| {
        anyhow!("aptos_oidb_pepper_service::process() failed with jwt decoding error 2: {e}")
    })?;

    // Decide the client_id in the input.
    let actual_aud = if ACCOUNT_DISCOVERY_CLIENTS.contains(&claims.claims.aud) {
        if let Some(aud) = overriding_aud.as_ref() {
            aud
        } else {
            &claims.claims.aud
        }
    } else {
        &claims.claims.aud
    };

    let input = PepperInput::V0( PepperInputV0 {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_owned(),
        uid_val,
        aud: actual_aud.clone(),
    });
    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper, vuf_proof) = vuf::scheme0::Scheme0::eval(&VUF_SCHEME0_SK, &input_bytes)?;
    ensure!(
        vuf_proof.is_empty(),
        "aptos_oidb_pepper_service::process() failed internal proof error"
    );
    let pepper_hexlified = hex::encode(pepper);
    //TODO: encrypt the pepper
    Ok(pepper_hexlified)
}

/// The set of the privileged clients.
///
/// TODO: should be loaded from env/an external service.
pub static ACCOUNT_DISCOVERY_CLIENTS: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("407408718192.apps.googleusercontent.com".to_string()); // Google OAuth 2.0 Playground
    set
});
