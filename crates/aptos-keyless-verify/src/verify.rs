// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The public-API entry point that downstream consumers call.

use crate::{
    configuration::Configuration,
    errors::VerifyError,
    groth16_vk::Groth16VerificationKey,
    jwk::RsaJwk,
    public_key::KeylessPublicKey,
    signature::KeylessSignature,
};

/// Verify a keyless proof-of-permission over a free-form message (the bytes
/// the user signed — for example, the pretty-printed decryption-request
/// string in an off-chain ACL flow).
///
/// All chain-dependent inputs are taken by reference so the caller has full
/// control over fetching and caching them. The function is pure: no network,
/// no clock, no I/O.
///
/// Verifies, in order:
///   1. `signature.exp_date_secs > now_unix_secs` (the EPK is still live).
///   2. `signature.cert.exp_horizon_secs <= config.max_exp_horizon_secs` for
///      ZK mode.
///   3. The `kid` in `signature.jwt_header_json` matches `jwk.kid`.
///   4. For ZK mode: the Groth16 proof in `signature.cert` verifies under
///      `groth16_vk`, with public-input hash derived from
///      `(pk.iss, pk.idc, ephemeral_pubkey, exp_date_secs, exp_horizon_secs,
///      extra_field, override_aud, jwk_hash)`.
///   5. For OpenID mode: the JWT RSA signature verifies under `jwk`, the
///      reconstructed OAuth nonce matches `signature.ephemeral_pubkey`, and
///      the claims commit to `pk.iss` / `pk.idc`.
///   6. If `config.training_wheels_pubkey` is set, the
///      `training_wheels_signature` on the ZK proof is valid.
///   7. `signature.ephemeral_signature` verifies as Ed25519 (or WebAuthn) on
///      `message` under `signature.ephemeral_pubkey`.
///
/// The function returns `Ok(())` only if every check above passes.
pub fn verify_keyless(
    pk: &KeylessPublicKey,
    signature: &KeylessSignature,
    message: &[u8],
    jwk: &RsaJwk,
    groth16_vk: &Groth16VerificationKey,
    config: &Configuration,
    now_unix_secs: u64,
) -> Result<(), VerifyError> {
    // TODO(impl): the full verification body lands in follow-up commits on this
    // branch. This first commit establishes the crate, its public API, and the
    // dependency surface. Reviewers can sanity-check that:
    //   1. The crate compiles cleanly inside aptos-core.
    //   2. The transitive dep set is acceptable for downstream consumers
    //      (no merlin, no tokio_unstable, no aptos-dkg, no aptos-runtimes).
    //   3. The type shapes line up with `aptos-types::keyless::*`.
    //
    // Implementation outline (port from aptos-core/types/src/keyless/):
    //   A. EPK expiry        — `exp_date_secs > now_unix_secs`
    //   B. Exp-horizon bound — `cert.exp_horizon_secs <= config.max_exp_horizon_secs`
    //   C. kid match         — `signature.jwt_kid() == jwk.kid`
    //   D. Public-input hash — port `bn254_circom::get_public_inputs_hash`
    //   E. Groth16 verify    — port `groth16_sig::verify_groth16_proof`
    //   F. (OpenID mode)     — port `openid_sig::verify_jwt_signature` + claim check
    //   G. Training wheels   — port the EphemeralSignature check
    //   H. Ephemeral sig     — Ed25519::verify on `message` under `ephemeral_pubkey`
    let _ = (pk, signature, message, jwk, groth16_vk, config, now_unix_secs);
    Err(VerifyError::Unsupported(
        "verify_keyless: full implementation is staged behind follow-up commits on this branch",
    ))
}
