// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_db::{init_account_db, ACCOUNT_RECOVERY_DB},
    account_managers::ACCOUNT_MANAGERS,
    error::PepperServiceError,
    external_resources::{jwk_fetcher, jwk_fetcher::JWKCache},
};
use aptos_crypto::asymmetric_encryption::{
    elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm, AsymmetricEncryption,
};
use aptos_infallible::duration_since_epoch;
use aptos_keyless_pepper_common::{
    account_recovery_db::AccountRecoveryDbEntry,
    jwt::Claims,
    vuf::{
        self,
        bls12381_g1_bls::PinkasPepper,
        slip_10::{get_aptos_derivation_path, ExtendedPepper},
        VUF,
    },
    PepperInput,
};
use aptos_logger::{info, warn};
use aptos_types::{
    account_address::AccountAddress,
    keyless::{Configuration, IdCommitment, KeylessPublicKey, OpenIdSig},
    transaction::authenticator::{AnyPublicKey, AuthenticationKey, EphemeralPublicKey},
};
use firestore::{paths, struct_path::path};
use jsonwebtoken::{Algorithm::RS256, DecodingKey, Validation};
use rand::thread_rng;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/637'/0'/0'/0'";

pub async fn process_common(
    vuf_private_key: &ark_bls12_381::Fr,
    jwk_cache: JWKCache,
    session_id: &Uuid,
    jwt: String,
    epk: EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    uid_key: Option<String>,
    derivation_path: Option<String>,
    encrypts_pepper: bool,
    aud: Option<String>,
    should_update_account_recovery_db: bool,
) -> Result<(Vec<u8>, Vec<u8>, AccountAddress), PepperServiceError> {
    let config = Configuration::new_for_devnet();

    let derivation_path = if let Some(path) = derivation_path {
        path
    } else {
        DEFAULT_DERIVATION_PATH.to_owned()
    };
    let checked_derivation_path = get_aptos_derivation_path(&derivation_path)
        .map_err(|e| PepperServiceError::BadRequest(e.to_string()))?;

    let curve25519_pk_point = match &epk {
        EphemeralPublicKey::Ed25519 { public_key } => public_key
            .to_compressed_edwards_y()
            .decompress()
            .ok_or_else(|| {
                PepperServiceError::BadRequest("the pk point is off-curve".to_string())
            })?,
        _ => {
            return Err(PepperServiceError::BadRequest(
                "Only Ed25519 epk is supported".to_string(),
            ));
        },
    };

    let claims = aptos_keyless_pepper_common::jwt::parse(jwt.as_str())
        .map_err(|e| PepperServiceError::BadRequest(format!("JWT decoding error: {e}")))?;
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if exp_date_secs <= now_secs {
        return Err(PepperServiceError::BadRequest("epk expired".to_string()));
    }

    let (max_exp_data_secs, overflowed) = claims
        .claims
        .iat
        .overflowing_add(config.max_exp_horizon_secs);
    if overflowed {
        return Err(PepperServiceError::BadRequest(
            "max_exp_data_secs overflowed".to_string(),
        ));
    }
    if exp_date_secs >= max_exp_data_secs {
        return Err(PepperServiceError::BadRequest(
            "epk expiry date too far".to_string(),
        ));
    }

    let actual_uid_key = if let Some(uid_key) = uid_key.as_ref() {
        uid_key
    } else {
        "sub"
    };

    let uid_val = if actual_uid_key == "email" {
        claims.claims.email.clone().ok_or_else(|| {
            PepperServiceError::BadRequest("`email` required but not found in jwt".to_string())
        })?
    } else if actual_uid_key == "sub" {
        claims.claims.sub.clone()
    } else {
        return Err(PepperServiceError::BadRequest(format!(
            "unsupported uid key: {}",
            actual_uid_key
        )));
    };

    let recalculated_nonce =
        OpenIdSig::reconstruct_oauth_nonce(epk_blinder.as_slice(), exp_date_secs, &epk, &config)
            .map_err(|e| {
                PepperServiceError::BadRequest(format!("nonce reconstruction error: {e}"))
            })?;

    if claims.claims.nonce != recalculated_nonce {
        return Err(PepperServiceError::BadRequest(
            "with nonce mismatch".to_string(),
        ));
    }

    let key_id = claims
        .header
        .kid
        .ok_or_else(|| PepperServiceError::BadRequest("missing kid in JWT".to_string()))?;

    let cached_key = jwk_fetcher::get_cached_jwk_as_rsa(&claims.claims.iss, &key_id, jwk_cache);

    let jwk = match cached_key {
        Ok(key) => key,
        Err(_) => jwk_fetcher::get_federated_jwk(&jwt)
            .await
            .map_err(|e| PepperServiceError::BadRequest(format!("JWK not found: {e}")))?,
    };
    let jwk_decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|e| PepperServiceError::BadRequest(format!("JWK not found: {e}")))?;

    let mut validation_with_sig_verification = Validation::new(RS256);
    validation_with_sig_verification.validate_exp = false; // Don't validate the exp time
    let _claims = jsonwebtoken::decode::<Claims>(
        jwt.as_str(),
        &jwk_decoding_key,
        &validation_with_sig_verification,
    ) // Signature verification happens here.
    .map_err(|e| {
        PepperServiceError::BadRequest(format!("JWT signature verification failed: {e}"))
    })?;

    // If the pepper request is is from an account manager, and has a target aud specified, compute the pepper for the target aud.
    let mut aud_overridden = false;
    let mut final_aud = claims.claims.aud.clone();
    if ACCOUNT_MANAGERS.contains(&(claims.claims.iss.clone(), claims.claims.aud.clone())) {
        if let Some(aud) = aud {
            final_aud = aud;
            aud_overridden = true;
        }
    };

    let input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_string(),
        uid_val,
        aud: final_aud,
    };

    if !aud_overridden {
        info!(
            session_id = session_id,
            iss = input.iss.clone(),
            aud = input.aud.clone(),
            uid_val = input.uid_val.clone(),
            uid_key = input.uid_key.clone(),
            "PepperInput is available."
        );
        if should_update_account_recovery_db {
            update_account_recovery_db(&input).await?;
        }
    }

    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper_base, vuf_proof) =
        vuf::bls12381_g1_bls::Bls12381G1Bls::eval(vuf_private_key, &input_bytes).map_err(|e| {
            PepperServiceError::InternalError(format!("bls12381_g1_bls eval error: {e}"))
        })?;
    if !vuf_proof.is_empty() {
        return Err(PepperServiceError::InternalError(
            "proof size should be 0".to_string(),
        ));
    }

    let pinkas_pepper = PinkasPepper::from_affine_bytes(&pepper_base).map_err(|_| {
        PepperServiceError::InternalError("Failed to derive pinkas pepper".to_string())
    })?;
    let master_pepper = pinkas_pepper.to_master_pepper();
    let derived_pepper = ExtendedPepper::from_seed(master_pepper.to_bytes())
        .map_err(|e| PepperServiceError::InternalError(e.to_string()))?
        .derive(&checked_derivation_path)
        .map_err(|e| PepperServiceError::InternalError(e.to_string()))?
        .get_pepper();

    let idc = IdCommitment::new_from_preimage(
        &derived_pepper,
        &input.aud,
        &input.uid_key,
        &input.uid_val,
    )
    .map_err(|e| PepperServiceError::InternalError(e.to_string()))?;
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
        .map_err(|e| {
            PepperServiceError::InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}"))
        })?;
        let pepper_encrypted = ElGamalCurve25519Aes256Gcm::enc(
            &mut main_rng,
            &mut aead_rng,
            &curve25519_pk_point,
            derived_pepper.to_bytes(),
        )
        .map_err(|e| {
            PepperServiceError::InternalError(format!("ElGamalCurve25519Aes256Gcm enc error: {e}"))
        })?;
        Ok((pepper_base_encrypted, pepper_encrypted, address))
    } else {
        Ok((pepper_base, derived_pepper.to_bytes().to_vec(), address))
    }
}

/// Save a pepper request into the account recovery DB.
///
/// TODO: once the account recovery DB flow is verified working e2e, DB error should not be ignored.
async fn update_account_recovery_db(input: &PepperInput) -> Result<(), PepperServiceError> {
    match ACCOUNT_RECOVERY_DB.get_or_init(init_account_db).await {
        Ok(db) => {
            let entry = AccountRecoveryDbEntry {
                iss: input.iss.clone(),
                aud: input.aud.clone(),
                uid_key: input.uid_key.clone(),
                uid_val: input.uid_val.clone(),
                first_request_unix_ms_minus_1q: None,
                last_request_unix_ms: None,
                num_requests: None,
            };
            let doc_id = entry.document_id();
            let now_unix_ms = duration_since_epoch().as_millis() as i64;

            // The update transactions use the following strategy.
            // 1. If not exists, create the document for the user identifier `(iss, aud, uid_key, uid_val)`.
            //    but leave counter/time fields unspecified.
            // 2. `num_requests += 1`, assuming the default value is 0.
            // 3. `last_request_unix_ms = max(last_request_unix_ms, now)`, assuming the default value is 0.
            // 4. `first_request_unix_ms = min(first_request_unix_ms, now)`, assuming the default value is +inf.
            //
            // This strategy is preferred because all the operations can be made server-side,
            // which means the txn should require only 1 RTT,
            // better than using read-compute-write pattern that requires 2 RTTs.
            //
            // This strategy does not work directly:
            // in firestore, the default value of a number field is 0, and we do not know a way to customize it for op 4.
            // The workaround here is apply an offset so 0 becomes a legitimate default value.
            // So we work with `first_request_unix_ms_minus_1q` instead,
            // which is defined as `first_request_unix_ms - 1_000_000_000_000_000`,
            // where 1_000_000_000_000_000 milliseconds is roughly 31710 years.

            let mut txn = db.begin_transaction().await.map_err(|e| {
                PepperServiceError::InternalError(format!("begin_transaction error: {e}"))
            })?;
            db.fluent()
                .update()
                .fields(paths!(AccountRecoveryDbEntry::{iss, aud, uid_key, uid_val}))
                .in_col("accounts")
                .document_id(&doc_id)
                .object(&entry) // op 1
                .transforms(|builder| {
                    builder.fields([
                        builder
                            .field(path!(AccountRecoveryDbEntry::num_requests))
                            .increment(1), // op 2
                        builder
                            .field(path!(AccountRecoveryDbEntry::last_request_unix_ms))
                            .maximum(now_unix_ms), // op 3
                        builder
                            .field(path!(
                                AccountRecoveryDbEntry::first_request_unix_ms_minus_1q
                            ))
                            .minimum(now_unix_ms - 1_000_000_000_000_000), // op 4
                    ])
                })
                .add_to_transaction(&mut txn)
                .map_err(|e| {
                    PepperServiceError::InternalError(format!("add_to_transaction error: {e}"))
                })?;
            let txn_result = txn.commit().await;

            if let Err(e) = txn_result {
                warn!("ACCOUNT_RECOVERY_DB operation failed: {e}");
            }
            Ok(())
        },
        Err(e) => {
            warn!("ACCOUNT_RECOVERY_DB client failed to init: {e}");
            Ok(())
        },
    }
}
