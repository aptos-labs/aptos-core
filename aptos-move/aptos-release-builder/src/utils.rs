// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_model::{code_writer::CodeWriter, emit, emitln};

pub(crate) fn generate_blob_as_hex_string(writer: &CodeWriter, data: &[u8]) {
    emit!(writer, "x\"");
    for b in data.iter() {
        emit!(writer, "{:02x}", b);
    }
    emit!(writer, "\"");
}

pub(crate) fn generate_next_execution_hash_blob(
    writer: &CodeWriter,
    for_address: AccountAddress,
    next_execution_hash: Vec<u8>,
) {
    if next_execution_hash == "vector::empty<u8>()".as_bytes() {
        emitln!(
                writer,
                "let framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @{}, {});\n",
                for_address,
                "vector::empty<u8>()",
            );
    } else {
        println!("{:?}", next_execution_hash);
        emitln!(
            writer,
            "let framework_signer = aptos_governance::resolve_multi_step_proposal("
        );
        writer.indent();
        emitln!(writer, "proposal_id,");
        emitln!(writer, "@{},", for_address);
        generate_blob_as_hex_string(writer, &next_execution_hash);
        emit!(writer, ",");
        writer.unindent();
        emitln!(writer, ");");
    }
}

pub(crate) fn generate_governance_proposal_header(
    writer: &CodeWriter,
    deps_names: &[&str],
    is_multi_step: bool,
    next_execution_hash: Vec<u8>,
) {
    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use aptos_framework::aptos_governance;");
    for deps_name in deps_names {
        emitln!(writer, "use {};", deps_name);
    }
    if next_execution_hash == "vector::empty<u8>()".as_bytes() {
        emitln!(writer, "use std::vector;");
    }
    emitln!(writer);

    emitln!(writer, "fun main(proposal_id: u64) {");
    writer.indent();

    if is_multi_step && !next_execution_hash.is_empty() {
        generate_next_execution_hash_blob(writer, AccountAddress::ONE, next_execution_hash);
    } else {
        emitln!(
            writer,
            "let framework_signer = aptos_governance::resolve(proposal_id, @{});\n",
            AccountAddress::ONE,
        );
    }
}

pub(crate) fn generate_testnet_header(writer: &CodeWriter, deps_names: &[&str]) {
    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use aptos_framework::aptos_governance;");
    for deps_name in deps_names {
        emitln!(writer, "use {};", deps_name);
    }
    emitln!(writer);

    emitln!(writer, "fun main(core_resources: &signer) {");
    writer.indent();

    emitln!(
        writer,
        "let core_signer = aptos_governance::get_signer_testnet_only(core_resources, @{});\n",
        AccountAddress::ONE,
    );
    emitln!(writer, "let framework_signer = &core_signer;\n");
}

pub(crate) fn finish_with_footer(writer: &CodeWriter) -> String {
    writer.unindent();
    emitln!(writer, "}");

    writer.unindent();
    emitln!(writer, "}");

    writer.process_result(|s| s.to_string())
}

pub(crate) fn generate_governance_proposal<F>(
    writer: &CodeWriter,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
    deps_names: &[&str],
    body: F,
) -> String
where
    F: FnOnce(&CodeWriter),
{
    if next_execution_hash.is_empty() {
        if is_testnet {
            generate_testnet_header(writer, deps_names);
        } else {
            generate_governance_proposal_header(
                writer,
                deps_names,
                false,
                "".to_owned().into_bytes(),
            );
        }
    } else {
        generate_governance_proposal_header(writer, deps_names, true, next_execution_hash);
    };

    body(writer);
    finish_with_footer(writer)
}
