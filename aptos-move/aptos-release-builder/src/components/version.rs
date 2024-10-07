// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::on_chain_config::AptosVersion;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_version_upgrade_proposal(
    version: &AptosVersion,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &["aptos_framework::version"],
        |writer| {
            emitln!(
                writer,
                "version::set_for_next_epoch({}, {});",
                signer_arg,
                version.major,
            );
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("version".to_string(), proposal));
    Ok(result)
}
