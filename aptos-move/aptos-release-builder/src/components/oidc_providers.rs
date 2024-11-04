// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::generate_governance_proposal};
use aptos_crypto::HashValue;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum OidcProviderOp {
    Upsert {
        issuer: String,
        config_url: String,
    },
    Remove {
        issuer: String,
        keep_observed_jwks: bool,
    },
}

pub fn generate_oidc_provider_ops_proposal(
    ops: &[OidcProviderOp],
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> anyhow::Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &["aptos_framework::jwks"],
        |writer| {
            for op in ops {
                write_op(writer, signer_arg, op);
            }
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("oidc-provider-ops".to_string(), proposal));
    Ok(result)
}

fn write_op(writer: &CodeWriter, signer_arg: &str, op: &OidcProviderOp) {
    match op {
        OidcProviderOp::Upsert { issuer, config_url } => {
            emitln!(
                writer,
                "jwks::upsert_oidc_provider_for_next_epoch({}, b\"{}\", b\"{}\");",
                signer_arg,
                issuer,
                config_url
            );
        },
        OidcProviderOp::Remove {
            issuer,
            keep_observed_jwks,
        } => {
            emitln!(
                writer,
                "jwks::remove_oidc_provider_for_next_epoch({}, b\"{}\");",
                signer_arg,
                issuer
            );
            if !keep_observed_jwks {
                emitln!(
                    writer,
                    "jwks::remove_issuer_from_observed_jwks({}, b\"{}\");",
                    signer_arg,
                    issuer
                );
            }
        },
    }
}
