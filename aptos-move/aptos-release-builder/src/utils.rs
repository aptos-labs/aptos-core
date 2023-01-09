// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use move_model::{code_writer::CodeWriter, emit, emitln};

pub(crate) fn generate_blob(writer: &CodeWriter, data: &[u8]) {
    emitln!(writer, "vector[");
    writer.indent();
    for (i, b) in data.iter().enumerate() {
        if i % 20 == 0 {
            if i > 0 {
                emitln!(writer);
            }
        } else {
            emit!(writer, " ");
        }
        emit!(writer, "{},", b);
    }
    emitln!(writer);
    writer.unindent();
    emit!(writer, "]")
}

pub(crate) fn generate_next_execution_hash_blob(
    writer: &CodeWriter,
    for_address: AccountAddress,
    next_execution_hash: String,
) {
    if next_execution_hash == "vector::empty<u8>()" {
        emitln!(
                writer,
                "let framework_signer = aptos_governance::resolve_multi_step_proposal(proposal_id, @{}, {});\n",
                for_address,
                next_execution_hash,
            );
    } else {
        let next_execution_hash_bytes = next_execution_hash.as_bytes();
        emitln!(
            writer,
            "let framework_signer = aptos_governance::resolve_multi_step_proposal("
        );
        writer.indent();
        emitln!(writer, "proposal_id,");
        emitln!(writer, "@{},", for_address);
        emit!(writer, "vector[");
        for (_, b) in next_execution_hash_bytes.iter().enumerate() {
            emit!(writer, "{}u8,", b);
        }
        emitln!(writer, "],");
        writer.unindent();
        emitln!(writer, "};");
    }
}

pub(crate) fn generate_governance_proposal_header(
    writer: &CodeWriter,
    deps_name: &str,
    is_multi_step: bool,
    next_execution_hash: &str,
) {
    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use aptos_framework::aptos_governance;");
    emitln!(writer, "use {};", deps_name);
    emitln!(writer);

    emitln!(writer, "fun main(proposal_id: u64) {");
    writer.indent();

    if is_multi_step && !next_execution_hash.is_empty() {
        generate_next_execution_hash_blob(
            writer,
            AccountAddress::ONE,
            next_execution_hash.to_owned(),
        );
    } else {
        emitln!(
            writer,
            "let framework_signer = aptos_governance::resolve(proposal_id, @{});\n",
            AccountAddress::ONE,
        );
    }
}

pub(crate) fn generate_testnet_header(writer: &CodeWriter, deps_name: &str) {
    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use aptos_framework::aptos_governance;");
    emitln!(writer, "use {};", deps_name);
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
    next_execution_hash: &str,
    deps_name: &str,
    body: F,
) -> String
where
    F: FnOnce(&CodeWriter),
{
    if next_execution_hash.is_empty() {
        if is_testnet {
            generate_testnet_header(writer, deps_name);
        } else {
            generate_governance_proposal_header(writer, deps_name, false, "");
        }
    } else {
        generate_governance_proposal_header(writer, deps_name, true, next_execution_hash);
    };

    body(writer);
    finish_with_footer(writer)
}
