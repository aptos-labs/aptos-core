// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::AptosMoveResolver;
use aptos_types::{
    bn254_circom::get_public_inputs_hash,
    chain_id::ChainId,
    jwks::{jwk::JWK, PatchedJWKs},
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    vm_status::{StatusCode, VMStatus},
    zkid::{ZkIdPublicKey, ZkIdSignature, ZkpOrOpenIdSig, MAX_ZK_ID_AUTHENTICATORS_ALLOWED},
};
use move_binary_format::errors::Location;
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};

macro_rules! invalid_signature {
    ($message:expr) => {
        VMStatus::error(StatusCode::INVALID_SIGNATURE, Some($message.to_owned()))
    };
}

fn get_current_time_onchain(
    resolver: &impl AptosMoveResolver,
) -> anyhow::Result<CurrentTimeMicroseconds, VMStatus> {
    CurrentTimeMicroseconds::fetch_config(resolver).ok_or_else(|| {
        VMStatus::error(
            StatusCode::VALUE_DESERIALIZATION_ERROR,
            Some("could not fetch CurrentTimeMicroseconds on-chain config".to_string()),
        )
    })
}

fn get_jwks_onchain(resolver: &impl AptosMoveResolver) -> anyhow::Result<PatchedJWKs, VMStatus> {
    let error_status = VMStatus::error(
        StatusCode::VALUE_DESERIALIZATION_ERROR,
        Some("could not fetch PatchedJWKs".to_string()),
    );
    let bytes = resolver
        .get_resource(&CORE_CODE_ADDRESS, &PatchedJWKs::struct_tag())
        .map_err(|e| e.finish(Location::Undefined).into_vm_status())?
        .ok_or_else(|| error_status.clone())?;
    let jwks = bcs::from_bytes::<PatchedJWKs>(&bytes).map_err(|_| error_status.clone())?;
    Ok(jwks)
}

fn get_jwk_for_zkid_authenticator(
    jwks: &PatchedJWKs,
    zkid_pub_key: &ZkIdPublicKey,
    zkid_sig: &ZkIdSignature,
) -> Result<JWK, VMStatus> {
    let jwt_header_parsed = zkid_sig
        .parse_jwt_header()
        .map_err(|_| invalid_signature!("Failed to get JWT header"))?;
    let jwk_move_struct = jwks
        .get_jwk(&zkid_pub_key.iss, &jwt_header_parsed.kid)
        .map_err(|_| invalid_signature!("JWK not found"))?;

    let jwk =
        JWK::try_from(jwk_move_struct).map_err(|_| invalid_signature!("Could not parse JWK"))?;
    Ok(jwk)
}

pub fn validate_zkid_authenticators(
    authenticators: &Vec<(ZkIdPublicKey, ZkIdSignature)>,
    resolver: &impl AptosMoveResolver,
    chain_id: ChainId,
) -> Result<(), VMStatus> {
    if authenticators.is_empty() {
        return Ok(());
    }

    if authenticators.len() > MAX_ZK_ID_AUTHENTICATORS_ALLOWED {
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

    for (zkid_pub_key, zkid_sig) in authenticators {
        let jwk = get_jwk_for_zkid_authenticator(&patched_jwks, zkid_pub_key, zkid_sig)?;

        match &zkid_sig.sig {
            ZkpOrOpenIdSig::Groth16Zkp(proof) => match jwk {
                JWK::RSA(rsa_jwk) => {
                    let public_inputs_hash =
                        get_public_inputs_hash(zkid_sig, zkid_pub_key, &rsa_jwk).map_err(|_| {
                            invalid_signature!("Could not compute public inputs hash")
                        })?;
                    proof
                        .verify_proof(public_inputs_hash, chain_id)
                        .map_err(|_| invalid_signature!("Proof verification failed"))?;
                },
                JWK::Unsupported(_) => return Err(invalid_signature!("JWK is not supported")),
            },
            ZkpOrOpenIdSig::OpenIdSig(openid_sig) => {
                match jwk {
                    JWK::RSA(rsa_jwk) => {
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
                            .verify_jwt_signature(rsa_jwk, &zkid_sig.jwt_header)
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
