// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{
    intern_type, single_signature_loader::load_single_signatures_for_script, Function,
    FunctionHandle, FunctionInstantiation,
};
use move_binary_format::{
    access::ScriptAccess,
    binary_views::BinaryIndexedView,
    errors::PartialVMResult,
    file_format::{CompiledScript, FunctionDefinitionIndex, Signature, SignatureIndex, Visibility},
};
use move_core_types::{ident_str, language_storage::ModuleId};
use move_vm_types::loaded_data::{
    runtime_access_specifier::AccessSpecifier,
    runtime_types::{StructIdentifier, Type},
    struct_name_indexing::StructNameIndexMap,
};
use std::{collections::BTreeMap, ops::Deref};
use triomphe::Arc;

// A Script is very similar to a `CompiledScript` but data is "transformed" to a representation
// more appropriate to execution.
// When code executes, indices in instructions are resolved against runtime structures
// (rather than "compiled") to make available data needed for execution.
#[derive(Clone, Debug)]
pub struct Script {
    // primitive pools
    pub(crate) script: Arc<CompiledScript>,

    // functions as indexes into the Loader function list
    pub(crate) function_refs: Vec<FunctionHandle>,
    // materialized instantiations, whether partial or not
    pub(crate) function_instantiations: Vec<FunctionInstantiation>,

    // entry point
    pub(crate) main: Arc<Function>,

    // a map of single-token signature indices to type
    pub(crate) single_signature_token_map: BTreeMap<SignatureIndex, Type>,
}

impl Script {
    pub(crate) fn new(
        script: Arc<CompiledScript>,
        struct_name_index_map: &StructNameIndexMap,
    ) -> PartialVMResult<Self> {
        let mut struct_names = vec![];
        for struct_handle in script.struct_handles() {
            let struct_name = script.identifier_at(struct_handle.name);
            let module_handle = script.module_handle_at(struct_handle.module);
            let module_id = script.module_id_for_handle(module_handle);
            let struct_name = StructIdentifier {
                module: module_id,
                name: struct_name.to_owned(),
            };
            struct_names.push(struct_name_index_map.struct_name_to_idx(&struct_name)?);
        }

        let mut function_refs = vec![];
        for func_handle in script.function_handles().iter() {
            let func_name = script.identifier_at(func_handle.name);
            let module_handle = script.module_handle_at(func_handle.module);
            let module_id = ModuleId::new(
                *script.address_identifier_at(module_handle.address),
                script.identifier_at(module_handle.name).to_owned(),
            );
            function_refs.push(FunctionHandle::Remote {
                module: module_id,
                name: func_name.to_owned(),
            });
        }

        let mut function_instantiations = vec![];
        for func_inst in script.function_instantiations() {
            let handle = function_refs[func_inst.handle.0 as usize].clone();
            let mut instantiation = vec![];
            for ty in &script.signature_at(func_inst.type_parameters).0 {
                instantiation.push(intern_type(
                    BinaryIndexedView::Script(&script),
                    ty,
                    &struct_names,
                )?);
            }
            function_instantiations.push(FunctionInstantiation {
                handle,
                instantiation,
            });
        }

        let code = script.code.code.clone();
        let parameters = script.signature_at(script.parameters).clone();

        let param_tys = parameters
            .0
            .iter()
            .map(|tok| intern_type(BinaryIndexedView::Script(&script), tok, &struct_names))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let locals = Signature(
            parameters
                .0
                .iter()
                .chain(script.signature_at(script.code.locals).0.iter())
                .cloned()
                .collect(),
        );
        let local_tys = locals
            .0
            .iter()
            .map(|tok| intern_type(BinaryIndexedView::Script(&script), tok, &struct_names))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let ty_param_abilities = script.type_parameters.clone();

        let main: Arc<Function> = Arc::new(Function {
            file_format_version: script.version(),
            index: FunctionDefinitionIndex(0),
            code,
            ty_param_abilities,
            native: None,
            is_native: false,
            visibility: Visibility::Private,
            is_entry: false,
            // TODO: main does not have a name. Revisit.
            name: ident_str!("main").to_owned(),
            // Script must not return values.
            return_tys: vec![],
            local_tys,
            param_tys,
            access_specifier: AccessSpecifier::Any,
            is_persistent: false,
            has_module_reentrancy_lock: false,
        });

        let single_signature_token_map = load_single_signatures_for_script(&script, &struct_names)?;

        Ok(Self {
            script,
            function_refs,
            function_instantiations,
            main,
            single_signature_token_map,
        })
    }

    pub(crate) fn entry_point(&self) -> Arc<Function> {
        self.main.clone()
    }

    pub(crate) fn function_at(&self, idx: u16) -> &FunctionHandle {
        &self.function_refs[idx as usize]
    }

    pub(crate) fn function_instantiation_handle_at(&self, idx: u16) -> &FunctionHandle {
        &self.function_instantiations[idx as usize].handle
    }

    pub(crate) fn function_instantiation_at(&self, idx: u16) -> &[Type] {
        &self.function_instantiations[idx as usize].instantiation
    }

    pub(crate) fn single_type_at(&self, idx: SignatureIndex) -> &Type {
        self.single_signature_token_map.get(&idx).unwrap()
    }
}

impl Deref for Script {
    type Target = Arc<CompiledScript>;

    fn deref(&self) -> &Self::Target {
        &self.script
    }
}
