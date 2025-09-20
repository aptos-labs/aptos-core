// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accounts::{
        account_managers::ACCOUNT_MANAGERS, account_recovery_db::AccountRecoveryDBInterface,
    },
    error::PepperServiceError,
    external_resources::{jwk_fetcher, jwk_fetcher::JWKCache, resource_fetcher::CachedResources},
};
use aptos_keyless_pepper_common::{
    jwt::Claims,
    vuf::{
        self,
        bls12381_g1_bls::PinkasPepper,
        slip_10,
        slip_10::{DerivationPath, ExtendedPepper},
        VUF,
    },
    PepperInput,
};
use aptos_logger::info;
use aptos_types::{
    account_address::AccountAddress,
    keyless::{Configuration, IdCommitment, KeylessPublicKey, OpenIdSig, Pepper},
    transaction::authenticator::{AnyPublicKey, AuthenticationKey, EphemeralPublicKey},
};
use jsonwebtoken::{Algorithm::RS256, DecodingKey, TokenData, Validation};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

// The default derivation path (if none is provided)
const DEFAULT_DERIVATION_PATH: &str = "m/44'/637'/0'/0'/0'";

// The default uid key (if none is provided)
const DEFAULT_UID_KEY: &str = "sub";

// The sub uid key string
const SUB_UID_KEY: &str = "sub";

// The email uid key string
const EMAIL_UID_KEY: &str = "email";

/// Handles the given pepper request, returning the pepper base, pepper and account address
pub async fn handle_pepper_request(
    vuf_private_key: &ark_bls12_381::Fr,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
    jwt: String,
    ephemeral_public_key: EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    uid_key: Option<String>,
    derivation_path: Option<String>,
    aud: Option<String>,
    account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync>,
) -> Result<(Vec<u8>, Vec<u8>, AccountAddress), PepperServiceError> {
    // Get the on-chain keyless configuration
    let keyless_configuration = get_on_chain_keyless_configuration(cached_resources)?;

    // Parse the JWT (without signature verification)
    let claims = aptos_keyless_pepper_common::jwt::parse(jwt.as_str())
        .map_err(|e| PepperServiceError::BadRequest(format!("JWT decoding error: {e}")))?;

    // Verify the public key expiry date
    verify_public_key_expiry_date_secs(exp_date_secs, &claims, &keyless_configuration)?;

    // Verify the oauth nonce
    verify_oath_nonce(
        &ephemeral_public_key,
        exp_date_secs,
        epk_blinder,
        &claims,
        &keyless_configuration,
    )?;

    // Verify the JWT signature
    verify_jwt_signature(jwk_cache, &jwt, &claims).await?;

    // Get the uid key and value
    let (uid_key, uid_val) = get_uid_key_and_value(uid_key, &claims)?;

    // Create the pepper input
    let pepper_input =
        create_pepper_input(aud, &claims, uid_key, uid_val, account_recovery_db).await?;

    // Create the pepper base using the vuf private key and the pepper input
    let pepper_base = create_pepper_base(vuf_private_key, &pepper_input)?;

    // Derive the pepper using the verified derivation path and the pepper base
    let verified_derivation_path = get_verified_derivation_path(derivation_path)?;
    let derived_pepper = derive_pepper(&verified_derivation_path, &pepper_base)?;
    let derived_pepper_bytes = derived_pepper.to_bytes().to_vec();

    // Create the account address
    let address = create_account_address(&pepper_input, &derived_pepper)?;

    // Return the pepper base, derived pepper and address
    Ok((pepper_base, derived_pepper_bytes, address))
}

/// Creates the account address using the pepper input and the derived pepper
fn create_account_address(
    pepper_input: &PepperInput,
    derived_pepper: &Pepper,
) -> Result<AccountAddress, PepperServiceError> {
    let id_commitment = IdCommitment::new_from_preimage(
        derived_pepper,
        &pepper_input.aud,
        &pepper_input.uid_key,
        &pepper_input.uid_val,
    )
    .map_err(|error| {
        PepperServiceError::InternalError(format!("Failed to create id commitment: {}", error))
    })?;
    let public_key = KeylessPublicKey {
        iss_val: pepper_input.iss.clone(),
        idc: id_commitment,
    };
    let address = AuthenticationKey::any_key(AnyPublicKey::keyless(public_key)).account_address();

    Ok(address)
}

/// Creates the pepper base using the VUF private key and the pepper input
fn create_pepper_base(
    vuf_private_key: &ark_bls12_381::Fr,
    pepper_input: &PepperInput,
) -> Result<Vec<u8>, PepperServiceError> {
    // Serialize the pepper input using BCS
    let input_bytes = bcs::to_bytes(&pepper_input).map_err(|error| {
        PepperServiceError::InternalError(format!(
            "Failed to serialize pepper input! Error: {:?}",
            error
        ))
    })?;

    // Generate the pepper base and proof using the VUF
    let (pepper_base, vuf_proof) =
        vuf::bls12381_g1_bls::Bls12381G1Bls::eval(vuf_private_key, &input_bytes).map_err(
            |error| {
                PepperServiceError::InternalError(format!(
                    "Failed to evaluate bls12381_g1_bls VUF: {}",
                    error
                ))
            },
        )?;

    // Verify that the proof is empty
    if !vuf_proof.is_empty() {
        return Err(PepperServiceError::InternalError(
            "The VUF proof is not empty! This shouldn't happen.".to_string(),
        ));
    }

    Ok(pepper_base)
}

/// Creates the pepper input, and updates the account recovery DB
async fn create_pepper_input(
    aud: Option<String>,
    claims: &TokenData<Claims>,
    uid_key: String,
    uid_val: String,
    account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync>,
) -> Result<PepperInput, PepperServiceError> {
    // Get the aud from the claims. Note: if the request is from an account manager,
    // and a target aud is specified, we will override the aud and  generate the
    // pepper input with the overridden aud. This is useful for pepper recovery.
    let claims_aud = claims.claims.aud.clone();
    let (aud, aud_overridden) =
        if ACCOUNT_MANAGERS.contains(&(claims.claims.iss.clone(), claims.claims.aud.clone())) {
            match aud {
                Some(overridden_aud) => (overridden_aud, true),
                None => (claims_aud, false),
            }
        } else {
            (claims_aud, false)
        };

    // Create the pepper input
    let pepper_input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key,
        uid_val,
        aud: aud.clone(),
    };
    info!("Successfully created PepperInput: {:?}", &pepper_input);

    // Update the account recovery DB (unless the aud was overridden)
    if aud_overridden {
        info!("The aud was overridden ({}) for the pepper input. Skipping account recovery DB update!", aud);
    } else {
        account_recovery_db
            .update_db_with_pepper_input(&pepper_input)
            .await?;
    }

    Ok(pepper_input)
}

/// Derives the pepper using the verified derivation path and the pepper base
fn derive_pepper(
    verified_derivation_path: &DerivationPath,
    pepper_base: &[u8],
) -> Result<Pepper, PepperServiceError> {
    let pinkas_pepper = PinkasPepper::from_affine_bytes(pepper_base).map_err(|error| {
        PepperServiceError::InternalError(format!("Failed to create pinkas pepper: {}", error))
    })?;
    let derived_pepper = ExtendedPepper::from_seed(pinkas_pepper.to_master_pepper().to_bytes())
        .map_err(|error| PepperServiceError::InternalError(error.to_string()))?
        .derive(&verified_derivation_path)
        .map_err(|error| PepperServiceError::InternalError(error.to_string()))?
        .get_pepper();

    Ok(derived_pepper)
}

/// Returns the on-chain keyless configuration from the cached resources
fn get_on_chain_keyless_configuration(
    cached_resources: CachedResources,
) -> Result<Configuration, PepperServiceError> {
    let on_chain_keyless_configuration = cached_resources
        .read_on_chain_keyless_configuration()
        .ok_or_else(|| {
            PepperServiceError::InternalError(
                "Failed to read on-chain keyless configuration".to_string(),
            )
        })?;
    on_chain_keyless_configuration
        .get_keyless_configuration()
        .map_err(|error| {
            PepperServiceError::InternalError(format!(
                "Failed to get keyless configuration: {error}"
            ))
        })
}

/// Returns the uid key and value from the claims
fn get_uid_key_and_value(
    uid_key: Option<String>,
    claims: &TokenData<Claims>,
) -> Result<(String, String), PepperServiceError> {
    // If `uid_key` is missing, use `sub` as the default
    let uid_key = match uid_key {
        Some(uid_key) => uid_key,
        None => {
            return Ok((DEFAULT_UID_KEY.into(), claims.claims.sub.clone()));
        },
    };

    // If the uid_key is "sub", return the sub claim value
    if uid_key == SUB_UID_KEY {
        return Ok((uid_key, claims.claims.sub.clone()));
    }

    // Otherwise, check if the uid_key is an email
    if uid_key == EMAIL_UID_KEY {
        let uid_value = claims.claims.email.clone().ok_or_else(|| {
            PepperServiceError::BadRequest(format!(
                "The {} uid_key was specified, but the email claim was not found in the JWT",
                EMAIL_UID_KEY
            ))
        })?;
        return Ok((uid_key, uid_value));
    }

    // Otherwise, an unsupported uid_key was specified
    Err(PepperServiceError::BadRequest(format!(
        "Unsupported uid key provided: {}",
        uid_key
    )))
}

/// Returns the verified derivation path
fn get_verified_derivation_path(
    derivation_path: Option<String>,
) -> Result<DerivationPath, PepperServiceError> {
    // Get the derivation path or use the default
    let derivation_path = derivation_path.unwrap_or(DEFAULT_DERIVATION_PATH.into());

    // Verify the derivation path
    slip_10::get_aptos_derivation_path(&derivation_path).map_err(|error| {
        PepperServiceError::BadRequest(format!("Invalid derivation path: {}", error))
    })
}

/// Verifies the JWT signature using the cached JWKs, or fetches the federated JWK if not found in cache
async fn verify_jwt_signature(
    jwk_cache: JWKCache,
    jwt: &str,
    claims: &TokenData<Claims>,
) -> Result<(), PepperServiceError> {
    // Get the key ID from the JWT header
    let key_id = claims
        .header
        .kid
        .clone()
        .ok_or_else(|| PepperServiceError::BadRequest("Missing kid in JWT".to_string()))?;

    // Get the JWK from the cache, or fetch the federated JWK if not found
    let rsa_jwk = match jwk_fetcher::get_cached_jwk_as_rsa(&claims.claims.iss, &key_id, jwk_cache) {
        Ok(rsa_jwk) => rsa_jwk,
        Err(error) => {
            info!("Failed to get cached JWK for issuer {} and key ID {}: {}. Attempting to fetch federated JWK.", &claims.claims.iss, &key_id, error);
            jwk_fetcher::get_federated_jwk(jwt).await.map_err(|error| {
                PepperServiceError::BadRequest(format!(
                    "Failed to fetch federated JWK: {}. JWT: {}",
                    error, jwt
                ))
            })?
        },
    };
    let jwk_decoding_key =
        DecodingKey::from_rsa_components(&rsa_jwk.n, &rsa_jwk.e).map_err(|error| {
            PepperServiceError::BadRequest(format!(
                "Failed to create decoding key from JWK: {}",
                error
            ))
        })?;

    // Validate the JWT signature.
    // TODO: can we avoid decoding the JWT twice?
    let mut validation_with_sig_verification = Validation::new(RS256);
    validation_with_sig_verification.validate_exp = false; // Don't validate the exp time
    jsonwebtoken::decode::<Claims>(jwt, &jwk_decoding_key, &validation_with_sig_verification) // Signature verification happens here
        .map_err(|e| {
            PepperServiceError::BadRequest(format!("JWT signature verification failed: {e}"))
        })?;

    Ok(())
}

/// Verifies that the oauth nonce in the claims matches the recalculated nonce
fn verify_oath_nonce(
    ephemeral_public_key: &EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    claims: &TokenData<Claims>,
    keyless_configuration: &Configuration,
) -> Result<(), PepperServiceError> {
    let recalculated_nonce = OpenIdSig::reconstruct_oauth_nonce(
        epk_blinder.as_slice(),
        exp_date_secs,
        ephemeral_public_key,
        keyless_configuration,
    )
    .map_err(|error| {
        PepperServiceError::BadRequest(format!("Failed to reconstruct oauth nonce: {}", error))
    })?;

    if claims.claims.nonce != recalculated_nonce {
        Err(PepperServiceError::BadRequest(format!(
            "The oauth nonce in the JWT does not match the recalculated nonce: jwt_nonce = {}, \
            recalculated_nonce = {}",
            claims.claims.nonce, recalculated_nonce
        )))
    } else {
        Ok(())
    }
}

/// Verifies that the given public key expiry date is in the future, and that it
/// is within the allowed horizon from the issued-at time in the claims.
fn verify_public_key_expiry_date_secs(
    exp_date_secs: u64,
    claims: &TokenData<Claims>,
    keyless_configuration: &Configuration,
) -> Result<(), PepperServiceError> {
    // Get the current time
    let time_now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Verify that the expiry date is in the future
    if exp_date_secs <= time_now_secs {
        return Err(PepperServiceError::BadRequest(format!(
            "The ephemeral public key expiry date has passed: exp_date_secs = {}, time_now_secs = {}",
            exp_date_secs, time_now_secs
        )));
    }

    // Get the maximum allowed expiry date
    let (max_exp_date_secs, overflowed) = claims
        .claims
        .iat
        .overflowing_add(keyless_configuration.max_exp_horizon_secs);
    if overflowed {
        return Err(PepperServiceError::BadRequest(
            "The maximum allowed expiry date overflowed".to_string(),
        ));
    }

    // Verify that the expiry date is within the allowed horizon
    if exp_date_secs >= max_exp_date_secs {
        Err(PepperServiceError::BadRequest(
            "The ephemeral public key expiry date is too far in the future (and beyond the max allowed horizon)".into()
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_uid_key_and_value() {
        // Create test token data
        let claims = TokenData {
            claims: Claims {
                iss: "test_issuer".into(),
                sub: "test_sub".into(),
                aud: "test_aud".into(),
                exp: 0,
                iat: 0,
                nonce: "test_nonce".into(),
                email: Some("test_email".into()),
                azp: None,
            },
            header: Default::default(),
        };

        // Test with no uid_key (should use the default)
        let (uid_key, uid_val) = get_uid_key_and_value(None, &claims).unwrap();
        assert_eq!(uid_key, "sub");
        assert_eq!(uid_val, "test_sub");

        // Test with "sub" uid_key
        let (uid_key, uid_val) = get_uid_key_and_value(Some("sub".into()), &claims).unwrap();
        assert_eq!(uid_key, "sub");
        assert_eq!(uid_val, "test_sub");

        // Test with "email" uid_key
        let (uid_key, uid_val) = get_uid_key_and_value(Some("email".into()), &claims).unwrap();
        assert_eq!(uid_key, "email");
        assert_eq!(uid_val, "test_email");

        // Test with an unsupported uid_key
        let result = get_uid_key_and_value(Some("unsupported".to_string()), &claims);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_derivation_path() {
        let invalid_path = Some("m/44'/637'/0'/0/0'".to_string()); // Invalid because one index is not hardened
        let result = get_verified_derivation_path(invalid_path);
        assert!(result.is_err());
    }
}
