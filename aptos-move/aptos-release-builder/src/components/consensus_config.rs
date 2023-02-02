// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::OnChainConsensusConfig;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn generate_consensus_upgrade_proposal(
    consensus_config: &OnChainConsensusConfig,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    emitln!(writer, "// Consensus config upgrade proposal\n");

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        "aptos_framework::consensus_config",
        |writer| {
            let consensus_config_blob = bcs::to_bytes(consensus_config).unwrap();
            assert!(consensus_config_blob.len() < 65536);

            emit!(writer, "let consensus_blob: vector<u8> = ");
            generate_blob(writer, &consensus_config_blob);
            emitln!(writer, ";\n");

            if is_testnet && next_execution_hash.is_empty() {
                emitln!(
                    writer,
                    "consensus_config::set(framework_signer, consensus_blob);"
                );
            } else {
                emitln!(
                    writer,
                    "consensus_config::set(&framework_signer, consensus_blob);"
                );
            }
        },
    );

    result.push(("consensus-config".to_string(), proposal));
    Ok(result)
}
