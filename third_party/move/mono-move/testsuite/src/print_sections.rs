// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Render print sections (bytecode, stackless IR, micro-ops) for the
//! differential and snapshot test harnesses.
//!
//! Given a list of [`PrintSection`]s requested via `// RUN: publish
//! --print(...)`, [`render`] produces a string containing a
//! `=== <section> ===` block per requested section per compiled module.
//! Sections are emitted in a fixed order (`bytecode` → `stackless` →
//! `micro-ops`) regardless of the order in the directive.

use crate::parser::PrintSection;
use anyhow::{anyhow, Result};
use mono_move_global_context::ExecutionGuard;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use specializer::{
    destack,
    lower::{lower_function, try_build_context, MicroOpsFunctionDisplay},
    stackless_exec_ir::ModuleIR,
};

/// Render the requested sections for all modules into a single string.
pub fn render(
    guard: &ExecutionGuard<'_>,
    modules: &[CompiledModule],
    sections: &[PrintSection],
) -> Result<String> {
    let want_bytecode = sections.contains(&PrintSection::Bytecode);
    let want_stackless = sections.contains(&PrintSection::Stackless);
    let want_micro_ops = sections.contains(&PrintSection::MicroOps);

    let mut out = String::new();
    for module in modules {
        if want_bytecode {
            push_section(&mut out, "=== bytecode ===\n", &format_bytecode(module)?);
        }
        if want_stackless || want_micro_ops {
            let module_ir =
                destack(module.clone(), guard).map_err(|e| anyhow!("destack failed: {:#}", e))?;
            if want_stackless {
                push_section(&mut out, "=== stackless ===\n", &module_ir.to_string());
            }
            if want_micro_ops {
                push_section(
                    &mut out,
                    "=== micro-ops ===\n",
                    &format_micro_ops(&module_ir),
                );
            }
        }
    }
    Ok(out)
}

fn push_section(out: &mut String, header: &str, content: &str) {
    out.push_str(header);
    out.push_str(content);
    if !content.ends_with('\n') {
        out.push('\n');
    }
}

fn format_bytecode(module: &CompiledModule) -> Result<String> {
    move_asm::disassembler::disassemble_module(String::new(), module)
        .map_err(|e| anyhow!("bytecode disassembly failed: {:#}", e))
}

/// Format the micro-ops for every function in `module_ir`, with a
/// `// module ...` banner and a stanza per function (or a
/// `skipped (...)` line when lowering is not yet supported).
pub fn format_micro_ops(module_ir: &ModuleIR) -> String {
    let module = &module_ir.module;
    let self_handle = module.module_handle_at(module.self_module_handle_idx);
    let addr = module.address_identifier_at(self_handle.address);
    let mod_name = module.identifier_at(self_handle.name);

    let mut out = String::new();
    out.push_str(&format!(
        "// module 0x{}::{}\n",
        addr.short_str_lossless(),
        mod_name
    ));

    for func_ir in module_ir.functions.iter().flatten() {
        let func_name = module.identifier_at(func_ir.name_idx).to_string();
        match try_build_context(module_ir, func_ir) {
            Err(e) => {
                out.push_str(&format!(
                    "\nfun {}(): skipped (context: {})\n",
                    func_name, e
                ));
            },
            Ok(None) => {
                out.push_str(&format!(
                    "\nfun {}(): skipped (not all types are concrete)\n",
                    func_name
                ));
            },
            Ok(Some(ctx)) => match lower_function(func_ir, &ctx) {
                Ok(ops) => {
                    out.push('\n');
                    out.push_str(
                        &MicroOpsFunctionDisplay {
                            func_name: &func_name,
                            ctx: &ctx,
                            ops: &ops,
                        }
                        .to_string(),
                    );
                },
                Err(e) => {
                    out.push_str(&format!(
                        "\nfun {}(): skipped (lowering: {})\n",
                        func_name, e
                    ));
                },
            },
        }
    }
    out
}
