// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{components::get_signer_arg, utils::*};
use anyhow::Result;
use move_model::{code_writer::CodeWriter, emitln, model::Loc};

pub fn generate_set_disallow_list_proposal(
    disallow_list: &[String],
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let signer_arg = get_signer_arg(is_testnet, &next_execution_hash);
    let mut result = vec![];

    let writer = CodeWriter::new(Loc::default());
    let proposal = generate_governance_proposal(
        &writer,
        is_testnet,
        next_execution_hash.clone(),
        &["aptos_framework::compiler_id_config"],
        |writer| {
            emitln!(writer, "use std::vector;");
            emitln!(writer, "let ids: vector<vec<u8>> = vector[];");
            emitln!(writer, "let i = 0;");
            emitln!(writer, "while (i < {}) {{", disallow_list.len());
            for id in disallow_list {
                emitln!(writer, "let x = b\"{}\";", id);
                emitln!(writer, "vector::push_back(&mut ids, x);");
            }
            emitln!(writer, "}};");
            emitln!(
                writer,
                "compiler_id_config::set_disallow_list({}, ids);",
                signer_arg,
            );
            emitln!(writer, "aptos_governance::reconfigure({});", signer_arg);
        },
    );

    result.push(("compiler_id_config".to_string(), proposal));
    Ok(result)
}
