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

pub(crate) fn generate_governance_proposal_header(writer: &CodeWriter, deps_name: &str) {
    emitln!(writer, "script {");
    writer.indent();

    emitln!(writer, "use aptos_framework::aptos_governance;");
    emitln!(writer, "use {};", deps_name);
    emitln!(writer);

    emitln!(writer, "fun main(proposal_id: u64) {");
    writer.indent();

    emitln!(
        writer,
        "let framework_signer = aptos_governance::resolve(proposal_id, @{});\n",
        AccountAddress::ONE,
    );
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
    deps_name: &str,
    body: F,
) -> String
where
    F: FnOnce(&CodeWriter),
{
    if is_testnet {
        generate_testnet_header(writer, deps_name);
    } else {
        generate_governance_proposal_header(writer, deps_name);
    }
    body(writer);
    finish_with_footer(writer)
}
