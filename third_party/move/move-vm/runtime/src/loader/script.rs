// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::{
    intern_type, BinaryCache, Function, FunctionHandle, FunctionInstantiation,
    ModuleStorageAdapter, Scope, ScriptHash, StructNameCache,
};
use move_binary_format::{
    access::ScriptAccess,
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{Bytecode, CompiledScript, FunctionDefinitionIndex, Signature, SignatureIndex},
};
use move_core_types::{identifier::Identifier, language_storage::ModuleId, vm_status::StatusCode};
use move_vm_types::loaded_data::{
    runtime_access_specifier::AccessSpecifier,
    runtime_types::{StructIdentifier, Type},
};
use std::{collections::BTreeMap, sync::Arc};

// A Script is very similar to a `CompiledScript` but data is "transformed" to a representation
// more appropriate to execution.
// When code executes, indices in instructions are resolved against runtime structures
// (rather than "compiled") to make available data needed for execution.
#[derive(Clone, Debug)]
pub(crate) struct Script {
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
        script_hash: &ScriptHash,
        cache: &ModuleStorageAdapter,
        name_cache: &StructNameCache,
    ) -> VMResult<Self> {
        let mut struct_names = vec![];
        for struct_handle in script.struct_handles() {
            let struct_name = script.identifier_at(struct_handle.name);
            let module_handle = script.module_handle_at(struct_handle.module);
            let module_id = script.module_id_for_handle(module_handle);
            cache
                .get_struct_type_by_identifier(struct_name, &module_id)
                .map_err(|err| err.finish(Location::Script))?
                .check_compatibility(struct_handle)
                .map_err(|err| err.finish(Location::Script))?;

            struct_names.push(name_cache.insert_or_get(StructIdentifier {
                module: module_id,
                name: struct_name.to_owned(),
            }));
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
                instantiation.push(
                    intern_type(BinaryIndexedView::Script(&script), ty, &struct_names)
                        .map_err(|e| e.finish(Location::Script))?,
                );
            }
            function_instantiations.push(FunctionInstantiation {
                handle,
                instantiation,
            });
        }

        let scope = Scope::Script(*script_hash);

        let code: Vec<Bytecode> = script.code.code.clone();
        let parameters = script.signature_at(script.parameters).clone();

        let param_tys = parameters
            .0
            .iter()
            .map(|tok| intern_type(BinaryIndexedView::Script(&script), tok, &struct_names))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;
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
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;
        let ty_param_abilities = script.type_parameters.clone();
        // TODO: main does not have a name. Revisit.
        let name = Identifier::new("main").unwrap();
        let (native, def_is_native) = (None, false); // Script entries cannot be native
        let main: Arc<Function> = Arc::new(Function {
            file_format_version: script.version(),
            index: FunctionDefinitionIndex(0),
            code,
            ty_param_abilities,
            native,
            is_native: def_is_native,
            is_friend_or_private: false,
            is_entry: false,
            scope,
            name,
            // Script must not return values.
            return_tys: vec![],
            local_tys,
            param_tys,
            access_specifier: AccessSpecifier::Any,
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
                                )
                                .finish(Location::Script));
                            },
                            Some(sig_token) => sig_token,
                        };
                        single_signature_token_map.insert(
                            *si,
                            intern_type(BinaryIndexedView::Script(&script), ty, &struct_names)
                                .map_err(|e| e.finish(Location::Script))?,
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

    pub(crate) fn function_instantiation_at(&self, idx: u16) -> &FunctionInstantiation {
        &self.function_instantiations[idx as usize]
    }

    pub(crate) fn single_type_at(&self, idx: SignatureIndex) -> &Type {
        self.single_signature_token_map.get(&idx).unwrap()
    }
}

// A script cache is a map from the hash value of a script and the `Script` itself.
// Script are added in the cache once verified and so getting a script out the cache
// does not require further verification (except for parameters and type parameters)
#[derive(Clone)]
pub(crate) struct ScriptCache {
    pub(crate) scripts: BinaryCache<ScriptHash, Script>,
}

impl ScriptCache {
    pub(crate) fn new() -> Self {
        Self {
            scripts: BinaryCache::new(),
        }
    }

    pub(crate) fn get(&self, hash: &ScriptHash) -> Option<Arc<Function>> {
        self.scripts.get(hash).map(|script| script.entry_point())
    }

    pub(crate) fn insert(&mut self, hash: ScriptHash, script: Script) -> Arc<Function> {
        match self.get(&hash) {
            Some(cached) => cached,
            None => {
                let script = self.scripts.insert(hash, script);
                script.entry_point()
            },
        }
    }
}
