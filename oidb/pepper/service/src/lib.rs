// Copyright © Aptos Foundation

use crate::vuf_keys::VUF_SCHEME0_SK;
use anyhow::{anyhow, bail, ensure};
use aptos_oidb_pepper_common::{
    jwt::Claims, nonce_derivation, nonce_derivation::NonceDerivationScheme,
    pepper_pre_image_derivation, pepper_pre_image_derivation::PepperPreImageDerivation, vuf,
    vuf::VUF, PepperRequest, PepperResponse, SimplePepperRequest,
};
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
    match request.schema_version.as_deref() {
        None | Some("0.1.0") => match process_inner(request) {
            Ok(hexlified) => PepperResponse::Raw {
                pepper_hexlified: hexlified,
            },
            Err(e) => PepperResponse::Error(e.to_string()),
        },
        _ => PepperResponse::Error("Unsupported schema version".to_string()),
    }
}

fn process_inner(request: PepperRequest) -> anyhow::Result<String> {
    let PepperRequest {
        jwt,
        overriding_aud,
        epk_serialized_hexlified,
        expiry_time_sec,
        blinder_hexlified,
        uid_key,
        ..
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

    let vuf_input_source = pepper_pre_image_derivation::scheme1::Source {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_owned(),
        uid_val,
        aud: actual_aud.clone(),
    };

    let vuf_input = pepper_pre_image_derivation::scheme1::Scheme::derive(&vuf_input_source);

    let (pepper, vuf_proof) = vuf::scheme0::Scheme::eval(&VUF_SCHEME0_SK, &vuf_input)?;
    ensure!(
        vuf_proof.is_empty(),
        "aptos_oidb_pepper_service::process() failed internal proof error"
    );
    let pepper_hexlified = hex::encode(pepper);
    //TODO: encrypt the pepper
    Ok(pepper_hexlified)
}

pub async fn process_unencrypted(request: SimplePepperRequest) -> anyhow::Result<String> {
    let SimplePepperRequest { jwt, uid_key } = request;
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
    let actual_aud = &claims.claims.aud;

    let vuf_input_source = pepper_pre_image_derivation::scheme1::Source {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_owned(),
        uid_val,
        aud: actual_aud.clone(),
    };

    let vuf_input = pepper_pre_image_derivation::scheme1::Scheme::derive(&vuf_input_source);

    let (pepper, vuf_proof) = vuf::scheme0::Scheme::eval(&VUF_SCHEME0_SK, &vuf_input)?;
    ensure!(
        vuf_proof.is_empty(),
        "aptos_oidb_pepper_service::process() failed internal proof error"
    );
    let pepper_hexlified = hex::encode(pepper);
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
