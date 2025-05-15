// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{intern_type, Function, FunctionHandle, FunctionInstantiation};
use move_binary_format::{
    access::ScriptAccess,
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CompiledScript, FunctionDefinitionIndex, Signature, SignatureIndex, Visibility,
    },
};
use move_core_types::{ident_str, language_storage::ModuleId, vm_status::StatusCode};
use move_vm_types::loaded_data::{
    runtime_access_specifier::AccessSpecifier,
    runtime_types::{StructIdentifier, Type},
    struct_name_indexing::StructNameIndexMap,
};
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

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

        let code: Vec<Bytecode> = script.code.code.clone();
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

        let mut single_signature_token_map = BTreeMap::new();
        for bc in &script.code.code {
            match bc {
                Bytecode::VecPack(si, _)
                | Bytecode::VecLen(si)
                | Bytecode::VecImmBorrow(si)
                | Bytecode::VecMutBorrow(si)
                | Bytecode::VecPushBack(si)
                | Bytecode::VecPopBack(si)
                | Bytecode::VecUnpack(si, _)
                | Bytecode::VecSwap(si) => {
                    if !single_signature_token_map.contains_key(si) {
                        let ty = match script.signature_at(*si).0.first() {
                            None => {
                                return Err(PartialVMError::new(
                                    StatusCode::VERIFIER_INVARIANT_VIOLATION,
                                )
                                .with_message(
                                    "the type argument for vector-related bytecode \
                                                expects one and only one signature token"
                                        .to_owned(),
                                ));
                            },
                            Some(sig_token) => sig_token,
                        };
                        single_signature_token_map.insert(
                            *si,
                            intern_type(BinaryIndexedView::Script(&script), ty, &struct_names)?,
                        );
                    }
                },
                _ => {},
            }
        }

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
