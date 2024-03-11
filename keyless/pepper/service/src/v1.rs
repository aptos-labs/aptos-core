// Copyright © Aptos Foundation

use std::time::{SystemTime, UNIX_EPOCH};
use jsonwebtoken::Algorithm::RS256;
use jsonwebtoken::Validation;
use rand::thread_rng;
use aptos_crypto::asymmetric_encryption::AsymmetricEncryption;
use aptos_crypto::asymmetric_encryption::elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm;
use aptos_keyless_pepper_common::{PepperInput, PepperRequestV1, PepperResponseV1, vuf};
use aptos_keyless_pepper_common::jwt::Claims;
use aptos_keyless_pepper_common::vuf::VUF;
use aptos_types::keyless::{Configuration, OpenIdSig};
use aptos_types::transaction::authenticator::EphemeralPublicKey;
use crate::jwk;
use crate::v1::ProcessingFailure::{BadRequest, InternalError};
use crate::vuf_keys::VUF_SK;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum ProcessingFailure {
    BadRequest(String),
    InternalError(String),
}

pub fn process(request: PepperRequestV1) -> Result<PepperResponseV1, ProcessingFailure> {
    let PepperRequestV1 {
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        ..
    } = request;
    let config = Configuration::new_for_devnet();

    let curve25519_pk_point = match &epk {
        EphemeralPublicKey::Ed25519 { public_key } => {
            public_key.to_compressed_edwards_y().decompress().ok_or_else(||BadRequest("the pk point is off-curve".to_string()))?
        }
        _ => {
            return Err(BadRequest("Only Ed25519 epk is supported".to_string()));
        }
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

    let input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_string(),
        uid_val,
        aud: claims.claims.aud.clone(),
    };
    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper, vuf_proof) = vuf::bls12381_g1_bls::Bls12381G1Bls::eval(&VUF_SK, &input_bytes)
        .map_err(|e| InternalError(format!("bls12381_g1_bls eval error: {e}")))?;
    if !vuf_proof.is_empty() {
        return Err(InternalError("proof size should be 0".to_string()));
    }
    let mut main_rng = thread_rng();
    let mut aead_rng = aes_gcm::aead::OsRng;
    let signature_encrypted = ElGamalCurve25519Aes256Gcm::enc(&mut main_rng, &mut aead_rng, &curve25519_pk_point, &pepper).map_err(|e|InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}")))?;
    Ok(PepperResponseV1 { signature_encrypted })
}
