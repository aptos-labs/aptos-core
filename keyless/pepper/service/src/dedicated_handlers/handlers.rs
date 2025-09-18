// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dedicated_handlers::process_common::process_common,
    error::PepperServiceError,
    external_resources::{jwk_fetcher, jwk_fetcher::JWKCache, resource_fetcher::CachedResources},
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_keyless_pepper_common::{
    PepperRequest, PepperResponse, SignatureResponse, VerifyRequest, VerifyResponse,
};
use aptos_types::{
    jwks::rsa::RSA_JWK,
    keyless::{
        get_public_inputs_hash, Configuration, EphemeralCertificate, Groth16ProofAndStatement,
        KeylessPublicKey, KeylessSignature, ZeroKnowledgeSig, ZKP,
    },
    transaction::authenticator::{AnyPublicKey, AnySignature, EphemeralPublicKey},
};
use ark_bn254::Fr;
use firestore::async_trait;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

/// A generic handler trait for processing requests and producing responses
#[async_trait]
pub trait HandlerTrait<TRequest, TResponse>: Send + Sync {
    /// Returns the name of the handler (e.g., the type name). This is useful for logging.
    fn get_handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    // TODO: is there a way we can remove the vuf_private_key param here?
    async fn handle_request(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        cached_resources: CachedResources,
        request: TRequest,
    ) -> Result<TResponse, PepperServiceError>;
}

/// A handler for processing pepper fetch requests
pub struct V0FetchHandler;

#[async_trait]
impl HandlerTrait<PepperRequest, PepperResponse> for V0FetchHandler {
    async fn handle_request(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        _cached_resources: CachedResources,
        request: PepperRequest,
    ) -> Result<PepperResponse, PepperServiceError> {
        // Parse the request
        let PepperRequest {
            jwt,
            epk,
            exp_date_secs,
            epk_blinder,
            uid_key,
            derivation_path,
        } = request;

        // Fetch the pepper
        let (_pepper_base, pepper, address) = process_common(
            vuf_private_key,
            jwk_cache,
            &Uuid::new_v4(),
            jwt,
            epk,
            exp_date_secs,
            epk_blinder,
            uid_key,
            derivation_path,
            false,
            None,
            true,
        )
        .await?;

        // Return the pepper response
        Ok(PepperResponse {
            pepper,
            address: address.to_vec(),
        })
    }
}

/// A handler for processing signature requests
pub struct V0SignatureHandler;

#[async_trait]
impl HandlerTrait<PepperRequest, SignatureResponse> for V0SignatureHandler {
    async fn handle_request(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        _cached_resources: CachedResources,
        request: PepperRequest,
    ) -> Result<SignatureResponse, PepperServiceError> {
        // Parse the request
        let PepperRequest {
            jwt,
            epk,
            exp_date_secs,
            epk_blinder,
            uid_key,
            derivation_path,
        } = request;

        // Fetch the pepper base (i.e., VUF signature)
        let (pepper_base, _pepper, _address) = process_common(
            vuf_private_key,
            jwk_cache,
            &Uuid::new_v4(),
            jwt,
            epk,
            exp_date_secs,
            epk_blinder,
            uid_key,
            derivation_path,
            false,
            None,
            false,
        )
        .await?;

        // Return the signature response
        Ok(SignatureResponse {
            signature: pepper_base,
        })
    }
}

/// A handler for processing signature verification requests.
// TODO: see if we should remove this endpoint.
pub struct V0VerifyHandler;

#[async_trait]
impl HandlerTrait<VerifyRequest, VerifyResponse> for V0VerifyHandler {
    async fn handle_request(
        &self,
        _: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        cached_resources: CachedResources,
        request: VerifyRequest,
    ) -> Result<VerifyResponse, PepperServiceError> {
        // Parse the request
        let VerifyRequest {
            public_key,
            signature,
            message,
            address: _, // TODO: what should we do with the address field?
        } = request;

        // Fetch the keyless public key and signature from the request
        let (keyless_public_key, keyless_signature) = match (&public_key, &signature) {
            (AnyPublicKey::Keyless { public_key }, AnySignature::Keyless { signature }) => {
                (public_key, signature)
            },
            (any_public_key, any_signature) => {
                return Err(PepperServiceError::BadRequest(format!(
                    "Unsupported public key or signature types for keyless verify request: {:?}, {:?}",
                    any_public_key, any_signature
                )));
            },
        };

        // Verify the expiry time of the keyless signature
        verify_signature_expiry(keyless_signature)?;

        // Verify the signature over the message
        verify_message_signature(&message, keyless_signature)?;

        // Get the keyless configuration and training wheels public key
        let (keyless_config, training_wheels_pubkey) =
            get_keyless_config_and_training_wheels_pubkey(&cached_resources)?;

        // Get the zero knowledge signature from the certificate
        let zero_knowledge_signature = get_zero_knowledge_signature(keyless_signature)?;

        // Verify the expiration horizon
        if zero_knowledge_signature.exp_horizon_secs > keyless_config.max_exp_horizon_secs {
            return Err(PepperServiceError::BadRequest(format!(
                "The expiration horizon is too long: {} seconds (max allowed: {} seconds)",
                zero_knowledge_signature.exp_horizon_secs, keyless_config.max_exp_horizon_secs
            )));
        }

        // Verify the override aud value
        if let Some(override_aud_val) = &zero_knowledge_signature.override_aud_val {
            keyless_config
                .is_allowed_override_aud(override_aud_val)
                .map_err(|error| {
                    PepperServiceError::BadRequest(format!(
                        "The given override aud value is not allowed: {}. Error: {}",
                        override_aud_val, error
                    ))
                })?;
        }

        // Verify the training wheels signature
        let public_inputs_hash = verify_training_wheels_signature(
            &jwk_cache,
            keyless_public_key,
            keyless_signature,
            &zero_knowledge_signature,
            &keyless_config,
            training_wheels_pubkey,
        )?;

        // Verify the groth16 proof in the zero knowledge signature
        verify_groth16_proof(
            cached_resources,
            zero_knowledge_signature,
            public_inputs_hash,
        )?;

        // All verifications passed
        Ok(VerifyResponse { success: true })
    }
}

/// Retrieves the keyless configuration and training wheels public key
fn get_keyless_config_and_training_wheels_pubkey(
    cached_resources: &CachedResources,
) -> Result<(Configuration, Option<EphemeralPublicKey>), PepperServiceError> {
    // Get the keyless configuration
    let on_chain_keyless_config = cached_resources
        .read_on_chain_keyless_configuration()
        .ok_or_else(|| {
            PepperServiceError::InternalError(
                "On-chain keyless config not cached locally.".to_string(),
            )
        })?;
    let configuration = on_chain_keyless_config
        .get_keyless_configuration()
        .map_err(|error| {
            PepperServiceError::InternalError(format!(
                "Failed to parse on-chain keyless config: {}",
                error
            ))
        })?;

    // Extract the training wheels public key
    let training_wheels_pubkey = match &configuration.training_wheels_pubkey {
        // This takes ~4.4 microseconds, so we are not too concerned about speed here.
        // (Run `cargo bench -- ed25519/pk_deserialize` in `crates/aptos-crypto`.)
        Some(public_key_bytes) => Some(EphemeralPublicKey::ed25519(
            Ed25519PublicKey::try_from(public_key_bytes.as_slice()).map_err(|error| {
                PepperServiceError::InternalError(format!(
                    "Failed to parse on-chain training wheels public key: {}",
                    error
                ))
            })?,
        )),
        None => None,
    };

    Ok((configuration, training_wheels_pubkey))
}

/// Fetches the RSA JWK for the given keyless public key and signature
fn get_rsa_jwk(
    keyless_public_key: &KeylessPublicKey,
    keyless_signature: &KeylessSignature,
    jwk_cache: JWKCache,
) -> Result<Arc<RSA_JWK>, PepperServiceError> {
    let KeylessPublicKey { idc: _, iss_val } = keyless_public_key;
    let jwt_header = keyless_signature.parse_jwt_header().map_err(|error| {
        PepperServiceError::BadRequest(format!(
            "Failed to decode JWT header from signature: {}",
            error
        ))
    })?;
    jwk_fetcher::get_cached_jwk_as_rsa(iss_val, &jwt_header.kid, jwk_cache).map_err(|error| {
        PepperServiceError::BadRequest(format!("Failed to get RSA JWK from cache: {}", error))
    })
}

/// Extracts the zero knowledge signature from the keyless signature
fn get_zero_knowledge_signature(
    keyless_signature: &KeylessSignature,
) -> Result<ZeroKnowledgeSig, PepperServiceError> {
    let KeylessSignature { cert, .. } = keyless_signature;
    match cert {
        EphemeralCertificate::ZeroKnowledgeSig(zero_knowledge_sig) => {
            Ok(zero_knowledge_sig.clone())
        },
        certificate => Err(PepperServiceError::BadRequest(format!(
            "Unsupported certificate type for keyless verify request: {:?}",
            certificate
        ))),
    }
}

/// Verifies the groth16 proof in the zero knowledge signature
fn verify_groth16_proof(
    cached_resources: CachedResources,
    zero_knowledge_sig: ZeroKnowledgeSig,
    public_inputs_hash: Fr,
) -> Result<(), PepperServiceError> {
    // Get the groth16 verification key from cached resources
    let onchain_groth16_vk = cached_resources.read_on_chain_groth16_vk().ok_or_else(|| {
        PepperServiceError::InternalError("Failed to read groth16 VK from cached resources.".into())
    })?;
    let ark_groth16_pvk = onchain_groth16_vk
        .to_ark_prepared_verifying_key()
        .map_err(|error| {
            PepperServiceError::InternalError(format!(
                "Failed to convert on-chain groth16 VK to Ark format: {}",
                error
            ))
        })?;

    // Verify the groth16 proof
    zero_knowledge_sig
        .verify_groth16_proof(public_inputs_hash, &ark_groth16_pvk)
        .map_err(|error| {
            PepperServiceError::BadRequest(format!("Groth16 proof verification failed: {}", error))
        })?;
    Ok(())
}

/// Verifies the signature over the provided message using the ephemeral public key and signature
fn verify_message_signature(
    message: &[u8],
    keyless_signature: &KeylessSignature,
) -> Result<(), PepperServiceError> {
    let KeylessSignature {
        cert: _,
        jwt_header_json: _,
        exp_date_secs: _,
        ephemeral_pubkey,
        ephemeral_signature,
    } = keyless_signature;

    ephemeral_signature
        .verify_arbitrary_msg(message, ephemeral_pubkey)
        .map_err(|error| {
            PepperServiceError::BadRequest(format!(
                "Message signature verification failed: {}",
                error
            ))
        })
}

/// Verifies that the keyless signature has not expired
fn verify_signature_expiry(signature: &KeylessSignature) -> Result<(), PepperServiceError> {
    let current_time_microseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    signature
        .verify_expiry(current_time_microseconds)
        .map_err(|error| {
            PepperServiceError::BadRequest(format!(
                "Signature expiry verification failed: {}",
                error
            ))
        })
}

/// Verifies the training wheels signature (if applicable) and returns the public inputs hash
fn verify_training_wheels_signature(
    jwk_cache: &JWKCache,
    keyless_public_key: &KeylessPublicKey,
    keyless_signature: &KeylessSignature,
    zero_knowledge_sig: &ZeroKnowledgeSig,
    keyless_config: &Configuration,
    training_wheels_pubkey: Option<EphemeralPublicKey>,
) -> Result<Fr, PepperServiceError> {
    // Get the public inputs hash
    let rsa_jwk = get_rsa_jwk(keyless_public_key, keyless_signature, jwk_cache.clone())?;
    let public_inputs_hash = get_public_inputs_hash(
        keyless_signature,
        keyless_public_key,
        &rsa_jwk,
        keyless_config,
    )
    .map_err(|error| {
        PepperServiceError::BadRequest(format!("Failed to compute public inputs hash: {}", error))
    })?;

    // Verify the training wheels signature
    if let Some(training_wheels_pubkey) = &training_wheels_pubkey {
        if let Some(training_wheels_signature) = &zero_knowledge_sig.training_wheels_signature {
            let ZKP::Groth16(groth16proof) = &zero_knowledge_sig.proof;
            let groth16_proof_and_statement =
                Groth16ProofAndStatement::new(*groth16proof, public_inputs_hash);
            training_wheels_signature
                .verify(&groth16_proof_and_statement, training_wheels_pubkey)
                .map_err(|error| {
                    PepperServiceError::BadRequest(format!(
                        "Could not verify training wheels signature: {}",
                        error,
                    ))
                })?;
        } else {
            return Err(PepperServiceError::BadRequest(
                "Training wheels signature expected but it is missing".into(),
            ));
        }
    }

    Ok(public_inputs_hash)
}
