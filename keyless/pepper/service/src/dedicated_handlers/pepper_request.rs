// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accounts::{
        account_managers::AccountRecoveryManagers, account_recovery_db::AccountRecoveryDBInterface,
    },
    error::PepperServiceError,
    external_resources::{jwk_fetcher, jwk_fetcher::JWKCache, resource_fetcher::CachedResources},
    metrics,
    vuf_keypair::VUFKeypair,
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
    time::{Instant, SystemTime, UNIX_EPOCH},
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
    vuf_keypair: Arc<VUFKeypair>,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
    jwt: String,
    ephemeral_public_key: EphemeralPublicKey,
    exp_date_secs: u64,
    epk_blinder: Vec<u8>,
    uid_key: Option<String>,
    derivation_path: Option<String>,
    account_recovery_managers: Arc<AccountRecoveryManagers>,
    aud_override: Option<String>,
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
    let pepper_input = create_pepper_input(
        &claims,
        uid_key,
        uid_val,
        account_recovery_managers,
        aud_override,
        account_recovery_db,
    )
    .await?;

    // Derive the pepper base, pepper bytes and account address.
    // Note: we do this using spawn_blocking() to avoid blocking the async runtime.
    let (pepper_base, derived_pepper_bytes, address) = tokio::task::spawn_blocking(move || {
        // Start the derivation timer
        let derivation_start_time = Instant::now();

        // Derive the pepper and account address
        let derivation_result =
            derive_pepper_and_account_address(vuf_keypair, derivation_path, &pepper_input);

        // Update the derivation metrics
        metrics::update_pepper_derivation_metrics(derivation_result.is_ok(), derivation_start_time);

        derivation_result
    })
    .await??;

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
    vuf_keypair: Arc<VUFKeypair>,
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
        vuf::bls12381_g1_bls::Bls12381G1Bls::eval(vuf_keypair.vuf_private_key(), &input_bytes)
            .map_err(|error| {
                PepperServiceError::InternalError(format!(
                    "Failed to evaluate bls12381_g1_bls VUF: {}",
                    error
                ))
            })?;

    // Verify that the proof is empty
    if !vuf_proof.is_empty() {
        return Err(PepperServiceError::InternalError(
            "The VUF proof is not empty! This shouldn't happen.".to_string(),
        ));
    }

    // Verify the pepper base output (this ensures we only ever return valid outputs,
    // and protects against various security issues, e.g., fault based side channels).
    vuf::bls12381_g1_bls::Bls12381G1Bls::verify(
        vuf_keypair.vuf_public_key(),
        &input_bytes,
        &pepper_base,
        &vuf_proof,
    )
    .map_err(|error| {
        PepperServiceError::InternalError(format!("VUF verification failed: {}", error))
    })?;

    Ok(pepper_base)
}

/// Creates the pepper input, and updates the account recovery DB
async fn create_pepper_input(
    claims: &TokenData<Claims>,
    uid_key: String,
    uid_val: String,
    account_recovery_managers: Arc<AccountRecoveryManagers>,
    aud_override: Option<String>,
    account_recovery_db: Arc<dyn AccountRecoveryDBInterface + Send + Sync>,
) -> Result<PepperInput, PepperServiceError> {
    let iss = claims.claims.iss.clone();
    let claims_aud = claims.claims.aud.clone();

    // Get the aud for the pepper input. Note: if the request is from an account
    // recovery manager, we will override the aud and generate the pepper input
    // with the overridden aud. This is useful for pepper recovery.
    let aud = if account_recovery_managers.contains(&iss, &claims_aud) {
        match aud_override {
            Some(aud_override) => aud_override, // Use the overridden aud
            None => {
                return Err(PepperServiceError::UnexpectedError(format!(
                    "The issuer {} and aud {} correspond to an account recovery manager, but no aud override was provided!",
                    &iss, &claims_aud
                )));
            },
        }
    } else if let Some(aud_override) = aud_override {
        return Err(PepperServiceError::UnexpectedError(format!(
            "The issuer {} and aud {} do not correspond to an account recovery manager, but an aud override was provided: {}!",
            &iss, &claims_aud, &aud_override
        )));
    } else {
        claims_aud // Use the aud directly from the claims
    };

    // Create the pepper input
    let pepper_input = PepperInput {
        iss,
        uid_key,
        uid_val,
        aud,
    };
    info!("Successfully created PepperInput: {:?}", &pepper_input);

    // Update the account recovery DB
    account_recovery_db
        .update_db_with_pepper_input(&pepper_input)
        .await?;

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

/// Derives the pepper base, pepper bytes and account address
pub fn derive_pepper_and_account_address(
    vuf_keypair: Arc<VUFKeypair>,
    derivation_path: Option<String>,
    pepper_input: &PepperInput,
) -> Result<(Vec<u8>, Vec<u8>, AccountAddress), PepperServiceError> {
    // Create the pepper base using the vuf private key and the pepper input
    let pepper_base = create_pepper_base(vuf_keypair, pepper_input)?;

    // Derive the pepper using the verified derivation path and the pepper base
    let verified_derivation_path = get_verified_derivation_path(derivation_path)?;
    let derived_pepper = derive_pepper(&verified_derivation_path, &pepper_base)?;
    let derived_pepper_bytes = derived_pepper.to_bytes().to_vec();

    // Create the account address
    let address = create_account_address(pepper_input, &derived_pepper)?;

    Ok((pepper_base, derived_pepper_bytes, address))
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
    use crate::{accounts::account_managers::AccountRecoveryManager, tests::utils};

    // Test token data constants
    const TEST_TOKEN_ISSUER: &str = "token_issuer";
    const TEST_TOKEN_SUB: &str = "token_sub";
    const TEST_TOKEN_AUD: &str = "token_aud";
    const TEST_TOKEN_AUD_OVERRIDE: &str = "token_aud_override";
    const TEST_TOKEN_EMAIL: &str = "token_email";
    const TEST_TOKEN_NONCE: &str = "token_nonce";

    // Hard-coded test pepper constants. These are used to sanity check the
    // generation logic for the pepper base, derived pepper and account address.
    // Note:
    // - These were generated using the tests in this file, and verified across
    //   several releases (to ensure backward compatibility).
    // - The "sub" variants are for when the "sub" uid key is used.
    // - The "email" variants are for when the "email" uid key is used.
    // - The "sub_override" variants are for when the "sub" uid key is used with an
    //   account manager that has an aud override.
    const TEST_EMAIL_ACCOUNT_ADDRESS: &str =
        "0x526bcbdbdb4641f1b2f7bbc865c8eb41e976285a9849844b4f543ea17cff3dd3";
    const TEST_EMAIL_DERIVED_PEPPER_HEX: &str =
        "9c76d2f26e8717ffbd499caad7ef5d85fdf24ce493ed111c8313f34429d10d";
    const TEST_EMAIL_PEPPER_BASE_HEX: &str = "96ef6f0fa8534a24b917dd773f14fa75a934e3bd21480391b49699c6b14735915de0adbdacc2670be61f06cb7215a57c";
    const TEST_EMAIL_VUF_PRIVATE_KEY_SEED: [u8; 32] = [1; 32];

    const TEST_SUB_ACCOUNT_ADDRESS: &str =
        "0xafdfc88e30c9858d34b2b5f63854cb4c2740c03a16cbc4df3f4adbe7b2e6c63f";
    const TEST_SUB_DERIVED_PEPPER_HEX: &str =
        "42803a2ec0739390232b81a20a78651210e6e185d70edc4fa5bddbc2416b24";
    const TEST_SUB_PEPPER_BASE_HEX: &str = "b6d25395110bad7fda25f36bd44ee8220f37e952718bbb0e92cfae1061b6bb982bf34779d936f1b3cc412b832cf76d83";
    const TEST_SUB_VUF_PRIVATE_KEY_SEED: [u8; 32] = [2; 32];

    const TEST_SUB_OVERRIDE_ACCOUNT_ADDRESS: &str =
        "0xf357a030bf7da7f4c28a55dfda8bc1725339576acb2623b526e8d3eb2f366b79";
    const TEST_SUB_OVERRIDE_DERIVED_PEPPER_HEX: &str =
        "21a8d5768336a41a5872351c806d56cca994b0acbe7f054afeed22bee06ef2";
    const TEST_SUB_OVERRIDE_PEPPER_BASE_HEX: &str = "b14111748bc9bde79f0a7edea91b06432800107c448dbe9cc89b47a49ec5094e2fe8976790f6574c7eb9abdc7c5b1df5";

    #[test]
    fn test_create_pepper_base() {
        // Create a test pepper input
        let pepper_input = PepperInput {
            iss: TEST_TOKEN_ISSUER.into(),
            uid_key: SUB_UID_KEY.into(),
            uid_val: TEST_TOKEN_SUB.into(),
            aud: TEST_TOKEN_AUD.into(),
        };

        // Create a test VUF keypair
        let vuf_keypair = utils::create_vuf_keypair(Some(TEST_SUB_VUF_PRIVATE_KEY_SEED));

        // Create the pepper base
        let pepper_base = create_pepper_base(vuf_keypair.clone(), &pepper_input).unwrap();

        // Verify the pepper base matches the expected value
        assert_eq!(
            hex::encode(pepper_base),
            TEST_SUB_PEPPER_BASE_HEX.to_string()
        );

        // Create an invalid keypair where the public and private keys do not match
        let invalid_private_key = *utils::create_vuf_keypair(None).vuf_private_key();
        let invalid_vuf_keypair = VUFKeypair::new(
            invalid_private_key,
            *vuf_keypair.vuf_public_key(),
            vuf_keypair.vuf_public_key_json().clone(),
        );

        // Create the pepper base using the invalid keypair. This should fail with
        // a verification error, because the public and private keys don't match.
        let pepper_service_error =
            create_pepper_base(Arc::new(invalid_vuf_keypair), &pepper_input).unwrap_err();
        assert!(pepper_service_error
            .to_string()
            .contains("VUF verification failed"));
    }

    #[test]
    fn test_get_uid_key_and_value() {
        // Create test token data
        let claims = create_test_token_data();

        // Test with no uid_key (should use the default)
        let (uid_key, uid_val) = get_uid_key_and_value(None, &claims).unwrap();
        assert_eq!(uid_key, SUB_UID_KEY);
        assert_eq!(uid_val, TEST_TOKEN_SUB);

        // Test with sub uid_key
        let (uid_key, uid_val) = get_uid_key_and_value(Some(SUB_UID_KEY.into()), &claims).unwrap();
        assert_eq!(uid_key, SUB_UID_KEY);
        assert_eq!(uid_val, TEST_TOKEN_SUB);

        // Test with email uid_key
        let (uid_key, uid_val) =
            get_uid_key_and_value(Some(EMAIL_UID_KEY.into()), &claims).unwrap();
        assert_eq!(uid_key, EMAIL_UID_KEY);
        assert_eq!(uid_val, TEST_TOKEN_EMAIL);

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

    #[tokio::test]
    async fn test_pepper_and_address_derivation_default() {
        // Create test token data
        let claims = create_test_token_data();

        // Get the default uid key and value (which should be "sub")
        let (uid_key, uid_val) = get_uid_key_and_value(None, &claims).unwrap();

        // Create the pepper input
        let accounts_managers_and_overrides = utils::get_empty_account_recovery_managers();
        let account_recovery_db = utils::get_mock_account_recovery_db();
        let pepper_input = create_pepper_input(
            &claims,
            uid_key,
            uid_val,
            accounts_managers_and_overrides,
            None,
            account_recovery_db,
        )
        .await
        .unwrap();

        // Get the VUF keypair
        let vuf_keypair = utils::create_vuf_keypair(Some(TEST_SUB_VUF_PRIVATE_KEY_SEED));

        // Verify the pepper base, derived pepper and account address
        verify_base_pepper_and_address_generation(
            vuf_keypair,
            &pepper_input,
            TEST_SUB_PEPPER_BASE_HEX,
            TEST_SUB_DERIVED_PEPPER_HEX,
            TEST_SUB_ACCOUNT_ADDRESS,
        );
    }

    #[tokio::test]
    async fn test_pepper_and_address_derivation_email() {
        // Create test token data
        let claims = create_test_token_data();

        // Get the email uid key and value
        let (uid_key, uid_val) =
            get_uid_key_and_value(Some(EMAIL_UID_KEY.into()), &claims).unwrap();

        // Create the pepper input
        let accounts_managers_and_overrides = utils::get_empty_account_recovery_managers();
        let account_recovery_db = utils::get_mock_account_recovery_db();
        let pepper_input = create_pepper_input(
            &claims,
            uid_key,
            uid_val,
            accounts_managers_and_overrides,
            None,
            account_recovery_db,
        )
        .await
        .unwrap();

        // Get the VUF keypair
        let vuf_keypair = utils::create_vuf_keypair(Some(TEST_EMAIL_VUF_PRIVATE_KEY_SEED));

        // Verify the pepper base, derived pepper and account address
        verify_base_pepper_and_address_generation(
            vuf_keypair,
            &pepper_input,
            TEST_EMAIL_PEPPER_BASE_HEX,
            TEST_EMAIL_DERIVED_PEPPER_HEX,
            TEST_EMAIL_ACCOUNT_ADDRESS,
        );
    }

    #[tokio::test]
    async fn test_pepper_and_address_derivation_sub() {
        // Create test token data
        let claims = create_test_token_data();

        // Get the sub uid key and value
        let (uid_key, uid_val) = get_uid_key_and_value(Some(SUB_UID_KEY.into()), &claims).unwrap();

        // Create the pepper input
        let accounts_managers_and_overrides = utils::get_empty_account_recovery_managers();
        let account_recovery_db = utils::get_mock_account_recovery_db();
        let pepper_input = create_pepper_input(
            &claims,
            uid_key,
            uid_val,
            accounts_managers_and_overrides,
            None,
            account_recovery_db,
        )
        .await
        .unwrap();

        // Get the VUF keypair
        let vuf_keypair = utils::create_vuf_keypair(Some(TEST_SUB_VUF_PRIVATE_KEY_SEED));

        // Verify the pepper base, derived pepper and account address
        verify_base_pepper_and_address_generation(
            vuf_keypair,
            &pepper_input,
            TEST_SUB_PEPPER_BASE_HEX,
            TEST_SUB_DERIVED_PEPPER_HEX,
            TEST_SUB_ACCOUNT_ADDRESS,
        );
    }

    #[tokio::test]
    async fn test_pepper_and_address_derivation_sub_aud_override() {
        // Create test token data
        let claims = create_test_token_data();

        // Get the sub uid key and value
        let (uid_key, uid_val) = get_uid_key_and_value(Some(SUB_UID_KEY.into()), &claims).unwrap();

        // Create the account recovery managers
        let account_recovery_manager =
            AccountRecoveryManager::new(claims.claims.iss.clone(), claims.claims.aud.clone());
        let account_recovery_managers =
            Arc::new(AccountRecoveryManagers::new(vec![account_recovery_manager]));

        // Create the pepper input with an aud override
        let account_recovery_db = utils::get_mock_account_recovery_db();
        let pepper_input = create_pepper_input(
            &claims,
            uid_key,
            uid_val,
            account_recovery_managers,
            Some(TEST_TOKEN_AUD_OVERRIDE.into()),
            account_recovery_db,
        )
        .await
        .unwrap();

        // Get the VUF keypair
        let vuf_keypair = utils::create_vuf_keypair(Some(TEST_SUB_VUF_PRIVATE_KEY_SEED));

        // Verify the pepper base, derived pepper and account address
        verify_base_pepper_and_address_generation(
            vuf_keypair,
            &pepper_input,
            TEST_SUB_OVERRIDE_PEPPER_BASE_HEX,
            TEST_SUB_OVERRIDE_DERIVED_PEPPER_HEX,
            TEST_SUB_OVERRIDE_ACCOUNT_ADDRESS,
        );
    }

    /// Creates test token data with predefined claims
    fn create_test_token_data() -> TokenData<Claims> {
        TokenData {
            claims: Claims {
                iss: TEST_TOKEN_ISSUER.into(),
                sub: TEST_TOKEN_SUB.into(),
                aud: TEST_TOKEN_AUD.into(),
                exp: 0,
                iat: 0,
                nonce: TEST_TOKEN_NONCE.into(),
                email: Some(TEST_TOKEN_EMAIL.into()),
                azp: None,
            },
            header: Default::default(),
        }
    }

    /// Verifies the generated pepper base, derived pepper and account address against the expected values
    fn verify_base_pepper_and_address_generation(
        vuf_keypair: Arc<VUFKeypair>,
        pepper_input: &PepperInput,
        expected_pepper_base_hex: &str,
        expected_derived_pepper_hex: &str,
        expected_account_address: &str,
    ) {
        // Derive the pepper base, pepper and account address
        let (pepper_base, derived_pepper_bytes, address) =
            derive_pepper_and_account_address(vuf_keypair, None, pepper_input).unwrap();

        // Verify the pepper base, derived pepper and account address
        assert_eq!(hex::encode(pepper_base), expected_pepper_base_hex);
        assert_eq!(
            hex::encode(derived_pepper_bytes),
            expected_derived_pepper_hex
        );
        assert_eq!(address.to_standard_string(), expected_account_address);
    }
}
