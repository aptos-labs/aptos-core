// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! How and where to record the Serde format of interesting Aptos types.
//! See API documentation with `cargo doc -p serde-reflection --open`

use aptos_batch_encryption::shared::ids::Id;
use aptos_crypto::ed25519::{Ed25519PublicKey, Ed25519Signature};
use aptos_types::{
    keyless::{self, EphemeralCertificate, Groth16Proof, IdCommitment, Pepper, ZeroKnowledgeSig},
    secret_sharing::{Ciphertext, EvalProof},
    transaction::authenticator::{EphemeralPublicKey, EphemeralSignature},
};
use clap::{Parser, ValueEnum};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
};
use serde_reflection::{Registry, Samples, Tracer};
use std::fmt::{Display, Formatter};

/// Rest API types
mod api;
/// Aptos transactions.
mod aptos;
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
    Aptos,
    Consensus,
    Network,
    MoveABI,
}

impl Corpus {
    /// Compute the registry of formats.
    pub fn get_registry(self) -> Registry {
        let result = match self {
            Corpus::API => api::get_registry(),
            Corpus::Aptos => aptos::get_registry(),
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
            Corpus::Aptos => aptos::output_file(),
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
            Corpus::Aptos => "Aptos",
            Corpus::Consensus => "Consensus",
            Corpus::Network => "Network",
            Corpus::MoveABI => "MoveABI",
        })
    }
}

/// Deserialization tracing doesn't understand serde aliases: `StructTag` would get an extra
/// `type_params` field, and `TypeTag` variants would be named after their aliases (e.g. `U8`).
/// Samples make the tracer use the serialized names instead. Needs `record_samples_for_structs`.
pub(crate) fn trace_type_tags(
    tracer: &mut Tracer,
    samples: &mut Samples,
) -> serde_reflection::Result<()> {
    let struct_tag = StructTag {
        address: AccountAddress::ONE,
        module: Identifier::new("m").unwrap(),
        name: Identifier::new("S").unwrap(),
        type_args: vec![TypeTag::Bool],
    };
    for type_tag in [
        TypeTag::Bool,
        TypeTag::U8,
        TypeTag::U16,
        TypeTag::U32,
        TypeTag::U64,
        TypeTag::U128,
        TypeTag::U256,
        TypeTag::I8,
        TypeTag::I16,
        TypeTag::I32,
        TypeTag::I64,
        TypeTag::I128,
        TypeTag::I256,
        TypeTag::Address,
        TypeTag::Signer,
        TypeTag::Vector(Box::new(TypeTag::Bool)),
        TypeTag::Struct(Box::new(struct_tag)),
    ] {
        tracer.trace_value(samples, &type_tag)?;
    }
    Ok(())
}

pub(crate) fn trace_keyless_structs(
    tracer: &mut Tracer,
    samples: &mut Samples,
    public_key: Ed25519PublicKey,
    signature: Ed25519Signature,
) -> serde_reflection::Result<()> {
    tracer.trace_type::<ZeroKnowledgeSig>(samples)?;
    tracer.trace_type::<EphemeralSignature>(samples)?;

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

// TODO(ibalajiarun): Revisit recording struct sample values
fn trace_encrypted_txn_structs(
    tracer: &mut Tracer,
    samples: &mut Samples,
) -> serde_reflection::Result<()> {
    tracer.trace_value(samples, &Ciphertext::random())?;
    tracer.trace_value(samples, &Id::one())?;
    tracer.trace_value(samples, &EvalProof::random())?;

    Ok(())
}
