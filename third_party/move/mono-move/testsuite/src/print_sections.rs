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
    native::NoNatives,
    types::{InternedType, EMPTY_TYPE_LIST},
    DescriptorId, FieldTypes, FrameOffset, LayoutId, LayoutProvider, ValueLayout,
};
use mono_move_global_context::ExecutionGuard;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use specializer::{
    destack,
    lower::context::{
        try_discover_types_for_lowering_in_function, try_lower_function, LoweringOutcome,
        SpecializerContext,
    },
    stackless_exec_ir::ModuleIR,
    LoweringResult,
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
                    &render_micro_ops(guard, &module_ir),
                );
            }
            if want_frame_layout {
                push_section(
                    &mut out,
                    "=== frame-layout ===\n",
                    &render_frame_layout(guard, &module_ir),
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

fn push_module_banner(out: &mut String, module: &CompiledModule) {
    let self_handle = module.module_handle_at(module.self_module_handle_idx);
    let addr = module.address_identifier_at(self_handle.address);
    let mod_name = module.identifier_at(self_handle.name);
    out.push_str(&format!(
        "// module 0x{}::{}\n",
        addr.short_str_lossless(),
        mod_name
    ));
}

/// Lower each function in `module_ir`, returning `(name, result)` pairs.
//
// TODO(completeness): we render only at publish time, so there is no way to render
// instantiated generics. Figure out what is the best way to print
// their code.
fn lower_functions(
    guard: &ExecutionGuard<'_>,
    module_ir: &ModuleIR,
) -> Vec<(String, LoweringResult<LoweringOutcome>)> {
    let mut loader_ctx = SnapshotLoaderContext { guard, module_ir };
    module_ir
        .functions
        .iter()
        .flatten()
        .map(|func_ir| {
            let name = module_ir.module.identifier_at(func_ir.name_idx).to_string();
            let result = try_discover_types_for_lowering_in_function(
                &mut loader_ctx,
                guard,
                module_ir,
                func_ir,
                EMPTY_TYPE_LIST,
            )
            .and_then(|descriptors| {
                try_lower_function(
                    module_ir,
                    func_ir,
                    EMPTY_TYPE_LIST,
                    guard,
                    guard,
                    descriptors,
                    &NoNatives,
                )
            });
            (name, result)
        })
        .collect()
}

/// Render the micro-ops section: module banner + per-function stanzas.
pub fn render_micro_ops(guard: &ExecutionGuard<'_>, module_ir: &ModuleIR) -> String {
    let mut out = String::new();
    push_module_banner(&mut out, &module_ir.module);
    for (name, result) in lower_functions(guard, module_ir) {
        match result {
            Ok(LoweringOutcome::Built(f)) => {
                out.push('\n');
                out.push_str(&f.to_string());
            },
            Ok(LoweringOutcome::Skipped(reason)) => {
                out.push_str(&format!("\nfun {}(): skipped ({})\n", name, reason));
            },
            Err(e) => {
                out.push_str(&format!("\nfun {}(): skipped (lowering: {:#})\n", name, e));
            },
        }
    }
    out
}

/// Render the frame-layout section for each function in the module.
pub fn render_frame_layout(guard: &ExecutionGuard<'_>, module_ir: &ModuleIR) -> String {
    let mut out = String::new();
    push_module_banner(&mut out, &module_ir.module);
    for (name, result) in lower_functions(guard, module_ir) {
        match result {
            Ok(LoweringOutcome::Built(f)) => {
                let offsets: Vec<String> = f
                    .frame_layout
                    .heap_ptr_offsets
                    .iter()
                    .map(|o| o.0.to_string())
                    .collect();
                out.push_str(&format!(
                    "fun {}: heap_ptr_offsets=[{}] zero_frame={}\n",
                    name,
                    offsets.join(", "),
                    f.zero_frame
                ));
            },
            Ok(LoweringOutcome::Skipped(reason)) => {
                out.push_str(&format!("fun {}: skipped ({})\n", name, reason));
            },
            Err(e) => {
                out.push_str(&format!("fun {}: skipped (lowering: {:#})\n", name, e));
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

impl LayoutProvider for SnapshotLoaderContext<'_, '_, '_> {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.guard.layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.guard.layout_id(ty)
    }
}

impl SpecializerContext for SnapshotLoaderContext<'_, '_, '_> {
    fn get_fields(
        &mut self,
        module_id: &InternedModuleId,
        nominal_name: &InternedIdentifier,
    ) -> LoweringResult<Option<FieldTypes>> {
        if *module_id != self.module_ir.module.id() {
            return Ok(None);
        }
        Ok(self
            .module_ir
            .module
            .interned_field_types(*nominal_name)
            .cloned())
    }

    fn publish_vec_descriptor(
        &self,
        elem_ty: InternedType,
        elem_size: u32,
        elem_ptr_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.guard
            .publish_vec_descriptor(elem_ty, elem_size, elem_ptr_offsets)
    }

    fn vec_descriptor_for(&self, elem_ty: InternedType) -> Option<DescriptorId> {
        self.guard.vec_descriptor_for(elem_ty)
    }

    fn publish_enum_descriptor(
        &self,
        enum_ty: InternedType,
        size: u32,
        variant_pointer_offsets: Vec<Vec<u32>>,
    ) -> DescriptorId {
        self.guard
            .publish_enum_descriptor(enum_ty, size, variant_pointer_offsets)
    }

    fn publish_captured_data_descriptor(
        &self,
        values_size: u32,
        pointer_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.guard
            .publish_captured_data_descriptor(values_size, pointer_offsets)
    }

    fn publish_layout(&self, ty: InternedType, layout: ValueLayout) -> LayoutId {
        self.guard.publish_layout(ty, layout)
    }

    fn publish_variant_layouts(
        &self,
        enum_ty: InternedType,
        variants: Vec<ValueLayout>,
    ) -> Box<[LayoutId]> {
        self.guard.publish_variant_layouts(enum_ty, variants)
    }

    fn publish_struct_descriptor(
        &self,
        struct_ty: InternedType,
        size: u32,
        ptr_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.guard
            .publish_struct_descriptor(struct_ty, size, ptr_offsets)
    }
}
