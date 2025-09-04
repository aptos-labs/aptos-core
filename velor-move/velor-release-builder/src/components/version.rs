// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_types::on_chain_config::VelorVersion;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_version_upgrade_proposal(
    version: &VelorVersion,
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
        &["velor_framework::version"],
        |writer| {
            emitln!(
                writer,
                "version::set_for_next_epoch({}, {});",
                signer_arg,
                version.major,
            );
            emitln!(writer, "velor_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("version".to_string(), proposal));
    Ok(result)
}
