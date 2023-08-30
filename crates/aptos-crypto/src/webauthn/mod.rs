// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
//! This module provides an API for the WebAuthn account authenticator as defined in [W3C WebAuthn Level 2](https://www.w3.org/TR/webauthn-2/).

pub mod webauthn_p256_keys;
pub mod webauthn_p256_sigs;

/// Webauthn traits that all signature schemes should implement
pub mod webauthn_traits;

pub use webauthn_p256_keys::{WebAuthnP256PublicKey, WebAuthnP256PublicKey as PublicKey};
pub use webauthn_p256_sigs::{WebAuthnP256Signature, WebAuthnP256Signature as Signature};
