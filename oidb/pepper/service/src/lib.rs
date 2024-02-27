// Copyright © Aptos Foundation

use crate::vuf_keys::VUF_SK;
use anyhow::{anyhow, bail, ensure};
use aptos_oidb_pepper_common::{
    jwt::Claims,
    vuf::{self, VUF},
    PepperInput, PepperRequest, PepperRequestV0, PepperResponse, PepperResponseV0,
};
use aptos_types::{
    oidb::{Configuration, OpenIdSig},
    transaction::authenticator::EphemeralPublicKey,
};
use jsonwebtoken::{Algorithm::RS256, Validation};

pub mod about;
pub mod jwk;
pub mod vuf_keys;

pub type Issuer = String;
pub type KeyID = String;

/// The core processing logic of this pepper service.
pub fn process(request: PepperRequest) -> PepperResponse {
    match request {
        PepperRequest::V0(req) => {
            let response_inner = match process_v0(req) {
                Ok(pepper) => PepperResponseV0::Ok(pepper),
                Err(e) => PepperResponseV0::Err(e.to_string()),
            };
            PepperResponse::V0(response_inner)
        },
    }
}

pub fn process_v0(request: PepperRequestV0) -> anyhow::Result<Vec<u8>> {
    let PepperRequestV0 {
        jwt,
        epk_hex_string,
        epk_expiry_time_secs,
        epk_blinder_hex_string,
        uid_key,
    } = request;

    let claims = aptos_oidb_pepper_common::jwt::parse(jwt.as_str())
        .map_err(|e| anyhow!("JWT decoding error: {e}"))?;

    let actual_uid_key = if let Some(uid_key) = uid_key.as_ref() {
        uid_key
    } else {
        "sub"
    };

    let uid_val = if actual_uid_key == "email" {
        claims
            .claims
            .email
            .clone()
            .ok_or_else(|| anyhow!("`email` required but not found in jwt"))?
    } else if actual_uid_key == "sub" {
        claims.claims.sub.clone()
    } else {
        bail!("unsupported uid key: {}", actual_uid_key)
    };

    let blinder = hex::decode(epk_blinder_hex_string)
        .map_err(|e| anyhow!("blinder unhexlification error: {e}"))?;
    let epk_bytes =
        hex::decode(epk_hex_string).map_err(|e| anyhow!("epk unhexlification error: {e}"))?;
    let epk = bcs::from_bytes::<EphemeralPublicKey>(&epk_bytes)
        .map_err(|e| anyhow!("epk bcs deserialization error: {e}"))?;
    let recalculated_nonce = OpenIdSig::reconstruct_oauth_nonce(
        blinder.as_slice(),
        epk_expiry_time_secs,
        &epk,
        &Configuration::new_for_devnet(),
    )
    .map_err(|e| anyhow!("nonce reconstruction error: {e}"))?;

    ensure!(
        claims.claims.nonce == recalculated_nonce,
        "with nonce mismatch"
    );

    let key_id = claims
        .header
        .kid
        .ok_or_else(|| anyhow!("missing kid in JWT"))?;

    let sig_pub_key = jwk::cached_decoding_key(&claims.claims.iss, &key_id)?;
    let mut validation_with_sig_verification = Validation::new(RS256);
    validation_with_sig_verification.validate_exp = false; // Don't validate the exp time
    let _claims = jsonwebtoken::decode::<Claims>(
        jwt.as_str(),
        sig_pub_key.as_ref(),
        &validation_with_sig_verification,
    ) // Signature verification happens here.
    .map_err(|e| anyhow!("JWT signature verification failed: {e}"))?;

    let input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: actual_uid_key.to_string(),
        uid_val,
        aud: claims.claims.aud.clone(),
    };
    let input_bytes = bcs::to_bytes(&input).unwrap();
    let (pepper, vuf_proof) = vuf::bls12381_g1_bls::Bls12381G1Bls::eval(&VUF_SK, &input_bytes)?;
    ensure!(vuf_proof.is_empty(), "internal proof error");
    Ok(pepper)
}
