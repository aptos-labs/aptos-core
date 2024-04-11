// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_managers::ACCOUNT_MANAGERS,
    vuf_keys::VUF_SK,
    ProcessingFailure::{BadRequest, InternalError},
};
use aptos_crypto::asymmetric_encryption::{
    elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm, AsymmetricEncryption,
};
use aptos_keyless_pepper_common::{
    jwt::Claims,
    vuf::{self, VUF},
    PepperInput, PepperRequest, PepperRequestV1, PepperResponse, PepperResponseV1,
};
use aptos_types::{
    keyless::{Configuration, OpenIdSig},
    transaction::authenticator::EphemeralPublicKey,
};
use jsonwebtoken::{Algorithm::RS256, Validation};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod about;
pub mod account_managers;
pub mod jwk;
pub mod metrics;
pub mod vuf_keys;

pub type Issuer = String;
pub type KeyID = String;

#[derive(Debug, Deserialize, Serialize)]
pub enum ProcessingFailure {
    BadRequest(String),
    InternalError(String),
}

pub fn process_v0(request: PepperRequest) -> Result<PepperResponse, ProcessingFailure> {
    let PepperRequest {
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
    } = request;
    let pepper = process_common(jwt, epk, exp_date_secs, epk_blinder, uid_key, false, None)?;
    Ok(PepperResponse { signature: pepper })
}

pub fn process_v1(request: PepperRequestV1) -> Result<PepperResponseV1, ProcessingFailure> {
    let PepperRequestV1 {
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        aud,
    } = request;
    let pepper_encrypted =
        process_common(jwt, epk, exp_date_secs, epk_blinder, uid_key, true, aud)?;
    Ok(PepperResponseV1 {
        signature_encrypted: pepper_encrypted,
    })
}

fn process_common(
    jwt: String,
    epk: EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    uid_key: Option<String>,
    encrypts_pepper: bool,
    aud: Option<String>,
) -> Result<Vec<u8>, ProcessingFailure> {
    let config = Configuration::new_for_devnet();

    let curve25519_pk_point = match &epk {
        EphemeralPublicKey::Ed25519 { public_key } => public_key
            .to_compressed_edwards_y()
            .decompress()
            .ok_or_else(|| BadRequest("the pk point is off-curve".to_string()))?,
        _ => {
            return Err(BadRequest("Only Ed25519 epk is supported".to_string()));
        },
    };

    let claims = aptos_keyless_pepper_common::jwt::parse(jwt.as_str())
        .map_err(|e| BadRequest(format!("JWT decoding error: {e}")))?;
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if exp_date_secs <= now_secs {
        return Err(BadRequest("epk expired".to_string()));
    }

    if exp_date_secs >= claims.claims.iat + config.max_exp_horizon_secs {
        return Err(BadRequest("epk expiry date too far".to_string()));
    }

    let actual_uid_key = if let Some(uid_key) = uid_key.as_ref() {
        uid_key
    } else {
        "sub"
    };

    let uid_val = if actual_uid_key == "email" {
        claims
            .claims
            .email
            .clone()
            .ok_or_else(|| BadRequest("`email` required but not found in jwt".to_string()))?
    } else if actual_uid_key == "sub" {
        claims.claims.sub.clone()
    } else {
        return Err(BadRequest(format!(
            "unsupported uid key: {}",
            actual_uid_key
        )));
    };

    let recalculated_nonce =
        OpenIdSig::reconstruct_oauth_nonce(epk_blinder.as_slice(), exp_date_secs, &epk, &config)
            .map_err(|e| BadRequest(format!("nonce reconstruction error: {e}")))?;

    if claims.claims.nonce != recalculated_nonce {
        return Err(BadRequest("with nonce mismatch".to_string()));
    }

    let key_id = claims
        .header
        .kid
        .ok_or_else(|| BadRequest("missing kid in JWT".to_string()))?;

    let sig_pub_key = jwk::cached_decoding_key(&claims.claims.iss, &key_id)
        .map_err(|e| BadRequest(format!("JWK not found: {e}")))?;
    let mut validation_with_sig_verification = Validation::new(RS256);
    validation_with_sig_verification.validate_exp = false; // Don't validate the exp time
    let _claims = jsonwebtoken::decode::<Claims>(
        jwt.as_str(),
        sig_pub_key.as_ref(),
        &validation_with_sig_verification,
    ) // Signature verification happens here.
    .map_err(|e| BadRequest(format!("JWT signature verification failed: {e}")))?;

    // If the pepper request is at V1, is from an account manager, and has a target aud specified, compute the pepper for the target aud.
    let mut final_aud = claims.claims.aud.clone();
    if ACCOUNT_MANAGERS.contains(&(claims.claims.iss.clone(), claims.claims.aud.clone())) {
        if let Some(aud) = aud {
            final_aud = aud;
        }
    };

    let input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_string(),
        uid_val,
        aud: final_aud,
    };
    info!(
        session_id = session_id,
        iss = input.iss,
        aud = input.aud,
        uid_val = input.uid_val,
        uid_key = input.uid_key,
        "PepperInput is available."
    );
    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper, vuf_proof) = vuf::bls12381_g1_bls::Bls12381G1Bls::eval(&VUF_SK, &input_bytes)
        .map_err(|e| InternalError(format!("bls12381_g1_bls eval error: {e}")))?;
    if !vuf_proof.is_empty() {
        return Err(InternalError("proof size should be 0".to_string()));
    }

    if encrypts_pepper {
        let mut main_rng = thread_rng();
        let mut aead_rng = aes_gcm::aead::OsRng;
        let pepper_encrypted = ElGamalCurve25519Aes256Gcm::enc(
            &mut main_rng,
            &mut aead_rng,
            &curve25519_pk_point,
            &pepper,
        )
        .map_err(|e| InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}")))?;
        Ok(pepper_encrypted)
    } else {
        Ok(pepper)
    }
}
