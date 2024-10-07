// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_framework::generate_blob_as_hex_string;
use aptos_types::on_chain_config::OnChainConsensusConfig;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn generate_consensus_upgrade_proposal(
    consensus_config: &OnChainConsensusConfig,
    is_testnet: bool,
    next_execution_hash: Option<HashValue>,
    is_multi_step: bool,
) -> Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    emitln!(writer, "// Consensus config upgrade proposal\n");
    let config_comment = format!("// config: {:#?}", consensus_config).replace('\n', "\n// ");
    emitln!(writer, "{}\n", config_comment);

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash,
        is_multi_step,
        &["aptos_framework::consensus_config"],
        |writer| {
            let consensus_config_blob = bcs::to_bytes(consensus_config).unwrap();
            assert!(consensus_config_blob.len() < 65536);

            emit!(writer, "let consensus_blob: vector<u8> = ");
            generate_blob_as_hex_string(writer, &consensus_config_blob);
            emitln!(writer, ";\n");

            emitln!(
                writer,
                "consensus_config::set_for_next_epoch({}, consensus_blob);",
                signer_arg
            );
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("consensus-config".to_string(), proposal));
    Ok(result)
}
