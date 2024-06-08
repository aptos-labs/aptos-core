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
    vuf::{
        self,
        bls12381_g1_bls::PinkasPepper,
        slip_10::{get_aptos_derivation_path, ExtendedPepper},
        VUF,
    },
    PepperInput, PepperRequest, PepperResponse, SignatureResponse,
};
use aptos_logger::info;
use aptos_types::{
    account_address::AccountAddress,
    keyless::{Configuration, IdCommitment, KeylessPublicKey, OpenIdSig},
    transaction::authenticator::{AnyPublicKey, AuthenticationKey, EphemeralPublicKey},
};
use jsonwebtoken::{Algorithm::RS256, Validation};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/637'/0'/0'/0'";

pub fn process_v0(request: PepperRequest) -> Result<PepperResponse, ProcessingFailure> {
    let session_id = Uuid::new_v4();
    let PepperRequest {
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        derivation_path,
    } = request;

    let (_pepper_base, pepper, address) = process_common(
        &session_id,
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        derivation_path,
        false,
        None,
    )?;

    Ok(PepperResponse {
        pepper,
        address: address.to_vec(),
    })
}

pub fn process_signature_v0(
    request: PepperRequest,
) -> Result<SignatureResponse, ProcessingFailure> {
    let session_id = Uuid::new_v4();
    let PepperRequest {
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        derivation_path,
    } = request;

    let (pepper_base, _pepper, _address) = process_common(
        &session_id,
        jwt,
        epk,
        exp_date_secs,
        epk_blinder,
        uid_key,
        derivation_path,
        false,
        None,
    )?;

    Ok(SignatureResponse {
        signature: pepper_base,
    })
}

fn process_common(
    session_id: &Uuid,
    jwt: String,
    epk: EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    uid_key: Option<String>,
    derivation_path: Option<String>,
    encrypts_pepper: bool,
    aud: Option<String>,
) -> Result<(Vec<u8>, Vec<u8>, AccountAddress), ProcessingFailure> {
    let config = Configuration::new_for_devnet();

    let derivation_path = if let Some(path) = derivation_path {
        path
    } else {
        DEFAULT_DERIVATION_PATH.to_owned()
    };
    let checked_derivation_path =
        get_aptos_derivation_path(&derivation_path).map_err(|e| BadRequest(e.to_string()))?;

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
    let (pepper_base, vuf_proof) = vuf::bls12381_g1_bls::Bls12381G1Bls::eval(&VUF_SK, &input_bytes)
        .map_err(|e| InternalError(format!("bls12381_g1_bls eval error: {e}")))?;
    if !vuf_proof.is_empty() {
        return Err(InternalError("proof size should be 0".to_string()));
    }

    let pinkas_pepper = PinkasPepper::from_affine_bytes(&pepper_base)
        .map_err(|_| InternalError("Failed to derive pinkas pepper".to_string()))?;
    let master_pepper = pinkas_pepper.to_master_pepper();
    let derived_pepper = ExtendedPepper::from_seed(master_pepper.to_bytes())
        .map_err(|e| InternalError(e.to_string()))?
        .derive(&checked_derivation_path)
        .map_err(|e| InternalError(e.to_string()))?
        .get_pepper();

    let idc = IdCommitment::new_from_preimage(
        &derived_pepper,
        &input.aud,
        &input.uid_key,
        &input.uid_val,
    )
    .map_err(|e| InternalError(e.to_string()))?;
    let public_key = KeylessPublicKey {
        iss_val: input.iss,
        idc,
    };
    let address =
        AuthenticationKey::any_key(AnyPublicKey::keyless(public_key.clone())).account_address();

    if encrypts_pepper {
        let mut main_rng: rand::prelude::ThreadRng = thread_rng();
        let mut aead_rng = aes_gcm::aead::OsRng;
        let pepper_base_encrypted = ElGamalCurve25519Aes256Gcm::enc(
            &mut main_rng,
            &mut aead_rng,
            &curve25519_pk_point,
            &pepper_base,
        )
        .map_err(|e| InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}")))?;
        let pepper_encrypted = ElGamalCurve25519Aes256Gcm::enc(
            &mut main_rng,
            &mut aead_rng,
            &curve25519_pk_point,
            derived_pepper.to_bytes(),
        )
        .map_err(|e| InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}")))?;
        Ok((pepper_base_encrypted, pepper_encrypted, address))
    } else {
        Ok((pepper_base, derived_pepper.to_bytes().to_vec(), address))
    }
}
