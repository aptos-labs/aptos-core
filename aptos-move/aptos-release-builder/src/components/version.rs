// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::Version;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_version_upgrade_proposal(
    version: &Version,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        &["aptos_framework::version"],
        |writer| {
            if is_testnet && next_execution_hash.is_empty() {
                emitln!(
                    writer,
                    "version::set_version(framework_signer, {});",
                    version.major,
                );
            } else {
                emitln!(
                    writer,
                    "version::set_version(&framework_signer, {});",
                    version.major,
                );
            }
        },
    );

    result.push(("version".to_string(), proposal));
    Ok(result)
}
