// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! How and where to record the Serde format of interesting Velor types.
//! See API documentation with `cargo doc -p serde-reflection --open`

use velor_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use velor_types::{
    keyless,
    keyless::{EphemeralCertificate, Groth16Proof, IdCommitment, Pepper, ZeroKnowledgeSig},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
};
use clap::{Parser, ValueEnum};
use serde_reflection::{Registry, Samples, Tracer};
use std::fmt::{Display, Formatter};

/// Rest API types
mod api;
/// Velor transactions.
mod velor;
/// Consensus messages.
mod consensus;
/// Analyze Serde formats to detect certain patterns.
mod linter;
/// Move ABI.
mod move_abi;
/// Network messages.
mod network;

pub use linter::lint_bcs_format;

#[derive(Debug, Parser, Clone, Copy, ValueEnum)]
/// A corpus of Rust types to trace, and optionally record on disk.
pub enum Corpus {
    API,
    Velor,
    Consensus,
    Network,
    MoveABI,
}

impl Corpus {
    /// Compute the registry of formats.
    pub fn get_registry(self) -> Registry {
        let result = match self {
            Corpus::API => api::get_registry(),
            Corpus::Velor => velor::get_registry(),
            Corpus::Consensus => consensus::get_registry(),
            Corpus::Network => network::get_registry(),
            Corpus::MoveABI => move_abi::get_registry(),
        };
        match result {
            Ok(registry) => registry,
            Err(error) => {
                panic!("[corpus {:?}] {}:{}", self, error, error.explanation());
            },
        }
    }

    /// Where to record this corpus on disk.
    pub fn output_file(self) -> Option<&'static str> {
        match self {
            Corpus::API => api::output_file(),
            Corpus::Velor => velor::output_file(),
            Corpus::Consensus => consensus::output_file(),
            Corpus::Network => network::output_file(),
            Corpus::MoveABI => move_abi::output_file(),
        }
    }
}

impl Display for Corpus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Corpus::API => "API",
            Corpus::Velor => "Velor",
            Corpus::Consensus => "Consensus",
            Corpus::Network => "Network",
            Corpus::MoveABI => "MoveABI",
        })
    }
}

pub(crate) fn trace_keyless_structs(
    tracer: &mut Tracer,
    samples: &mut Samples,
    public_key: Ed25519PublicKey,
    signature: Ed25519Signature,
) -> serde_reflection::Result<()> {
    let keyless_public_key = keyless::KeylessPublicKey {
        iss_val: "".to_string(),
        idc: IdCommitment::new_from_preimage(&Pepper::from_number(2), "", "", "").unwrap(),
    };
    let keyless_signature = keyless::KeylessSignature {
        cert: EphemeralCertificate::ZeroKnowledgeSig(ZeroKnowledgeSig {
            proof: Groth16Proof::dummy_proof().into(),
            exp_horizon_secs: 0,
            extra_field: None,
            override_aud_val: None,
            training_wheels_signature: None,
        }),
        jwt_header_json: "".to_string(),
        exp_date_secs: 0,
        ephemeral_pubkey: EphemeralPublicKey::Ed25519 { public_key },
        ephemeral_signature: EphemeralSignature::Ed25519 { signature },
    };
    tracer.trace_value(samples, &keyless_public_key)?;
    tracer.trace_value(samples, &keyless_signature)?;

    Ok(())
}
