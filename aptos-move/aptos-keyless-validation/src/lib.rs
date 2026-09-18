// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validation of the keyless authenticators a transaction carries: the proof
//! that an identity provider vouched for the ephemeral key that signed it.
//!
//! Every read of chain state goes through [`KeylessStateView`], so the same
//! validation serves any VM.

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_types::{
    jwks::{jwk::JWK, AllProvidersJWKs, FederatedJWKs, PatchedJWKs},
    keyless::{
        get_public_inputs_hash, AnyKeylessPublicKey, Configuration, EphemeralCertificate,
        Groth16ProofAndStatement, KeylessPublicKey, KeylessSignature, KEYLESS_ACCOUNT_MODULE_NAME,
        ZKP,
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
use thiserror::Error;

/// Why a keyless authenticator failed validation.
#[derive(Debug, Error)]
pub enum KeylessValidationError {
    /// The authenticator uses a keyless mode that is not enabled.
    #[error("the keyless mode is not enabled")]
    FeatureDisabled,

    /// The on-chain time could not be read.
    #[error("could not fetch CurrentTimeMicroseconds on-chain config")]
    CurrentTimeUnavailable,
    /// The identity providers' keys could not be read.
    #[error("could not deserialize PatchedJWKs")]
    JwksUnavailable,
    /// The keyless configuration is not set on chain.
    #[error(
        "get_resource failed on {}::{}::{}",
        CORE_CODE_ADDRESS.to_hex_literal(),
        Configuration::struct_tag().module,
        Configuration::struct_tag().name
    )]
    ConfigurationUnavailable,
    /// A federated account's keys could not be read at the address it nominates.
    #[error("Could not fetch federated PatchedJWKs at {0}")]
    FederatedJwksUnavailable(AccountAddress),

    /// A zero-knowledge authenticator was used before the verifying key was set on chain.
    #[error("Groth16 VK has not been set on-chain")]
    VerifyingKeyNotSet,
    /// The transaction carries more keyless authenticators than allowed.
    #[error("Too many keyless authenticators")]
    TooManyAuthenticators,
    /// The ephemeral key pair has expired.
    #[error("The ephemeral keypair has expired")]
    Expired,
    /// The expiry horizon exceeds the configured maximum.
    #[error("The expiration horizon is too long")]
    ExpiryHorizonTooLong,
    /// The `aud` override is not allow-listed.
    #[error(
        "override aud is not allow-listed in 0x1::{}",
        KEYLESS_ACCOUNT_MODULE_NAME
    )]
    OverrideAudNotAllowed,
    /// The training wheels public key set on chain is malformed.
    #[error("The training wheels PK set on chain is not a valid PK")]
    InvalidTrainingWheelsKey,
    /// The training wheels signature is required but absent.
    #[error("Training wheels signature expected but it is missing")]
    TrainingWheelsSignatureMissing,
    /// The training wheels signature does not verify.
    #[error("Could not verify training wheels signature")]
    TrainingWheelsSignatureInvalid,

    /// The JWT header could not be parsed.
    #[error("Failed to parse JWT header")]
    MalformedJwtHeader,
    /// No key of the issuer matches the JWT's key id.
    #[error("JWK for {iss} with KID {kid} was not found")]
    JwkNotFound { iss: String, kid: String },
    /// The stored JWK could not be decoded.
    #[error("Could not unpack Any in JWK Move struct")]
    MalformedJwk,
    /// The JWK's algorithm differs from the JWT header's.
    #[error("JWK alg ({jwk_alg}) does not match JWT header's alg ({jwt_alg})")]
    JwkAlgMismatch { jwk_alg: String, jwt_alg: String },
    /// The issuer's key for this key id is of an unsupported kind.
    #[error(
        "JWK with KID {kid} and hex-encoded payload {} is not supported",
        hex::encode(.payload)
    )]
    UnsupportedJwk { kid: String, payload: Vec<u8> },
    /// The JWK to verify against is not an RSA key.
    #[error("JWK is not supported")]
    JwkNotRsa,

    /// The proof's public inputs could not be hashed.
    #[error("Could not compute public inputs hash")]
    PublicInputsHash,
    /// The zero-knowledge proof does not verify.
    #[error("Proof verification failed")]
    ProofInvalid,
    /// The JWT's claims do not match the authenticator.
    #[error("OpenID claim verification failed")]
    OpenIdClaimsInvalid,
    /// The JWT's RSA signature does not verify.
    #[error("RSA signature verification failed for OpenIdSig")]
    OpenIdSignatureInvalid,
}

impl KeylessValidationError {
    /// The status a transaction failing with this error is discarded with.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::FeatureDisabled => StatusCode::FEATURE_UNDER_GATING,
            Self::CurrentTimeUnavailable
            | Self::JwksUnavailable
            | Self::ConfigurationUnavailable => StatusCode::VALUE_DESERIALIZATION_ERROR,
            Self::FederatedJwksUnavailable(_)
            | Self::VerifyingKeyNotSet
            | Self::TooManyAuthenticators
            | Self::Expired
            | Self::ExpiryHorizonTooLong
            | Self::OverrideAudNotAllowed
            | Self::InvalidTrainingWheelsKey
            | Self::TrainingWheelsSignatureMissing
            | Self::TrainingWheelsSignatureInvalid
            | Self::MalformedJwtHeader
            | Self::JwkNotFound { .. }
            | Self::MalformedJwk
            | Self::JwkAlgMismatch { .. }
            | Self::UnsupportedJwk { .. }
            | Self::JwkNotRsa
            | Self::PublicInputsHash
            | Self::ProofInvalid
            | Self::OpenIdClaimsInvalid
            | Self::OpenIdSignatureInvalid => StatusCode::INVALID_SIGNATURE,
        }
    }
}

impl From<KeylessValidationError> for VMStatus {
    fn from(err: KeylessValidationError) -> Self {
        // A disabled feature is reported without a message.
        let message = if matches!(err, KeylessValidationError::FeatureDisabled) {
            None
        } else {
            Some(err.to_string())
        };
        VMStatus::error(err.status_code(), message)
    }
}

/// The chain state keyless validation reads. `None` means the value could not
/// be read.
pub trait KeylessStateView {
    /// The current on-chain time, which ephemeral keys expire against.
    fn current_time(&self) -> Option<CurrentTimeMicroseconds>;

    /// The identity providers' keys, as JWK consensus has agreed them.
    fn patched_jwks(&self) -> Option<PatchedJWKs>;

    /// The keys a federated keyless account nominates at `jwk_addr`, for an
    /// issuer the framework's own set does not cover.
    fn federated_jwks(&self, jwk_addr: &AccountAddress) -> Option<FederatedJWKs>;
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
) -> Result<JWK, KeylessValidationError> {
    let jwt_header = sig
        .parse_jwt_header()
        .map_err(|_| KeylessValidationError::MalformedJwtHeader)?;

    let jwk_move_struct = jwks.get_jwk(&pk.iss_val, &jwt_header.kid).map_err(|_| {
        KeylessValidationError::JwkNotFound {
            iss: pk.iss_val.clone(),
            kid: jwt_header.kid.clone(),
        }
    })?;

    let jwk = JWK::try_from(jwk_move_struct).map_err(|_| KeylessValidationError::MalformedJwk)?;

    match &jwk {
        JWK::RSA(rsa_jwk) => {
            if rsa_jwk.alg != jwt_header.alg {
                return Err(KeylessValidationError::JwkAlgMismatch {
                    jwk_alg: rsa_jwk.alg.clone(),
                    jwt_alg: jwt_header.alg.clone(),
                });
            }
        },
        JWK::Unsupported(jwk) => {
            return Err(KeylessValidationError::UnsupportedJwk {
                kid: jwt_header.kid.clone(),
                payload: jwk.payload.clone(),
            })
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
) -> Result<(), KeylessValidationError> {
    let mut with_zk = false;
    for (pk, sig) in authenticators {
        // Feature-gating for keyless TXNs (whether ZK or ZKless, whether passkey-based or not)
        if matches!(sig.cert, EphemeralCertificate::ZeroKnowledgeSig { .. }) {
            if !features.is_zk_keyless_enabled() {
                return Err(KeylessValidationError::FeatureDisabled);
            }

            with_zk = true;
        }
        if matches!(sig.cert, EphemeralCertificate::OpenIdSig { .. })
            && !features.is_zkless_keyless_enabled()
        {
            return Err(KeylessValidationError::FeatureDisabled);
        }
        if matches!(sig.ephemeral_signature, EphemeralSignature::WebAuthn { .. })
            && !features.is_keyless_with_passkeys_enabled()
        {
            return Err(KeylessValidationError::FeatureDisabled);
        }
        if matches!(pk, AnyKeylessPublicKey::Federated { .. })
            && !features.is_federated_keyless_enabled()
        {
            return Err(KeylessValidationError::FeatureDisabled);
        }
    }

    // If there are ZK authenticators, the Groth16 VK must have been set on-chain.
    if with_zk && pvk.is_none() {
        return Err(KeylessValidationError::VerifyingKeyNotSet);
    }

    let config = configuration.ok_or(KeylessValidationError::ConfigurationUnavailable)?;
    if authenticators.len() > config.max_signatures_per_txn as usize {
        return Err(KeylessValidationError::TooManyAuthenticators);
    }

    let onchain_timestamp_obj = state
        .current_time()
        .ok_or(KeylessValidationError::CurrentTimeUnavailable)?;
    // Check the expiry timestamp on all authenticators first to fail fast
    // This is a redundant check to quickly dismiss expired signatures early and save compute on more computationally costly checks.
    // The actual check is performed in `verify_keyless_signature_without_ephemeral_signature_check`.
    for (_, sig) in authenticators {
        sig.verify_expiry(onchain_timestamp_obj.microseconds)
            .map_err(|_| KeylessValidationError::Expired)?;
    }

    let patched_jwks = state
        .patched_jwks()
        .ok_or(KeylessValidationError::JwksUnavailable)?;

    let training_wheels_pk = match &config.training_wheels_pubkey {
        None => None,
        // This takes ~4.4 microseconds, so we are not too concerned about speed here.
        // (Run `cargo bench -- ed25519/pk_deserialize` in `crates/aptos-crypto`.)
        Some(bytes) => Some(EphemeralPublicKey::ed25519(
            Ed25519PublicKey::try_from(bytes.as_slice())
                .map_err(|_| KeylessValidationError::InvalidTrainingWheelsKey)?,
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
                        let federated_jwks = state.federated_jwks(&fed_pk.jwk_addr).ok_or(
                            KeylessValidationError::FederatedJwksUnavailable(fed_pk.jwk_addr),
                        )?;
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
) -> Result<(), KeylessValidationError> {
    signature
        .verify_expiry(onchain_timestamp_microseconds)
        .map_err(|_| KeylessValidationError::Expired)?;
    match &signature.cert {
        EphemeralCertificate::ZeroKnowledgeSig(zksig) => match jwk {
            JWK::RSA(rsa_jwk) => {
                if zksig.exp_horizon_secs > config.max_exp_horizon_secs {
                    return Err(KeylessValidationError::ExpiryHorizonTooLong);
                }

                // If an `aud` override was set for account recovery purposes, check that it is
                // in the allow-list on-chain.
                if let Some(override_aud_val) = &zksig.override_aud_val {
                    config
                        .is_allowed_override_aud(override_aud_val)
                        .map_err(|_| KeylessValidationError::OverrideAudNotAllowed)?;
                }
                match &zksig.proof {
                    ZKP::Groth16(groth16proof) => {
                        let public_inputs_hash = get_public_inputs_hash(
                            signature,
                            public_key.inner_keyless_pk(),
                            rsa_jwk,
                            config,
                        )
                        .map_err(|_| KeylessValidationError::PublicInputsHash)?;

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
                                            KeylessValidationError::TrainingWheelsSignatureInvalid
                                        })?;
                                },
                                None => {
                                    return Err(
                                        KeylessValidationError::TrainingWheelsSignatureMissing,
                                    );
                                },
                            }
                        }

                        zksig
                            .verify_groth16_proof(public_inputs_hash, pvk.unwrap())
                            .map_err(|_| KeylessValidationError::ProofInvalid)?;
                    },
                }
            },
            JWK::Unsupported(_) => return Err(KeylessValidationError::JwkNotRsa),
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
                        .map_err(|_| KeylessValidationError::OpenIdClaimsInvalid)?;

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
                        .map_err(|_| KeylessValidationError::OpenIdSignatureInvalid)?;
                },
                JWK::Unsupported(_) => return Err(KeylessValidationError::JwkNotRsa),
            }
        },
    }
    Ok(())
}
