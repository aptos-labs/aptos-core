// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validation of the keyless authenticators a transaction carries: the proof
//! that an identity provider vouched for the ephemeral key that signed it.
//!
//! Every read of chain state goes through [`KeylessStateView`], so the same
//! validation serves any VM.

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::{
    invalid_signature,
    jwks::{jwk::JWK, AllProvidersJWKs, FederatedJWKs, PatchedJWKs},
    keyless::{
        get_public_inputs_hash, AnyKeylessPublicKey, Configuration, EphemeralCertificate,
        Groth16ProofAndStatement, KeylessPublicKey, KeylessSignature, ZKP,
    },
    on_chain_config::{CurrentTimeMicroseconds, Features},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
    vm_status::{StatusCode, VMStatus},
};
use ark_bn254::Bn254;
use ark_groth16::PreparedVerifyingKey;
use move_core_types::{
    account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS,
    move_resource::MoveStructType,
};

#[macro_export]
macro_rules! value_deserialization_error {
    ($message:expr) => {{
        ::aptos_types::vm_status::VMStatus::error(
            ::aptos_types::vm_status::StatusCode::VALUE_DESERIALIZATION_ERROR,
            Some($message.to_owned()),
        )
    }};
}

/// The chain state keyless validation reads.
pub trait KeylessStateView {
    /// The current on-chain time, which ephemeral keys expire against.
    fn current_time(&self) -> Result<CurrentTimeMicroseconds, VMStatus>;

    /// The identity providers' keys, as JWK consensus has agreed them.
    fn patched_jwks(&self) -> Result<PatchedJWKs, VMStatus>;

    /// The keys a federated keyless account nominates at `jwk_addr`, for an
    /// issuer the framework's own set does not cover.
    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Result<FederatedJWKs, VMStatus>;
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
pub fn validate_authenticators(
    pvk: Option<&PreparedVerifyingKey<Bn254>>,
    configuration: Option<&Configuration>,
    authenticators: &Vec<(AnyKeylessPublicKey, KeylessSignature)>,
    features: &Features,
    state: &impl KeylessStateView,
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

    let config = configuration.ok_or_else(|| {
        // Preserve error code for compatibility.
        value_deserialization_error!(format!(
            "get_resource failed on {}::{}::{}",
            CORE_CODE_ADDRESS.to_hex_literal(),
            Configuration::struct_tag().module,
            Configuration::struct_tag().name
        ))
    })?;
    if authenticators.len() > config.max_signatures_per_txn as usize {
        return Err(invalid_signature!("Too many keyless authenticators"));
    }

    let onchain_timestamp_obj = state.current_time()?;
    // Check the expiry timestamp on all authenticators first to fail fast
    // This is a redundant check to quickly dismiss expired signatures early and save compute on more computationally costly checks.
    // The actual check is performed in `verify_keyless_signature_without_ephemeral_signature_check`.
    for (_, sig) in authenticators {
        sig.verify_expiry(onchain_timestamp_obj.microseconds)
            .map_err(|_| invalid_signature!("The ephemeral keypair has expired"))?;
    }

    let patched_jwks = state.patched_jwks()?;

    let training_wheels_pk = match &config.training_wheels_pubkey {
        None => None,
        // This takes ~4.4 microseconds, so we are not too concerned about speed here.
        // (Run `cargo bench -- ed25519/pk_deserialize` in `crates/aptos-crypto`.)
        Some(bytes) => Some(EphemeralPublicKey::ed25519(
            Ed25519PublicKey::try_from(bytes.as_slice()).map_err(|_| {
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
                            state.federated_jwks(&fed_pk.jwk_addr).map_err(|_| {
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
    pvk: Option<&PreparedVerifyingKey<Bn254>>,
) -> Result<(), VMStatus> {
    signature
        .verify_expiry(onchain_timestamp_microseconds)
        .map_err(|_| invalid_signature!("The ephemeral keypair has expired"))?;
    match &signature.cert {
        EphemeralCertificate::ZeroKnowledgeSig(zksig) => match jwk {
            JWK::RSA(rsa_jwk) => {
                if zksig.exp_horizon_secs > config.max_exp_horizon_secs {
                    return Err(invalid_signature!("The expiration horizon is too long"));
                }

                // If an `aud` override was set for account recovery purposes, check that it is
                // in the allow-list on-chain.
                if let Some(override_aud_val) = &zksig.override_aud_val {
                    config.is_allowed_override_aud(override_aud_val)?;
                }
                match &zksig.proof {
                    ZKP::Groth16(groth16proof) => {
                        let public_inputs_hash = get_public_inputs_hash(
                            signature,
                            public_key.inner_keyless_pk(),
                            rsa_jwk,
                            config,
                        )
                        .map_err(|_| invalid_signature!("Could not compute public inputs hash"))?;

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
                                    ));
                                },
                            }
                        }

                        zksig
                            .verify_groth16_proof(public_inputs_hash, pvk.unwrap())
                            .map_err(|_| invalid_signature!("Proof verification failed"))?;
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
