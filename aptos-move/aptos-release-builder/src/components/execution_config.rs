// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::*;
use anyhow::Result;
use aptos_types::on_chain_config::OnChainExecutionConfig;
use move_model::{code_writer::CodeWriter, emit, emitln, model::Loc};

pub fn generate_execution_config_upgrade_proposal(
    execution_config: &OnChainExecutionConfig,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());

    emitln!(writer, "// Execution config upgrade proposal\n");
    let config_comment = format!("// config: {:#?}", execution_config).replace('\n', "\n// ");
    emitln!(writer, "{}\n", config_comment);

    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        &["aptos_framework::execution_config"],
        |writer| {
            let execution_config_blob = bcs::to_bytes(execution_config).unwrap();
            assert!(execution_config_blob.len() < 65536);

            emit!(writer, "let execution_blob: vector<u8> = ");
            generate_blob(writer, &execution_config_blob);
            emitln!(writer, ";\n");

            if is_testnet && next_execution_hash.is_empty() {
                emitln!(
                    writer,
                    "execution_config::set(framework_signer, execution_blob);"
                );
            } else {
                emitln!(
                    writer,
                    "execution_config::set(&framework_signer, execution_blob);"
                );
            }
        },
    );

    result.push(("execution-config".to_string(), proposal));
    Ok(result)
}
