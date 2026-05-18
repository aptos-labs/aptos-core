// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/transaction/authenticator.rs @ rev 8ec3fb76.
// Only the keyless-relevant ephemeral types are pulled in.

use serde::{Deserialize, Serialize};

/// Public key under which a `KeylessSignature.ephemeral_signature` verifies.
/// Either an Ed25519 key (the common case) or a Secp256r1 WebAuthn passkey.
///
/// BCS layout matches `aptos_types::transaction::authenticator::EphemeralPublicKey`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EphemeralPublicKey {
    Ed25519 {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>, // 32 raw bytes
    },
    Secp256r1Ecdsa {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>, // 33 compressed-point bytes
    },
}

/// Signature counterpart to [`EphemeralPublicKey`]. WebAuthn signatures carry
/// the WebAuthn assertion structure (authenticatorData, clientDataJSON, ECDSA
/// sig) — modeled as opaque bytes here; the verifier interprets them.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum EphemeralSignature {
    Ed25519 {
        #[serde(with = "serde_bytes")]
        signature: Vec<u8>, // 64 raw bytes
    },
    WebAuthn {
        /// BCS-encoded `PartialAuthenticatorAssertionResponse`.
        #[serde(with = "serde_bytes")]
        signature: Vec<u8>,
    },
}
