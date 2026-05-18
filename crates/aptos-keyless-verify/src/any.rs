// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/transaction/authenticator.rs @ rev 8ec3fb76.
// Only the variants needed to wrap keyless keys/signatures into a `SingleKey`
// authenticator are modeled here; other variants are kept as opaque bytes so
// the BCS discriminants stay stable.

use crate::{ephemeral::EphemeralSignature, public_key::KeylessPublicKey, signature::KeylessSignature};
use serde::{Deserialize, Serialize};

/// `AnyPublicKey` enum variant tags, matching the BCS encoding used by
/// `aptos-types`. We only model the `Keyless` and `FederatedKeyless` variants
/// here in full; the others are kept as opaque byte blobs so unknown
/// public-key types still round-trip without us needing to track every
/// scheme.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AnyPublicKey {
    Ed25519 {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>,
    },
    Secp256k1Ecdsa {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>,
    },
    Secp256r1Ecdsa {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>,
    },
    Keyless {
        public_key: KeylessPublicKey,
    },
    FederatedKeyless {
        // TODO(federated-keyless): add jwk_addr + KeylessPublicKey
        #[serde(with = "serde_bytes")]
        raw: Vec<u8>,
    },
    SlhDsaSha2_128s {
        #[serde(with = "serde_bytes")]
        public_key: Vec<u8>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AnySignature {
    Ed25519 {
        signature: EphemeralSignature, // wire-layout-equivalent: tag+bytes
    },
    Secp256k1Ecdsa {
        #[serde(with = "serde_bytes")]
        signature: Vec<u8>,
    },
    WebAuthn {
        #[serde(with = "serde_bytes")]
        signature: Vec<u8>,
    },
    Keyless {
        signature: KeylessSignature,
    },
    SlhDsaSha2_128s {
        #[serde(with = "serde_bytes")]
        signature: Vec<u8>,
    },
}
