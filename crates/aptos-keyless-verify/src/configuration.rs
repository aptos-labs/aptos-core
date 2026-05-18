// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
//
// Vendored from aptos-core/types/src/keyless/configuration.rs @ rev 8ec3fb76.

use serde::{Deserialize, Serialize};

/// On-chain keyless configuration. Published as `0x1::keyless_account::Configuration`.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Configuration {
    /// Hashes of override `aud_val`s allowed in `ZeroKnowledgeSig::override_aud_val`
    /// (used for account recovery).
    pub override_aud_vals: Vec<String>,

    /// Max signatures per signed transaction (only relevant for AccountAuthenticator
    /// — we expose for completeness).
    pub max_signatures_per_txn: u16,

    /// Hard cap on EPK lifetime committed by `ZeroKnowledgeSig::exp_horizon_secs`.
    pub max_exp_horizon_secs: u64,

    /// Optional training-wheels EPK; if present, every ZK signature must include
    /// a `training_wheels_signature` valid under this key.
    pub training_wheels_pubkey: Option<Vec<u8>>,

    /// Max bytes for the JWT `extra_field` revealed in a ZK signature.
    pub max_commited_epk_bytes: u16,

    /// Max bytes for `iss`.
    pub max_iss_val_bytes: u16,

    /// Max bytes for the JWT `extra_field`.
    pub max_extra_field_bytes: u16,

    /// Max bytes for the JWT header.
    pub max_jwt_header_b64_bytes: u32,
}
