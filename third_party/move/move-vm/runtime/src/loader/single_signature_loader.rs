// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::type_loader::intern_type;
use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{Bytecode, CodeUnit, CompiledModule, CompiledScript, SignatureIndex},
};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
use std::collections::BTreeMap;

/// Maps indices of single signatures to their types. For example, vector bytecode instruction
/// carries an index of a signature of vector element. This loader ensures that all these indices
/// map to their runtime types.
struct SingleSignatureMap<'a> {
    /// Script or a module.
    view: BinaryIndexedView<'a>,
    /// The map from single signature index to its runtime type.
    index_to_type: BTreeMap<SignatureIndex, Type>,
    /// Interned struct names as indices. The caller is responsible for ensuring that all names
    /// are interned and included here.
    struct_idxs: &'a [StructNameIndex],
}

/// Creates a map of all single signature indices to their types for a script.
///
/// The caller is responsible for ensuring that all struct indices (interned names) are included
/// here.

pub(crate) fn load_single_signatures_for_script(
    script: &CompiledScript,
    struct_idxs: &[StructNameIndex],
) -> PartialVMResult<BTreeMap<SignatureIndex, Type>> {
    let mut map = SingleSignatureMap {
        view: BinaryIndexedView::Script(script),
        index_to_type: BTreeMap::new(),
        struct_idxs,
    };

    map.load_single_signatures_for_code_unit(&script.code)?;

    Ok(map.index_to_type)
}

/// Creates a map of all single signature indices to their types for a module.
///
/// The caller is responsible for ensuring that all struct indices (interned names) are included
/// here.
pub(crate) fn load_single_signatures_for_module(
    module: &CompiledModule,
    struct_idxs: &[StructNameIndex],
) -> PartialVMResult<BTreeMap<SignatureIndex, Type>> {
    let mut map = SingleSignatureMap {
        view: BinaryIndexedView::Module(module),
        index_to_type: BTreeMap::new(),
        struct_idxs,
    };

    for function_def in module.function_defs() {
        if let Some(code_unit) = &function_def.code {
            map.load_single_signatures_for_code_unit(code_unit)?;
        }
    }

    Ok(map.index_to_type)
}

impl<'a> SingleSignatureMap<'a> {
    /// Goes over all bytecode instructions, adding mappings from signature index to runtime type
    /// if needed.
    fn load_single_signatures_for_code_unit(&mut self, code: &CodeUnit) -> PartialVMResult<()> {
        for instr in &code.code {
            if let Some(idx) = instr.get_signature_idx() {
                self.load_single_signature_idx(idx, instr)?;
            }
        }
        Ok(())
    }

    /// If signature index is not in the map, converts the signature to runtime type and adds it.
    fn load_single_signature_idx(
        &mut self,
        idx: SignatureIndex,
        instruction: &Bytecode,
    ) -> PartialVMResult<()> {
        if self.index_to_type.contains_key(&idx) {
            return Ok(());
        }

        let sig_toks = &self.view.signature_at(idx).0;
        if sig_toks.len() != 1 {
            return Err(
                PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION).with_message(
                    format!("A single token signature is expected for {:?}", instruction),
                ),
            );
        }

        let ty = intern_type(self.view, &sig_toks[0], self.struct_idxs)?;
        self.index_to_type.insert(idx, ty);
        Ok(())
    }
}
