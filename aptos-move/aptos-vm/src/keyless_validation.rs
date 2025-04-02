// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::{
    invalid_signature,
    jwks::{jwk::JWK, AllProvidersJWKs, FederatedJWKs, PatchedJWKs},
    keyless::{
        get_public_inputs_hash, AnyKeylessPublicKey, Configuration, EphemeralCertificate,
        Groth16ProofAndStatement, Groth16VerificationKey, KeylessPublicKey, KeylessSignature, ZKP,
    },
    on_chain_config::{CurrentTimeMicroseconds, Features, OnChainConfig},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
    vm_status::{StatusCode, VMStatus},
};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use move_binary_format::errors::Location;
use move_core_types::{
    account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS,
    move_resource::MoveStructType,
};
use move_vm_runtime::ModuleStorage;
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
    module_storage: &impl ModuleStorage,
) -> anyhow::Result<T, VMStatus> {
    get_resource_on_chain_at_addr(&CORE_CODE_ADDRESS, resolver, module_storage)
}

fn get_resource_on_chain_at_addr<T: MoveStructType + for<'a> Deserialize<'a>>(
    addr: &AccountAddress,
    resolver: &impl AptosMoveResolver,
    module_storage: &impl ModuleStorage,
) -> anyhow::Result<T, VMStatus> {
    let struct_tag = T::struct_tag();
    let metadata = module_storage
        .fetch_existing_module_metadata(&struct_tag.address, &struct_tag.module)
        .map_err(|e| e.into_vm_status())?;
    let bytes = resolver
        .get_resource_bytes_with_metadata_and_layout(addr, &struct_tag, &metadata, None)
        .map_err(|e| e.finish(Location::Undefined).into_vm_status())?
        .0
        .ok_or_else(|| {
            value_deserialization_error!(format!(
                "get_resource failed on {}::{}::{}",
                addr.to_hex_literal(),
                T::struct_tag().module,
                T::struct_tag().name
            ))
        })?;
    let obj = bcs::from_bytes::<T>(&bytes).map_err(|_| {
        value_deserialization_error!(format!(
            "could not deserialize {}::{}::{}",
            addr.to_hex_literal(),
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

fn get_federated_jwks_onchain(
    resolver: &impl AptosMoveResolver,
    jwk_addr: &AccountAddress,
    module_storage: &impl ModuleStorage,
) -> anyhow::Result<FederatedJWKs, VMStatus> {
    get_resource_on_chain_at_addr::<FederatedJWKs>(jwk_addr, resolver, module_storage)
}

pub(crate) fn get_groth16_vk_onchain(
    resolver: &impl AptosMoveResolver,
    module_storage: &impl ModuleStorage,
) -> anyhow::Result<Groth16VerificationKey, VMStatus> {
    get_resource_on_chain::<Groth16VerificationKey>(resolver, module_storage)
}

fn get_configs_onchain(
    resolver: &impl AptosMoveResolver,
    module_storage: &impl ModuleStorage,
) -> anyhow::Result<Configuration, VMStatus> {
    get_resource_on_chain::<Configuration>(resolver, module_storage)
}

// Fetches a JWK from the PatchedJWKs dictionary (which maps each `iss` to its set of JWKs)
//
// This could fail for several reasons:
//  - alg field mismatch: JWT header vs JWK
//  - bad JWT header
//  - bad Any serialization (something is really wrong)
//  - did not find the JWK for the kid
//  - found the JWK for the kid but it is an UnsupportedJWK
fn get_jwk_for_authenticator(
    jwks: &AllProvidersJWKs,
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
pub(crate) fn validate_authenticators(
    pvk: &Option<PreparedVerifyingKey<Bn254>>,
    authenticators: &Vec<(AnyKeylessPublicKey, KeylessSignature)>,
    features: &Features,
    resolver: &impl AptosMoveResolver,
    module_storage: &impl ModuleStorage,
) -> Result<(), VMStatus> {
    let mut with_zk = false;
    for (pk, sig) in authenticators {
        // Feature-gating for keyless TXNs (whether ZK or ZKless, whether passkey-based or not)
        if matches!(sig.cert, EphemeralCertificate::ZeroKnowledgeSig { .. }) {
            if !features.is_zk_keyless_enabled() {
                return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
            }

            with_zk = true;
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
        if matches!(pk, AnyKeylessPublicKey::Federated { .. })
            && !features.is_federated_keyless_enabled()
        {
            return Err(VMStatus::error(StatusCode::FEATURE_UNDER_GATING, None));
        }
    }

    // If there are ZK authenticators, the Groth16 VK must have been set on-chain.
    if with_zk && pvk.is_none() {
        return Err(invalid_signature!("Groth16 VK has not been set on-chain"));
    }

    let config = &get_configs_onchain(resolver, module_storage)?;
    if authenticators.len() > config.max_signatures_per_txn as usize {
        // println!("[aptos-vm][groth16] Too many keyless authenticators");
        return Err(invalid_signature!("Too many keyless authenticators"));
    }

    let onchain_timestamp_obj = get_current_time_onchain(resolver)?;
    // Check the expiry timestamp on all authenticators first to fail fast
    // This is a redundant check to quickly dismiss expired signatures early and save compute on more computationally costly checks.
    // The actual check is performed in `verify_keyless_signature_without_ephemeral_signature_check`.
    for (_, sig) in authenticators {
        sig.verify_expiry(onchain_timestamp_obj.microseconds)
            .map_err(|_| {
                // println!("[aptos-vm][groth16] ZKP expired");

                invalid_signature!("The ephemeral keypair has expired")
            })?;
    }

    let patched_jwks = get_jwks_onchain(resolver)?;

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

    for (pk, sig) in authenticators {
        // Try looking up the jwk in 0x1.
        let jwk = match get_jwk_for_authenticator(&patched_jwks.jwks, pk.inner_keyless_pk(), sig) {
            // 1: If found in 0x1, then we consider that the ground truth & we are done.
            Ok(jwk) => jwk,
            // 2: If not found in 0x1, we check the Keyless PK type.
            Err(e) => {
                match pk {
                    // 2.a: If this is a federated keyless account; look in `jwk_addr` for JWKs
                    AnyKeylessPublicKey::Federated(fed_pk) => {
                        let federated_jwks =
                            get_federated_jwks_onchain(resolver, &fed_pk.jwk_addr, module_storage)
                                .map_err(|_| {
                                    invalid_signature!(format!(
                                        "Could not fetch federated PatchedJWKs at {}",
                                        fed_pk.jwk_addr
                                    ))
                                })?;
                        // 2.a.i If not found in jwk_addr either, then we fail the validation.
                        get_jwk_for_authenticator(&federated_jwks.jwks, pk.inner_keyless_pk(), sig)?
                    },
                    // 2.b: If this is not a federated keyless account, then we fail the validation.
                    AnyKeylessPublicKey::Normal(_) => return Err(e),
                }
            },
        };
        verify_keyless_signature_without_ephemeral_signature_check(
            pk,
            sig,
            &jwk,
            onchain_timestamp_obj.microseconds,
            &training_wheels_pk,
            config,
            pvk,
        )?;
    }

    Ok(())
}

pub fn verify_keyless_signature_without_ephemeral_signature_check(
    public_key: &AnyKeylessPublicKey,
    signature: &KeylessSignature,
    jwk: &JWK,
    onchain_timestamp_microseconds: u64,
    training_wheels_pk: &Option<EphemeralPublicKey>,
    config: &Configuration,
    pvk: &Option<PreparedVerifyingKey<Bn254>>,
) -> Result<(), VMStatus> {
    signature
        .verify_expiry(onchain_timestamp_microseconds)
        .map_err(|_| {
            // println!("[aptos-vm][groth16] ZKP expired");

            invalid_signature!("The ephemeral keypair has expired")
        })?;
    match &signature.cert {
        EphemeralCertificate::ZeroKnowledgeSig(zksig) => match jwk {
            JWK::RSA(rsa_jwk) => {
                if zksig.exp_horizon_secs > config.max_exp_horizon_secs {
                    // println!("[aptos-vm][groth16] Expiration horizon is too long");
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
                        let public_inputs_hash = get_public_inputs_hash(
                            signature,
                            public_key.inner_keyless_pk(),
                            rsa_jwk,
                            config,
                        )
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

                        let result =
                            zksig.verify_groth16_proof(public_inputs_hash, pvk.as_ref().unwrap());

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
            JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
        },
        EphemeralCertificate::OpenIdSig(openid_sig) => {
            match jwk {
                JWK::RSA(rsa_jwk) => {
                    openid_sig
                        .verify_jwt_claims(
                            signature.exp_date_secs,
                            &signature.ephemeral_pubkey,
                            public_key.inner_keyless_pk(),
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
                        .verify_jwt_signature(rsa_jwk, &signature.jwt_header_json)
                        .map_err(|_| {
                            invalid_signature!("RSA signature verification failed for OpenIdSig")
                        })?;
                },
                JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
            }
        },
    }
    Ok(())
}
