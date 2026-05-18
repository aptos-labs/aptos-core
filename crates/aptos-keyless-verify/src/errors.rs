// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Error type returned by [`verify_keyless`](crate::verify::verify_keyless) and helpers.

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("BCS decode failed: {0}")]
    Decode(String),

    #[error("ephemeral signature verification failed")]
    EphemeralSig,

    #[error("Groth16 proof verification failed")]
    Groth16,

    #[error("JWT signature verification failed")]
    JwtSig,

    #[error("JWT claim mismatch: {0}")]
    ClaimMismatch(&'static str),

    #[error("ephemeral public key expired (exp_date_secs={exp}, now={now})")]
    EpkExpired { exp: u64, now: u64 },

    #[error("exp_horizon_secs ({given}) exceeds configuration limit ({max})")]
    ExpHorizonTooLarge { given: u64, max: u64 },

    #[error("kid in JWT header ({header_kid}) does not match supplied JWK ({jwk_kid})")]
    KidMismatch {
        header_kid: String,
        jwk_kid: String,
    },

    #[error("training-wheels signature missing or invalid")]
    TrainingWheels,

    #[error("unsupported: {0}")]
    Unsupported(&'static str),

    #[error("internal: {0}")]
    Internal(&'static str),
}
