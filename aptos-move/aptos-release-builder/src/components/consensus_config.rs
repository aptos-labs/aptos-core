// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::OnChainConsensusConfig;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn generate_consensus_upgrade_proposal(
    consensus_config: &OnChainConsensusConfig,
    is_testnet: bool,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let gas_schedule_blob = bcs::to_bytes(consensus_config).unwrap();

    assert!(gas_schedule_blob.len() < 65536);

    let writer = CodeWriter::new(Loc::default());

    emitln!(writer, "// Consensus config upgrade proposal\n");

    if is_testnet {
        generate_testnet_header(&writer, "aptos_framework::consensus_config");
    } else {
        generate_governance_proposal_header(&writer, "aptos_framework::consensus_config");
    }

    emit!(writer, "let consensus_blob: vector<u8> = ");
    generate_blob(&writer, &gas_schedule_blob);
    emitln!(writer, ";\n");

    emitln!(
        writer,
        "consensus_config::set(framework_signer, consensus_blob);"
    );

    result.push(("consensus-config".to_string(), finish_with_footer(&writer)));
    Ok(result)
}
