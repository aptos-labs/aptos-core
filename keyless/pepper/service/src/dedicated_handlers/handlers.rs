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
    keyless::{
        get_public_inputs_hash, EphemeralCertificate, Groth16ProofAndStatement, KeylessPublicKey,
        KeylessSignature, ZKP,
    },
    transaction::authenticator::{AnyPublicKey, AnySignature, EphemeralPublicKey},
};
use firestore::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[async_trait]
pub trait HandlerTrait<TRequest, TResponse>: Send + Sync {
    /// Returns the name of the handler (e.g., the type name). This is useful for logging.
    fn get_handler_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    // TODO: is there a way we can remove the vuf_private_key param here?
    async fn handle(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        cached_resources: CachedResources,
        request: TRequest,
    ) -> Result<TResponse, PepperServiceError>;
}

pub struct V0FetchHandler;

#[async_trait]
impl HandlerTrait<PepperRequest, PepperResponse> for V0FetchHandler {
    async fn handle(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        _cached_resources: CachedResources,
        request: PepperRequest,
    ) -> Result<PepperResponse, PepperServiceError> {
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
            vuf_private_key,
            jwk_cache,
            &session_id,
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

        Ok(PepperResponse {
            pepper,
            address: address.to_vec(),
        })
    }
}

pub struct V0SignatureHandler;

#[async_trait]
impl HandlerTrait<PepperRequest, SignatureResponse> for V0SignatureHandler {
    async fn handle(
        &self,
        vuf_private_key: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        _cached_resources: CachedResources,
        request: PepperRequest,
    ) -> Result<SignatureResponse, PepperServiceError> {
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
            vuf_private_key,
            jwk_cache,
            &session_id,
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

        Ok(SignatureResponse {
            signature: pepper_base,
        })
    }
}

#[macro_export]
macro_rules! invalid_signature {
    ($message:expr) => {
        PepperServiceError::BadRequest($message.to_owned())
    };
}

pub struct V0VerifyHandler;

#[async_trait]
impl HandlerTrait<VerifyRequest, VerifyResponse> for V0VerifyHandler {
    async fn handle(
        &self,
        _: &ark_bls12_381::Fr,
        jwk_cache: JWKCache,
        cached_resources: CachedResources,
        request: VerifyRequest,
    ) -> Result<VerifyResponse, PepperServiceError> {
        let VerifyRequest {
            public_key,
            signature,
            message,
            address: _,
        } = request;
        if let (AnyPublicKey::Keyless { public_key }, AnySignature::Keyless { signature }) =
            (&public_key, &signature)
        {
            let KeylessPublicKey { idc: _, iss_val } = public_key;
            let KeylessSignature {
                cert,
                jwt_header_json: _,
                exp_date_secs: _,
                ephemeral_pubkey,
                ephemeral_signature,
            } = signature;
            let current_time_microseconds = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64;
            signature
                .verify_expiry(current_time_microseconds)
                .map_err(|_| invalid_signature!("The ephemeral keypair has expired"))?;
            ephemeral_signature
                .verify_arbitrary_msg(&message, ephemeral_pubkey)
                .map_err(|e| {
                    PepperServiceError::BadRequest(format!("Ephemeral sig check failed: {e}"))
                })?;
            let jwt_header = signature.parse_jwt_header().map_err(|e| {
                PepperServiceError::BadRequest(format!("JWT header decoding error: {e}"))
            })?;
            let jwk = jwk_fetcher::get_cached_jwk_as_rsa(iss_val, &jwt_header.kid, jwk_cache)
                .map_err(|e| PepperServiceError::BadRequest(format!("JWK not found: {e}")))?;
            let config_api_repr = cached_resources
                .read_on_chain_keyless_configuration()
                .ok_or_else(|| {
                    PepperServiceError::InternalError(
                        "API keyless config not cached locally.".to_string(),
                    )
                })?;
            let config = config_api_repr.to_rust_repr().map_err(|e| {
                PepperServiceError::InternalError(format!(
                    "Could not parse API keyless config: {e}"
                ))
            })?;
            let training_wheels_pk = match &config.training_wheels_pubkey {
                None => None,
                // This takes ~4.4 microseconds, so we are not too concerned about speed here.
                // (Run `cargo bench -- ed25519/pk_deserialize` in `crates/aptos-crypto`.)
                Some(bytes) => Some(EphemeralPublicKey::ed25519(
                    Ed25519PublicKey::try_from(bytes.as_slice()).map_err(|_| {
                        // println!("[aptos-vm][groth16] On chain TW PK is invalid");
                        invalid_signature!("The training wheels PK set on chain is not a valid PK")
                    })?,
                )),
            };
            match cert {
                EphemeralCertificate::ZeroKnowledgeSig(zksig) => {
                    if zksig.exp_horizon_secs > config.max_exp_horizon_secs {
                        // println!("[aptos-vm][groth16] Expiration horizon is too long");
                        return Err(invalid_signature!("The expiration horizon is too long"));
                    }
                    if zksig.override_aud_val.is_some() {
                        config
                            .is_allowed_override_aud(zksig.override_aud_val.as_ref().unwrap())
                            .map_err(|_| {
                                // println!("[aptos-vm][groth16] PIH computation failed");
                                invalid_signature!("Could not compute public inputs hash")
                            })?;
                    }
                    match &zksig.proof {
                        ZKP::Groth16(groth16proof) => {
                            // let start = std::time::Instant::now();
                            let public_inputs_hash =
                                get_public_inputs_hash(signature, public_key, &jwk, &config)
                                    .map_err(|_| {
                                        // println!("[aptos-vm][groth16] PIH computation failed");
                                        invalid_signature!("Could not compute public inputs hash")
                                    })?;
                            // println!("Public inputs hash time: {:?}", start.elapsed());

                            let groth16_and_stmt =
                                Groth16ProofAndStatement::new(*groth16proof, public_inputs_hash);

                            // The training wheels signature is only checked if a training wheels PK is set on chain
                            if training_wheels_pk.is_some() {
                                match &zksig.training_wheels_signature {
                                    Some(training_wheels_sig) => {
                                        training_wheels_sig
                                            .verify(
                                                &groth16_and_stmt,
                                                training_wheels_pk.as_ref().unwrap(),
                                            )
                                            .map_err(|_| {
                                                // println!("[aptos-vm][groth16] TW sig verification failed");
                                                invalid_signature!(
                                                    "Could not verify training wheels signature"
                                                )
                                            })?;
                                    },
                                    None => {
                                        // println!("[aptos-vm][groth16] Expected TW sig to be set");
                                        return Err(invalid_signature!(
                                            "Training wheels signature expected but it is missing"
                                        ));
                                    },
                                }
                            }

                            let onchain_groth16_vk =
                                cached_resources.read_on_chain_groth16_vk().ok_or_else(|| {
                                    PepperServiceError::InternalError(
                                        "No Groth16 VK cached locally.".to_string(),
                                    )
                                })?;
                            let ark_groth16_pvk = onchain_groth16_vk.to_ark_pvk().map_err(|e| {
                                PepperServiceError::InternalError(format!(
                                    "Onchain-to-ark convertion err: {e}"
                                ))
                            })?;
                            let result =
                                zksig.verify_groth16_proof(public_inputs_hash, &ark_groth16_pvk);
                            result.map_err(|_| {
                                // println!("[aptos-vm][groth16] ZKP verification failed");
                                // println!("[aptos-vm][groth16] PIH: {}", public_inputs_hash);
                                // match zksig.proof {
                                //     ZKP::Groth16(proof) => {
                                //         println!("[aptos-vm][groth16] ZKP: {}", proof.hash());
                                //     },
                                // }
                                // println!(
                                //     "[aptos-vm][groth16] PVK: {}",
                                //     Groth16VerificationKey::from(pvk).hash()
                                // );
                                invalid_signature!("Proof verification failed")
                            })?;
                        },
                    }
                },
                EphemeralCertificate::OpenIdSig(_) => {
                    return Err(invalid_signature!(
                        "Could not verify training wheels signature"
                    ))
                },
            }
            return Ok(VerifyResponse { success: true });
        }
        Err(invalid_signature!("Not a keyless signature"))
    }
}
