// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::Version;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_version_upgrade_proposal(
    version: &Version,
    is_testnet: bool,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    if is_testnet {
        generate_testnet_header(&writer, "aptos_framework::version");
    } else {
        generate_governance_proposal_header(&writer, "aptos_framework::version");
    }

    emitln!(
        writer,
        "version::set_version(framework_signer, {});",
        version.major,
    );

    result.push(("version".to_string(), finish_with_footer(&writer)));
    Ok(result)
}
