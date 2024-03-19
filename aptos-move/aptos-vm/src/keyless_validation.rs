// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::{
    invalid_signature,
    jwks::{jwk::JWK, PatchedJWKs},
    keyless::{
        get_public_inputs_hash, Configuration, EphemeralCertificate, Groth16ProofAndStatement,
        Groth16VerificationKey, KeylessPublicKey, KeylessSignature, ZKP,
    },
    on_chain_config::{CurrentTimeMicroseconds, Features, OnChainConfig},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
    vm_status::{StatusCode, VMStatus},
};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use move_binary_format::errors::Location;
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};
use once_cell::sync::Lazy;
use quick_cache::sync::Cache;
use serde::Deserialize;
use std::sync::{Mutex, MutexGuard};

macro_rules! value_deserialization_error {
    ($message:expr) => {{
        VMStatus::error(
            StatusCode::VALUE_DESERIALIZATION_ERROR,
            Some($message.to_owned()),
        )
    }};
}

/// TODO(keyless): Comments say Cache should be wrapped in an Arc if used from multiple threads, but do I even need one if it's in a sync::Lazy?
pub(crate) static ZKP_CACHE: Lazy<Cache<Groth16ProofAndStatement, bool>> =
    Lazy::new(|| Cache::new(1_000_000));

/// Whenever the on-chain VK is different from the last seen VK, (1) the ZKP cache is cleared and
/// (2) the last seen VK is updated.
/// Initially, the last seen VK is initialized to an invalid value so that it is imeediately replaced
/// by the on-chain value.
pub(crate) static LAST_SEEN_VK: Lazy<Mutex<PreparedVerifyingKey<Bn254>>> =
    Lazy::new(|| Mutex::new(PreparedVerifyingKey::default()));

fn compare_and_swap_last_seen_vk(
    vk: &Groth16VerificationKey,
) -> anyhow::Result<MutexGuard<PreparedVerifyingKey<Bn254>>, VMStatus> {
    let mut last_seen_vk = LAST_SEEN_VK.lock().unwrap();

    if *vk != *last_seen_vk {
        ZKP_CACHE.clear();
        // let start = std::time::Instant::now();
        *last_seen_vk = vk
            .try_into()
            .map_err(|_| invalid_signature!("Could not deserialize on-chain Groth16 VK"))?;
        // println!("Groth16 VK deserialization time: {:?}", start.elapsed());
    }

    Ok(last_seen_vk)
}

#[cfg(any(test, feature = "testing"))]
pub fn zkp_cache_num_hits() -> usize {
    ZKP_CACHE.hits() as usize
}

/// Returns the # of cached ZKPs.
#[cfg(any(test, feature = "testing"))]
pub fn zkp_cache_size() -> usize {
    ZKP_CACHE.len()
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

fn get_jwk_for_authenticator(
    jwks: &PatchedJWKs,
    pk: &KeylessPublicKey,
    sig: &KeylessSignature,
) -> Result<JWK, VMStatus> {
    let jwt_header = sig
        .parse_jwt_header()
        .map_err(|_| invalid_signature!("Failed to parse JWT header"))?;
    let jwk_move_struct = jwks.get_jwk(&pk.iss_val, &jwt_header.kid).map_err(|_| {
        invalid_signature!(format!(
            "JWK for {} with KID {} was not found",
            pk.iss_val, jwt_header.kid
        ))
    })?;

    let jwk = JWK::try_from(jwk_move_struct)
        .map_err(|_| invalid_signature!("Could not unpack Any in JWK Move struct"))?;

    match &jwk {
        JWK::RSA(rsa_jwk) => {
            if rsa_jwk.alg != jwt_header.alg {
                return Err(invalid_signature!(format!(
                    "JWK alg ({}) does not match JWT header's alg ({})",
                    rsa_jwk.alg, jwt_header.alg
                )));
            }
        },
        JWK::Unsupported(jwk) => {
            return Err(invalid_signature!(format!(
                "JWK with KID {} and hex-encoded payload {} is not supported",
                jwt_header.kid,
                hex::encode(&jwk.payload)
            )))
        },
    }

    Ok(jwk)
}

/// Ensures that **all** keyless authenticators in the transaction are valid.
///
/// WARNING: This function is NOT re-entrant.
pub(crate) fn validate_authenticators(
    authenticators: &Vec<(KeylessPublicKey, KeylessSignature)>,
    features: &Features,
    resolver: &impl AptosMoveResolver,
) -> Result<(), VMStatus> {
    for (_, sig) in authenticators {
        // Feature-gating for keyless TXNs (whether ZK or ZKless, whether passkey-based or not)
        if matches!(sig.cert, EphemeralCertificate::ZeroKnowledgeSig { .. })
            && !features.is_zk_keyless_enabled()
        {
            return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
        }

        if matches!(sig.cert, EphemeralCertificate::OpenIdSig { .. })
            && !features.is_zkless_keyless_enabled()
        {
            return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
        }
        if matches!(sig.ephemeral_signature, EphemeralSignature::WebAuthn { .. })
            && !features.is_keyless_with_passkeys_enabled()
        {
            return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
        }
    }

    let config = &get_configs_onchain(resolver)?;
    if authenticators.len() > config.max_signatures_per_txn as usize {
        return Err(invalid_signature!("Too many keyless authenticators"));
    }

    let onchain_timestamp_obj = get_current_time_onchain(resolver)?;
    // Check the expiry timestamp on all authenticators first to fail fast
    for (_, sig) in authenticators {
        sig.verify_expiry(&onchain_timestamp_obj)
            .map_err(|_| invalid_signature!("The ephemeral keypair has expired"))?;
    }

    // If the VK has changed, the cache will be invalidated.
    let vk = get_groth16_vk_onchain(resolver)?;
    // TODO(keyless): I believe we need a lock here because the VK might be changed by another TXN
    //  during parallel execution? But this effectively serializes the verification of all keyless
    //  TXNs, which is a BIG performance penalty, so we need to avoid this somehow.
    let pvk = compare_and_swap_last_seen_vk(&vk)?;

    let patched_jwks = get_jwks_onchain(resolver)?;

    let training_wheels_pk = match &config.training_wheels_pubkey {
        None => None,
        Some(bytes) => Some(EphemeralPublicKey::ed25519(
            Ed25519PublicKey::try_from(bytes.as_slice()).map_err(|_| {
                invalid_signature!("The training wheels PK set on chain is not a valid PK")
            })?,
        )),
    };

    for (pk, sig) in authenticators {
        let jwk = get_jwk_for_authenticator(&patched_jwks, pk, sig)?;

        match &sig.cert {
            EphemeralCertificate::ZeroKnowledgeSig(zksig) => match jwk {
                JWK::RSA(rsa_jwk) => {
                    if zksig.exp_horizon_secs > config.max_exp_horizon_secs {
                        return Err(invalid_signature!("The expiration horizon is too long"));
                    }

                    // If an `aud` override was set for account recovery purposes, check that it is
                    // in the allow-list on-chain.
                    if zksig.override_aud_val.is_some() {
                        config.is_allowed_override_aud(zksig.override_aud_val.as_ref().unwrap())?;
                    }

                    match &zksig.proof {
                        ZKP::Groth16(groth16proof) => {
                            // let start = std::time::Instant::now();
                            let public_inputs_hash =
                                get_public_inputs_hash(sig, pk, &rsa_jwk, config).map_err(
                                    |_| invalid_signature!("Could not compute public inputs hash"),
                                )?;
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
                                                invalid_signature!(
                                                    "Could not verify training wheels signature"
                                                )
                                            })?;
                                    },
                                    None => {
                                        return Err(invalid_signature!(
                                            "Training wheels signature expected but it is missing"
                                        ))
                                    },
                                }
                            }

                            // If the ZKP has not successfully verified over this statment in the past,
                            // manually verify it & cache it. Otherwise, skip.
                            if matches!(ZKP_CACHE.get(&groth16_and_stmt), None | Some(false)) {
                                let result = zksig.verify_groth16_proof(public_inputs_hash, &pvk);

                                if result.is_ok() {
                                    ZKP_CACHE.insert(groth16_and_stmt, true);
                                }

                                result
                                    .map_err(|_| invalid_signature!("Proof verification failed"))?;
                            }
                        },
                    }
                },
                JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
            },
            EphemeralCertificate::OpenIdSig(openid_sig) => {
                match jwk {
                    JWK::RSA(rsa_jwk) => {
                        openid_sig
                            .verify_jwt_claims(sig.exp_date_secs, &sig.ephemeral_pubkey, pk, config)
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
                            .verify_jwt_signature(&rsa_jwk, &sig.jwt_header_json)
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
