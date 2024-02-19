// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::{
    invalid_signature,
    jwks::{jwk::JWK, PatchedJWKs},
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    transaction::authenticator::EphemeralPublicKey,
    vm_status::{StatusCode, VMStatus},
    zkid::{
        get_public_inputs_hash, Configuration, Groth16VerificationKey, ZkIdPublicKey,
        ZkIdSignature, ZkpOrOpenIdSig,
    },
};
use move_binary_format::errors::Location;
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};
use serde::Deserialize;

macro_rules! value_deserialization_error {
    ($message:expr) => {{
        VMStatus::error(
            StatusCode::VALUE_DESERIALIZATION_ERROR,
            Some($message.to_owned()),
        )
    }};
}

fn get_resource_on_chain<T: MoveStructType + for<'a> Deserialize<'a>>(
    resolver: &impl AptosMoveResolver,
) -> anyhow::Result<T, VMStatus> {
    let bytes = resolver
        .get_resource(&CORE_CODE_ADDRESS, &T::struct_tag())
        .map_err(|e| e.finish(Location::Undefined).into_vm_status())?
        .ok_or_else(|| {
            value_deserialization_error!(format!(
                "get_resource failed on {}::{}::{}",
                CORE_CODE_ADDRESS.to_hex_literal(),
                T::struct_tag().module,
                T::struct_tag().name
            ))
        })?;
    let obj = bcs::from_bytes::<T>(&bytes).map_err(|_| {
        value_deserialization_error!(format!(
            "could not deserialize {}::{}::{}",
            CORE_CODE_ADDRESS.to_hex_literal(),
            T::struct_tag().module,
            T::struct_tag().name
        ))
    })?;
    Ok(obj)
}

fn get_current_time_onchain(
    resolver: &impl AptosMoveResolver,
) -> anyhow::Result<CurrentTimeMicroseconds, VMStatus> {
    CurrentTimeMicroseconds::fetch_config(resolver).ok_or_else(|| {
        value_deserialization_error!("could not fetch CurrentTimeMicroseconds on-chain config")
    })
}

fn get_jwks_onchain(resolver: &impl AptosMoveResolver) -> anyhow::Result<PatchedJWKs, VMStatus> {
    PatchedJWKs::fetch_config(resolver)
        .ok_or_else(|| value_deserialization_error!("could not deserialize PatchedJWKs"))
}

fn get_groth16_vk_onchain(
    resolver: &impl AptosMoveResolver,
) -> anyhow::Result<Groth16VerificationKey, VMStatus> {
    get_resource_on_chain::<Groth16VerificationKey>(resolver)
}

fn get_configs_onchain(
    resolver: &impl AptosMoveResolver,
) -> anyhow::Result<Configuration, VMStatus> {
    get_resource_on_chain::<Configuration>(resolver)
}

fn get_jwk_for_zkid_authenticator(
    jwks: &PatchedJWKs,
    zkid_pub_key: &ZkIdPublicKey,
    zkid_sig: &ZkIdSignature,
) -> Result<JWK, VMStatus> {
    let jwt_header = zkid_sig
        .parse_jwt_header()
        .map_err(|_| invalid_signature!("Failed to parse JWT header"))?;
    let jwk_move_struct = jwks
        .get_jwk(&zkid_pub_key.iss_val, &jwt_header.kid)
        .map_err(|_| {
            invalid_signature!(format!(
                "JWK for {} with KID {} was not found",
                zkid_pub_key.iss_val, jwt_header.kid
            ))
        })?;

    let jwk = JWK::try_from(jwk_move_struct)
        .map_err(|_| invalid_signature!("Could not unpack Any in JWK Move struct"))?;
    Ok(jwk)
}

pub fn validate_zkid_authenticators(
    authenticators: &Vec<(ZkIdPublicKey, ZkIdSignature)>,
    resolver: &impl AptosMoveResolver,
) -> Result<(), VMStatus> {
    if authenticators.is_empty() {
        return Ok(());
    }

    let config = &get_configs_onchain(resolver)?;

    if authenticators.len() > config.max_zkid_signatures_per_txn as usize {
        return Err(invalid_signature!("Too many zkID authenticators"));
    }

    let onchain_timestamp_obj = get_current_time_onchain(resolver)?;
    // Check the expiry timestamp on all authenticators first to fail fast
    for (_, zkid_sig) in authenticators {
        zkid_sig
            .verify_expiry(&onchain_timestamp_obj)
            .map_err(|_| invalid_signature!("The ephemeral keypair has expired"))?;
    }

    let patched_jwks = get_jwks_onchain(resolver)?;
    let pvk = &get_groth16_vk_onchain(resolver)?
        .try_into()
        .map_err(|_| invalid_signature!("Could not deserialize on-chain Groth16 VK"))?;

    let training_wheels_pk = match &config.training_wheels_pubkey {
        None => None,
        Some(bytes) => Some(EphemeralPublicKey::ed25519(
            Ed25519PublicKey::try_from(bytes.as_slice()).map_err(|_| {
                invalid_signature!("The training wheels PK set on chain is not a valid PK")
            })?,
        )),
    };

    for (zkid_pub_key, zkid_sig) in authenticators {
        let jwk = get_jwk_for_zkid_authenticator(&patched_jwks, zkid_pub_key, zkid_sig)?;

        match &zkid_sig.sig {
            ZkpOrOpenIdSig::Groth16Zkp(proof) => match jwk {
                JWK::RSA(rsa_jwk) => {
                    if proof.exp_horizon_secs > config.max_exp_horizon_secs {
                        return Err(invalid_signature!("The expiration horizon is too long"));
                    }

                    // If an `aud` override was set for account recovery purposes, check that it is
                    // in the allow-list on-chain.
                    if proof.override_aud_val.is_some() {
                        config.is_allowed_override_aud(proof.override_aud_val.as_ref().unwrap())?;
                    }

                    // The training wheels signature is only checked if a training wheels PK is set on chain
                    if training_wheels_pk.is_some() {
                        proof
                            .verify_training_wheels_sig(training_wheels_pk.as_ref().unwrap())
                            .map_err(|_| {
                                invalid_signature!("Could not verify training wheels signature")
                            })?;
                    }

                    let public_inputs_hash =
                        get_public_inputs_hash(zkid_sig, zkid_pub_key, &rsa_jwk, config).map_err(
                            |_| invalid_signature!("Could not compute public inputs hash"),
                        )?;
                    proof
                        .verify_proof(public_inputs_hash, pvk)
                        .map_err(|_| invalid_signature!("Proof verification failed"))?;
                },
                JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
            },
            ZkpOrOpenIdSig::OpenIdSig(openid_sig) => {
                match jwk {
                    JWK::RSA(rsa_jwk) => {
                        openid_sig
                            .verify_jwt_claims(
                                zkid_sig.exp_timestamp_secs,
                                &zkid_sig.ephemeral_pubkey,
                                zkid_pub_key,
                                config,
                            )
                            .map_err(|_| invalid_signature!("OpenID claim verification failed"))?;

                        // TODO(OpenIdSig): Implement batch verification for all RSA signatures in
                        //  one TXN.
                        // Note: Individual OpenID RSA signature verification will be fast when the
                        // RSA public exponent is small (e.g., 65537). For the same TXN, batch
                        // verification of all RSA signatures will be even faster even when the
                        // exponent is the same. Across different TXNs, batch verification will be
                        // (1) more difficult to implement and (2) not very beneficial since, when
                        // it fails, bad signature identification will require re-verifying all
                        // signatures assuming an adversarial batch.
                        //
                        // We are now ready to verify the RSA signature
                        openid_sig
                            .verify_jwt_signature(&rsa_jwk, &zkid_sig.jwt_header_b64)
                            .map_err(|_| {
                                invalid_signature!(
                                    "RSA signature verification failed for OpenIdSig"
                                )
                            })?;
                    },
                    JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
                }
            },
        }
    }
    Ok(())
}
