// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Render print sections for the differential and snapshot test harnesses.
//!
//! Given a list of [`PrintSection`]s requested via `// RUN: publish
//! --print(...)`, [`render`] produces a string containing a
//! `=== <section> ===` block per requested section per compiled module.
//!
//! Sections are emitted in the following order, when requested:
//!
//! - `bytecode`
//! - `stackless`
//! - `micro-ops`
//! - `frame-layout`

use crate::parser::PrintSection;
use anyhow::{anyhow, Result};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    types::{view_type_list, FieldLayout, InternedType, InternedTypeList, EMPTY_TYPE_LIST},
    FieldTypes, Interner,
};
use mono_move_global_context::ExecutionGuard;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use specializer::{
    destack,
    lower::{
        context::{try_set_lowering_requirements_for_function, SpecializerContext},
        gc_layout::derive_frame_layout,
        lower_function, try_build_context, BuildContextOutcome, MicroOpsFunctionDisplay,
    },
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
    let want_frame_layout = sections.contains(&PrintSection::FrameLayout);

    let mut out = String::new();
    for module in modules {
        if want_bytecode {
            push_section(&mut out, "=== bytecode ===\n", &format_bytecode(module)?);
        }
        if want_stackless || want_micro_ops || want_frame_layout {
            let module_ir =
                destack(module.clone(), guard).map_err(|e| anyhow!("destack failed: {:#}", e))?;
            if want_stackless {
                push_section(&mut out, "=== stackless ===\n", &module_ir.to_string());
            }
            if want_micro_ops {
                push_section(
                    &mut out,
                    "=== micro-ops ===\n",
                    &format_micro_ops(guard, &module_ir),
                );
            }
            if want_frame_layout {
                push_section(
                    &mut out,
                    "=== frame-layout ===\n",
                    &format_frame_layout(guard, &module_ir),
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
///
/// Sizes types per-function via `try_build_context_v2` before invoking
/// `try_build_context`, so the second call's `type_size_and_align`
/// lookups all hit when the function only references types from the
/// module itself.
pub fn format_micro_ops(guard: &ExecutionGuard<'_>, module_ir: &ModuleIR) -> String {
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

    let mut loader_ctx = SnapshotLoaderContext { guard, module_ir };

    for func_ir in module_ir.functions.iter().flatten() {
        let func_name = module.identifier_at(func_ir.name_idx).to_string();
        if let Err(e) = try_set_lowering_requirements_for_function(
            &mut loader_ctx,
            module_ir,
            func_ir,
            // TODO: we render only at publish time, so there is no way to
            //  render instantiated generics. Figure out what is the best
            //  way to print their code.
            EMPTY_TYPE_LIST,
        ) {
            out.push_str(&format!(
                "\nfun {}(): skipped (cannot set lowering requirements: {})\n",
                func_name, e
            ));
            continue;
        }
        match try_build_context(module_ir, func_ir, EMPTY_TYPE_LIST, guard) {
            Err(e) => {
                out.push_str(&format!(
                    "\nfun {}(): skipped (context: {})\n",
                    func_name, e
                ));
            },
            Ok(BuildContextOutcome::Skipped(reason)) => {
                out.push_str(&format!("\nfun {}(): skipped ({})\n", func_name, reason));
            },
            Ok(BuildContextOutcome::Built(ctx)) => match lower_function(func_ir, &ctx) {
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

/// Format derived `frame_layout` and `zero_frame` for every function
/// in `module_ir`, one line per function. Skipped functions surface
/// their `skipped (...)` reason (same shape as `format_micro_ops`)
/// instead of layout values.
pub fn format_frame_layout(guard: &ExecutionGuard<'_>, module_ir: &ModuleIR) -> String {
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

    let mut loader_ctx = SnapshotLoaderContext { guard, module_ir };

    for func_ir in module_ir.functions.iter().flatten() {
        let func_name = module.identifier_at(func_ir.name_idx).to_string();
        if let Err(e) = try_set_lowering_requirements_for_function(
            &mut loader_ctx,
            module_ir,
            func_ir,
            EMPTY_TYPE_LIST,
        ) {
            out.push_str(&format!(
                "fun {}: skipped (cannot set lowering requirements: {})\n",
                func_name, e
            ));
            continue;
        }
        match try_build_context(module_ir, func_ir, EMPTY_TYPE_LIST, guard) {
            Err(e) => {
                out.push_str(&format!("fun {}: skipped (context: {})\n", func_name, e));
            },
            Ok(BuildContextOutcome::Skipped(reason)) => {
                out.push_str(&format!("fun {}: skipped ({})\n", func_name, reason));
            },
            Ok(BuildContextOutcome::Built(ctx)) => {
                // Mirror `try_lower_function`'s substitution path.
                let home_list = guard.type_list_of(&func_ir.home_slot_types);
                let home_list = match guard.subst_type_list(home_list, EMPTY_TYPE_LIST) {
                    Ok(l) => l,
                    Err(e) => {
                        out.push_str(&format!("fun {}: skipped (subst: {})\n", func_name, e));
                        continue;
                    },
                };
                match derive_frame_layout(&ctx, func_ir, view_type_list(home_list)) {
                    Ok(derived) => {
                        let offsets: Vec<String> = derived
                            .heap_ptr_offsets
                            .iter()
                            .map(|o| o.0.to_string())
                            .collect();
                        out.push_str(&format!(
                            "fun {}: heap_ptr_offsets=[{}] zero_frame={}\n",
                            func_name,
                            offsets.join(", "),
                            derived.zero_frame
                        ));
                    },
                    Err(e) => {
                        out.push_str(&format!("fun {}: skipped (gc_layout: {})\n", func_name, e));
                    },
                }
            },
        }
    }
    out
}

/// `LoaderContext` shim for snapshot rendering. Treats the module being
/// inspected as the only "in-scope" module — cross-module nominals are
/// reported as unresolved, mirroring what `try_build_context` would see
/// today. Layout publishing forwards to the supplied execution guard.
struct SnapshotLoaderContext<'a, 'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    module_ir: &'a ModuleIR,
}

impl SpecializerContext for SnapshotLoaderContext<'_, '_, '_> {
    fn get_fields(
        &mut self,
        module_id: &InternedModuleId,
        nominal_name: &InternedIdentifier,
    ) -> Result<Option<FieldTypes>> {
        if *module_id != self.module_ir.module.id() {
            return Ok(None);
        }
        Ok(self
            .module_ir
            .module
            .interned_field_types(*nominal_name)
            .cloned())
    }

    fn set_nominal_layout(
        &self,
        ty: InternedType,
        size: u32,
        align: u32,
        fields: Option<&[FieldLayout]>,
    ) -> Result<()> {
        self.guard.set_nominal_layout(ty, size, align, fields)
    }

    fn subst_type(&self, ty: InternedType, ty_args: InternedTypeList) -> Result<InternedType> {
        self.guard.subst_type(ty, ty_args)
    }
}
