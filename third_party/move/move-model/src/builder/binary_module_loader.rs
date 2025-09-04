// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module implements functions which allow to add binary modules (`CompiledModule`) and
//! scripts (`CompiledScript`) to the global env.

use crate::{
    ast::{
        AccessSpecifier as ASTAccessSpecifier, AccessSpecifierKind as ASTAccessSpecifierKind,
        Address, AddressSpecifier as ASTAddressSpecifier, Attribute, ModuleName,
        ResourceSpecifier as ASTResourceSpecifier,
    },
    model::{
        FieldData, FieldId, FunId, FunctionData, FunctionKind, GlobalEnv, Loc, ModuleData,
        ModuleId, MoveIrLoc, Parameter, StructData, StructId, StructVariant, TypeParameter,
        TypeParameterKind,
    },
    symbol::{Symbol, SymbolPool},
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
};
use itertools::Itertools;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        AccessKind as FFAccessKind, AddressSpecifier as FFAddressSpecifier, FunctionAttribute,
        FunctionDefinitionIndex, FunctionHandleIndex, MemberCount,
        ResourceSpecifier as FFResourceSpecifier, SignatureToken, StructDefinitionIndex,
        StructHandleIndex, TableIndex, VariantIndex, Visibility,
    },
    internals::ModuleIndex,
    views::{
        FieldDefinitionView, FunctionDefinitionView, FunctionHandleView, ModuleView,
        StructDefinitionView, StructHandleView, ViewInternals,
    },
    CompiledModule,
};
use move_bytecode_source_map::source_map::{SourceMap, SourceName};
use move_core_types::{ability::AbilitySet, account_address::AccountAddress, language_storage};
use std::collections::BTreeMap;

/// Macro to abort the execution if `with_dep_closure` is specified while dependencies are missing.
macro_rules! abort_if_missing {
    ($with_dep_closure:expr) => {
        if $with_dep_closure {
            panic!("Malformed bytecode or module loader bug. Please report this.");
        } else {
            // This should not be reached since no existing code sets `with_dep_closure` to `false`.
            // Adding an alert in case things change in the future.
            unimplemented!("[TODO #17414]");
        }
    };
}

impl GlobalEnv {
    /// Loads the compiled module into the environment. If the module already exists,
    /// information about it will be refined according to the provided `module`
    /// and `source_map` parameters. (See below about refinement.)
    ///
    /// If `with_dep_closure` is false, all dependencies of the module must be already present
    /// in the environment. If it is true, 'stub' definitions will be added to the environment
    /// for the needed declarations as they are found in the usage tables of `module`. Adding
    /// stubs leads to 'partial' modules in the environment. Those can be refined in subsequent
    /// `load_compiled_module` calls.
    ///
    /// During loading, any inconsistencies will be reported as errors to the environment.
    /// This can be missing dependencies, but also mismatches between declarations found
    /// in the usage tables of the loaded module and what is already present in the environment.
    /// The post state of the env is guaranteed to be consistent if no errors are produced.
    ///
    /// The compiled module is expected to be verified.
    pub fn load_compiled_module(
        &mut self,
        with_dep_closure: bool,
        module: CompiledModule,
        source_map: SourceMap,
    ) -> ModuleId {
        // NOTE: for the implementation, we leverage the already existing and battle-tested
        // `env.attach_compiled_module` function. This one expects all the module members
        // already in the environment, populated from the AST, and then attaches the bytecode
        // and initializes derived data. This function here basically simulates populating
        // the initial env from bytecode instead of AST, and then calls the bytecode
        // attach.
        let mut loader = BinaryModuleLoader::new(self, with_dep_closure, &module, &source_map);
        loader.load();
        let BinaryModuleLoader { module_id, .. } = loader;
        self.attach_compiled_module(module_id, module, source_map);
        self.update_loaded_modules();
        module_id
    }
}

struct BinaryModuleLoader<'a> {
    env: &'a mut GlobalEnv,
    with_dep_closure: bool,
    module: ModuleView<'a, CompiledModule>,
    module_name: ModuleName,
    module_id: ModuleId,
    module_loc: Loc,
}

impl<'a> BinaryModuleLoader<'a> {
    pub(crate) fn new(
        env: &'a mut GlobalEnv,
        with_dep_closure: bool,
        module: &'a CompiledModule,
        source_map: &'a SourceMap,
    ) -> Self {
        // Create the new modules needed in the environment. If `with_dep_closure` is
        // not enabled, this will error if dependencies are not found.
        let module = ModuleView::new(module);
        let module_loc = env.to_loc(&source_map.definition_location);
        let module_name = make_module_name(module.id(), env.symbol_pool());
        let vector_module_name = ModuleName::new(
            Address::Numerical(AccountAddress::ONE),
            env.symbol_pool().make("vector"),
        );
        let mut module_id = None;
        for handle in module.module_handles() {
            let name = make_module_name(handle.module_id(), env.symbol_pool());
            match env.find_module(&name) {
                None => {
                    if handle.module_id() != module.id() && !with_dep_closure {
                        env.error(
                            &module_loc,
                            &format!(
                                "while loading binary module `{}`: unresolved dependency to module `{}`",
                                module_name.display(env),
                                handle.module_id()
                            ),
                        )
                    } else {
                        let id = ModuleId::new(env.module_data.len());
                        let mut data = ModuleData::new(name.clone(), id, module_loc.clone());
                        if name == vector_module_name {
                            // If this is `0x1::vector`, add well known vector functions.
                            // Those functions do not appear in the bytecode because they are
                            // implemented by specific instructions, but they are in the stackless
                            // bytecode.
                            add_well_known_vector_funs(env, &mut data);
                        }
                        // attach source map if this is the loaded module
                        if handle.module_id() == module.id() {
                            data.source_map = Some(source_map.clone());
                            // Remember id of the loaded module
                            module_id = Some(ModuleId::new(env.module_data.len()))
                        }
                        env.module_data.push(data);
                    }
                },
                Some(module_env) => {
                    if handle.module_id() == module.id() {
                        module_id = Some(module_env.get_id());
                    }
                },
            }
        }
        // If the vector module has not been loaded so far, do now, as it can still be
        // indirectly used via instructions in the code. We could also scan the code
        // if there is any usage, but it's fine to have it in the env even if not used.
        if env.find_module(&vector_module_name).is_none() {
            let id = ModuleId::new(env.module_data.len());
            let mut data = ModuleData::new(vector_module_name, id, module_loc.clone());
            add_well_known_vector_funs(env, &mut data);
            env.module_data.push(data)
        }
        Self {
            env,
            with_dep_closure,
            module,
            module_name,
            module_id: module_id.expect("expected Self module handle"),
            module_loc,
        }
    }

    /// Runs the module builder, adding the resulting module to the environment.
    pub(crate) fn load(&mut self) {
        let struct_handle_to_def: BTreeMap<StructHandleIndex, StructDefinitionIndex> = self
            .module
            .structs()
            .enumerate()
            .map(|(idx, s)| {
                (
                    s.handle_idx(),
                    StructDefinitionIndex::new(idx as TableIndex),
                )
            })
            .collect();
        let fun_handle_to_def: BTreeMap<FunctionHandleIndex, FunctionDefinitionIndex> = self
            .module
            .functions()
            .enumerate()
            .map(|(idx, f)| {
                (
                    f.handle_idx(),
                    FunctionDefinitionIndex::new(idx as TableIndex),
                )
            })
            .collect();
        for (handle_idx, handle_view) in self.module.struct_handles().enumerate() {
            let handle_idx = StructHandleIndex::new(handle_idx as TableIndex);
            let def_view = struct_handle_to_def.get(&handle_idx).map(|def_idx| {
                (
                    *def_idx,
                    self.module.structs().nth(def_idx.into_index()).unwrap(),
                )
            });
            if def_view.is_some() || self.with_dep_closure {
                self.add_or_update_struct(handle_view, def_view)
            }
        }

        for (handle_idx, handle_view) in self.module.function_handles().enumerate() {
            let handle_idx = FunctionHandleIndex::new(handle_idx as TableIndex);
            let def_view = fun_handle_to_def.get(&handle_idx).map(|def_idx| {
                (
                    *def_idx,
                    self.module.functions().nth(def_idx.into_index()).unwrap(),
                )
            });

            if def_view.is_some() || self.with_dep_closure {
                self.add_or_update_fun(handle_view, def_view)
            }
        }
    }

    /// Construct StructData from a handle view and an optional definition view. We always
    /// have a handle view, but only for structs defined in the current module also
    /// a definition view.
    fn add_or_update_struct(
        &mut self,
        handle_view: StructHandleView<CompiledModule>,
        def_view: Option<(StructDefinitionIndex, StructDefinitionView<CompiledModule>)>,
    ) {
        let has_def = def_view.is_some();
        let module_name = make_module_name(handle_view.module_id(), self.env.symbol_pool());
        let module_id = self
            .env
            .find_module(&module_name)
            .expect("undefined module")
            .get_id();
        let struct_id = StructId::new(self.sym(handle_view.name().as_str()));
        let struct_source_map = self.env.module_data[module_id.to_usize()]
            .source_map
            .as_ref()
            .and_then(|sm| {
                def_view
                    .as_ref()
                    .and_then(|(def_idx, _)| sm.get_struct_source_map(*def_idx).ok())
            });
        let loc = self.loc(struct_source_map.map(|s| s.definition_location));
        let type_params = handle_view
            .type_parameters()
            .iter()
            .enumerate()
            .map(|(i, tp)| {
                let (name, loc) = self.type_param(struct_source_map.map(|s| &s.type_parameters), i);
                let kind = TypeParameterKind {
                    abilities: tp.constraints,
                    is_phantom: tp.is_phantom,
                };
                TypeParameter(name, kind, loc)
            })
            .collect_vec();
        let mut field_data: BTreeMap<FieldId, FieldData> = BTreeMap::new();
        let variants = if let Some((_, def_view)) = def_view {
            if def_view.variant_count() > 0 {
                // enum type
                let mut variant_map: BTreeMap<Symbol, StructVariant> = BTreeMap::new();
                for variant in 0..def_view.variant_count() {
                    let variant = variant as VariantIndex;
                    let variant_str = def_view.variant_name(variant).as_str();
                    let variant_sym = self.sym(variant_str);
                    variant_map.insert(variant_sym, StructVariant {
                        loc: loc.clone(), // source map has no info, so default to struct
                        attributes: vec![],
                        order: variant as usize,
                    });
                    for (offset, fld) in def_view.fields_optional_variant(Some(variant)).enumerate()
                    {
                        let loc = self.loc(struct_source_map.and_then(|s| {
                            s.get_field_location(Some(variant), offset as MemberCount)
                        }));
                        let field_id_str =
                            FieldId::make_variant_field_id_str(variant_str, fld.name().as_str());
                        let field_id = FieldId::new(self.sym(&field_id_str));
                        field_data.insert(
                            field_id,
                            self.field_data(loc, offset, Some(variant_sym), fld),
                        );
                    }
                }
                Some(variant_map)
            } else {
                // struct type
                for (offset, fld) in def_view.fields_optional_variant(None).enumerate() {
                    let loc = self.loc(
                        struct_source_map
                            .and_then(|s| s.get_field_location(None, offset as MemberCount)),
                    );
                    let field_id = FieldId::new(self.sym(fld.name().as_str()));
                    field_data.insert(field_id, self.field_data(loc, offset, None, fld));
                }
                None
            }
        } else {
            None
        };

        let mut new = false;
        let struct_data = self.env.module_data[module_id.to_usize()]
            .struct_data
            .entry(struct_id)
            .or_insert_with(|| {
                new = true;
                StructData {
                    abilities: handle_view.abilities(),
                    ..StructData::new(struct_id.symbol(), loc.clone())
                }
            });

        // Verify consistency if the type is already loaded. Can't report it now because
        // the env is mut borrowed.
        let has_error = !new
            && (handle_view.abilities() != struct_data.abilities
                || !type_params_logical_equal(&type_params, &struct_data.type_params));

        if new || has_def {
            // Update if the entry is new, or there is information which is exclusive
            // to definition, like locations.
            struct_data.loc = loc;
            struct_data.type_params = type_params;
            struct_data.field_data = field_data;
            struct_data.variants = variants;
        }

        if has_error {
            self.error(format!(
                "type `{}` has incompatible signature with already loaded type",
                struct_id.symbol().display(self.env.symbol_pool())
            ))
        }
    }

    /// Construct field data.
    fn field_data(
        &self,
        loc: Loc,
        offset: usize,
        variant: Option<Symbol>,
        view: FieldDefinitionView<CompiledModule>,
    ) -> FieldData {
        FieldData {
            name: self.sym(view.name().as_str()),
            loc,
            offset,
            variant,
            ty: self.ty(view.signature_token()),
        }
    }

    /// Construct FunctionData from a handle view and an optional definition view. We always
    /// have a handle view, but only for functions defined in the current module also
    /// a definition view.
    fn add_or_update_fun(
        &mut self,
        handle_view: FunctionHandleView<CompiledModule>,
        def_view: Option<(
            FunctionDefinitionIndex,
            FunctionDefinitionView<CompiledModule>,
        )>,
    ) {
        let has_def = def_view.is_some();
        let module_name = make_module_name(handle_view.module_id(), self.env.symbol_pool());
        let module_id = self
            .env
            .find_module(&module_name)
            .expect("undefined module")
            .get_id();
        let fun_id = FunId::new(self.sym(handle_view.name().as_str()));
        let fun_source_map = self.env.module_data[module_id.to_usize()]
            .source_map
            .as_ref()
            .and_then(|sm| {
                def_view
                    .as_ref()
                    .and_then(|(def_idx, _)| sm.get_function_source_map(*def_idx).ok())
            });

        let loc = self.loc(fun_source_map.map(|s| s.definition_location));
        let type_params = handle_view
            .type_parameters()
            .iter()
            .enumerate()
            .map(|(i, tp)| {
                let (name, loc) = self.type_param(fun_source_map.map(|s| &s.type_parameters), i);
                let kind = TypeParameterKind {
                    abilities: *tp,
                    is_phantom: false,
                };
                TypeParameter(name, kind, loc)
            })
            .collect_vec();

        let params = handle_view
            .parameters()
            .0
            .iter()
            .enumerate()
            .map(|(i, sign)| {
                let (name, loc) = fun_source_map
                    .and_then(|s| {
                        s.get_parameter_or_local_name(i as u64)
                            .map(|(n, l)| (self.sym(&n), self.loc(Some(l))))
                    })
                    .unwrap_or_else(|| (self.sym(&format!("p{}", i)), loc.clone()));
                Parameter(name, self.ty(sign), loc)
            })
            .collect_vec();

        let result_type = Type::tuple(handle_view.return_().0.iter().map(|s| self.ty(s)).collect());

        // Convert access specifiers from file format to AST format
        let rw_specifiers = self.rw_specifiers(&handle_view, &params);
        let acquire_specifiers = if let Some((_, def_view)) = def_view.clone() {
            self.acquire_specifiers(&def_view)
        } else {
            None
        };
        // Combine read/write and acquire specifiers.
        let access_specifiers = rw_specifiers
            .clone()
            .zip(acquire_specifiers.clone())
            .map(|(mut rw, acquire)| {
                rw.extend(acquire);
                rw
            })
            .or(rw_specifiers)
            .or(acquire_specifiers);

        let (visibility, is_native, kind) = if let Some((_, def_view)) = def_view {
            (
                def_view.visibility(),
                def_view.is_native(),
                if def_view.is_entry() {
                    FunctionKind::Entry
                } else {
                    FunctionKind::Regular
                },
            )
        } else {
            // We do not know this info from the handle of an imported function, so choose
            // defaults
            (Visibility::Public, false, FunctionKind::Regular)
        };

        // add attributes to the function
        let mut attributes = vec![];
        for attr in handle_view.attributes() {
            match attr {
                FunctionAttribute::Persistent => {
                    if !visibility.is_public() {
                        let node_id = self.env.new_node(Loc::default(), Type::Tuple(vec![]));
                        let sym = self
                            .env
                            .symbol_pool()
                            .make(well_known::PERSISTENT_ATTRIBUTE);
                        attributes.push(Attribute::Apply(node_id, sym, vec![]));
                    }
                },
                FunctionAttribute::ModuleLock => {
                    let node_id = self.env.new_node(Loc::default(), Type::Tuple(vec![]));
                    let sym = self
                        .env
                        .symbol_pool()
                        .make(well_known::MODULE_LOCK_ATTRIBUTE);
                    attributes.push(Attribute::Apply(node_id, sym, vec![]));
                },
            }
        }

        let mut new = false;
        let fun_data = self.env.module_data[module_id.to_usize()]
            .function_data
            .entry(fun_id)
            .or_insert_with(|| {
                new = true;
                FunctionData {
                    visibility,
                    is_native,
                    kind,
                    attributes,
                    ..FunctionData::new(fun_id.symbol(), loc)
                }
            });

        // Verify consistency if the type is already loaded. Can't report it now because
        // the env is mut borrowed.
        let has_error = !new
            && (!type_params_logical_equal(&type_params, &fun_data.type_params)
                || !params_logical_equal(&params, &fun_data.params)
                || result_type != fun_data.result_type);

        if new || has_def {
            // Update if the entry is new, or there is information which is exclusive
            // to definition, like locations.
            fun_data.type_params = type_params;
            fun_data.params = params;
            fun_data.access_specifiers = access_specifiers;
            fun_data.result_type = result_type;
        }

        if has_error {
            // verify consistency if the function is already loaded
            self.error(format!(
                "function `{}` has incompatible signature with already loaded function",
                fun_id.symbol().display(self.env.symbol_pool())
            ))
        }
    }

    fn rw_specifiers(
        &self,
        handle_view: &FunctionHandleView<CompiledModule>,
        params: &[Parameter],
    ) -> Option<Vec<ASTAccessSpecifier>> {
        let ff_access_specifiers = handle_view.access_specifiers()?;
        let mut access_specifiers = Vec::new();
        let ff_module = handle_view.module();

        // Helper function mapping a file format `ModuleId` to a `ModuleEnv` in move model
        let ff_mid_to_module_env = |ff_mid: language_storage::ModuleId| {
            let mname = self.env.to_module_name(&ff_mid);
            self.env
                .find_module(&mname)
                .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure))
        };

        for ff_acc_spec in ff_access_specifiers {
            let spec_kind = match ff_acc_spec.kind {
                FFAccessKind::Reads => ASTAccessSpecifierKind::Reads,
                FFAccessKind::Writes => ASTAccessSpecifierKind::Writes,
            };
            let spec_negated = ff_acc_spec.negated;
            let resource_spec = match ff_acc_spec.resource {
                FFResourceSpecifier::Any => ASTResourceSpecifier::Any,
                FFResourceSpecifier::DeclaredAtAddress(addr_idx) => {
                    let acct_addr = ff_module.address_identifier_at(addr_idx);
                    ASTResourceSpecifier::DeclaredAtAddress(Address::Numerical(*acct_addr))
                },
                FFResourceSpecifier::DeclaredInModule(mhid) => {
                    // ModuleHandleIndex -> ModuleHandle -> File format ModuleId
                    let ff_mhandle = ff_module.module_handle_at(mhid);
                    let ff_mid = ff_module.module_id_for_handle(ff_mhandle);
                    // File format ModuleId -> ModuleEnv in move model
                    let module_env = ff_mid_to_module_env(ff_mid);
                    ASTResourceSpecifier::DeclaredInModule(module_env.get_id())
                },
                FFResourceSpecifier::Resource(shid) => {
                    // StructHandleIndex -> StructHandle -> ModuleHandle -> File format ModuleId
                    let ff_shandle = ff_module.struct_handle_at(shid);
                    let ff_mhandle = ff_module.module_handle_at(ff_shandle.module);
                    let ff_mid = ff_module.module_id_for_handle(ff_mhandle);
                    // StructHandle -> struct name -> StructId in move model
                    let sname = ff_module.identifier_at(ff_shandle.name);
                    let struct_id = StructId::new(self.sym(sname.as_str()));
                    // File format ModuleId -> ModuleEnv in move model -> StructEnv in move model
                    let module_env = ff_mid_to_module_env(ff_mid);
                    let struct_env = self
                        .env
                        .get_struct_opt(module_env.get_id().qualified(struct_id))
                        .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure));
                    ASTResourceSpecifier::Resource(
                        module_env
                            .get_id()
                            .qualified_inst(struct_env.get_id(), vec![]),
                    )
                },
                FFResourceSpecifier::ResourceInstantiation(shid, sig_idx) => {
                    // Process similar to `FFResourceSpecifier::Resource`
                    let ff_shandle = ff_module.struct_handle_at(shid);
                    let ff_mhandle = ff_module.module_handle_at(ff_shandle.module);
                    let ff_mid = ff_module.module_id_for_handle(ff_mhandle);
                    let sname = ff_module.identifier_at(ff_shandle.name);
                    let sig = ff_module.signature_at(sig_idx);
                    let type_args: Vec<_> = sig.0.iter().map(|arg| self.ty(arg)).collect();
                    let struct_id = StructId::new(self.sym(sname.as_str()));
                    let module_env = ff_mid_to_module_env(ff_mid);
                    let struct_env = self
                        .env
                        .get_struct_opt(module_env.get_id().qualified(struct_id))
                        .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure));
                    ASTResourceSpecifier::Resource(
                        module_env
                            .get_id()
                            .qualified_inst(struct_env.get_id(), type_args),
                    )
                },
            };
            let addr_spec = match ff_acc_spec.address {
                FFAddressSpecifier::Any => ASTAddressSpecifier::Any,
                FFAddressSpecifier::Literal(addr_idx) => {
                    let addr = Address::Numerical(*ff_module.address_identifier_at(addr_idx));
                    ASTAddressSpecifier::Address(addr)
                },
                FFAddressSpecifier::Parameter(param_idx, func_inst_inx) => {
                    if let Some(func_inst_idx) = func_inst_inx {
                        // FunctionInstantiationIndex -> FunctionInstantiation -> FunctionHandle -> ModuleHandle -> File format ModuleId
                        let ff_func_inst = ff_module.function_instantiation_at(func_inst_idx);
                        let ff_func_handle = ff_module.function_handle_at(ff_func_inst.handle);
                        let ff_mhandle = ff_module.module_handle_at(ff_func_handle.module);
                        let ff_mid = ff_module.module_id_for_handle(ff_mhandle);
                        // FunctionHandle -> function name -> FunId in move model
                        let fname = ff_module.identifier_at(ff_func_handle.name);
                        let fun_id = FunId::new(self.sym(fname.as_str()));
                        // File format ModuleId -> ModuleEnv in move model -> FunctionEnv in move model
                        let module_env = ff_mid_to_module_env(ff_mid);
                        let fun_env = self
                            .env
                            .get_function_opt(module_env.get_id().qualified(fun_id))
                            .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure));
                        ASTAddressSpecifier::Call(
                            module_env
                                .get_id()
                                .qualified_inst(fun_env.get_id(), fun_env.get_parameter_types()),
                            params[param_idx as usize].0,
                        )
                    } else {
                        ASTAddressSpecifier::Parameter(params[param_idx as usize].0)
                    }
                },
            };
            let loc = Loc::default();
            let ast_access_specifier: ASTAccessSpecifier = ASTAccessSpecifier {
                kind: spec_kind,
                negated: spec_negated,
                resource: (loc.clone(), resource_spec),
                address: (loc.clone(), addr_spec),
                loc,
            };
            access_specifiers.push(ast_access_specifier);
        }
        Some(access_specifiers)
    }

    fn acquire_specifiers(
        &self,
        def_view: &FunctionDefinitionView<CompiledModule>,
    ) -> Option<Vec<ASTAccessSpecifier>> {
        let mut access_specifiers = Vec::new();
        let ff_module = def_view.module();
        let ff_acquires = def_view.acquired_resources();
        if ff_acquires.is_empty() {
            return None;
        }
        for sdef_idx in def_view.acquired_resources() {
            let spec_kind = ASTAccessSpecifierKind::LegacyAcquires;
            let spec_negated = false;
            let resource_spec = {
                // StructDefinitionIndex -> StructHandleIndex -> StructHandle -> ModuleHandle -> File format ModuleId
                let struct_hidx = ff_module.struct_def_at(*sdef_idx).struct_handle;
                let struct_handle = ff_module.struct_handle_at(struct_hidx);
                let mhandle = ff_module.module_handle_at(struct_handle.module);
                let ff_mid = ff_module.module_id_for_handle(mhandle);
                // StructHandle -> struct name -> StructId in move model
                let sname = ff_module.identifier_at(struct_handle.name);
                let struct_id = StructId::new(self.sym(sname.as_str()));
                // File format ModuleId -> ModuleEnv in move model -> StructEnv in move model
                let mname = self.env.to_module_name(&ff_mid);
                let module_env = self
                    .env
                    .find_module(&mname)
                    .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure));
                let struct_env = self
                    .env
                    .get_struct_opt(module_env.get_id().qualified(struct_id))
                    .unwrap_or_else(|| abort_if_missing!(self.with_dep_closure));
                ASTResourceSpecifier::Resource(
                    module_env
                        .get_id()
                        .qualified_inst(struct_env.get_id(), vec![]),
                )
            };
            let addr_spec = ASTAddressSpecifier::Any;
            let loc = Loc::default();
            let ast_acquire_specifier: ASTAccessSpecifier = ASTAccessSpecifier {
                kind: spec_kind,
                negated: spec_negated,
                resource: (loc.clone(), resource_spec),
                address: (loc.clone(), addr_spec),
                loc,
            };
            access_specifiers.push(ast_acquire_specifier);
        }
        Some(access_specifiers)
    }

    fn ty(&self, sign: &SignatureToken) -> Type {
        let resolver = |module_name, struct_sym| {
            let struct_id = StructId::new(struct_sym);
            if module_name == self.module_name {
                self.module_id.qualified(struct_id)
            } else if let Some(m) = self.env.find_module(&module_name) {
                m.get_id().qualified(struct_id)
            } else {
                self.error(format!(
                    "unresolved module reference `{}`",
                    module_name.display_full(self.env)
                ));
                self.module_id.qualified(struct_id)
            }
        };
        Type::from_signature_token(self.env, self.module.module(), &resolver, sign)
    }

    fn type_param(&self, params: Option<&Vec<SourceName>>, i: usize) -> (Symbol, Loc) {
        let (name, loc) = params
            .and_then(|p| p.get(i))
            .map(|(name, loc)| (name.to_owned(), Some(*loc)))
            .unwrap_or_else(|| (format!("T{}", i), None));
        (self.sym(&name), self.loc(loc))
    }

    fn loc(&self, loc: Option<MoveIrLoc>) -> Loc {
        loc.map(|l| self.env.to_loc(&l))
            .unwrap_or_else(|| self.module_loc.clone())
    }

    fn sym(&self, id: &str) -> Symbol {
        self.env.symbol_pool().make(id)
    }

    fn error(&self, msg: impl AsRef<str>) {
        self.env.error(
            &self.module_loc,
            &format!(
                "while loading binary module `{}`: {}",
                self.module_name.display(self.env),
                msg.as_ref()
            ),
        )
    }
}

fn make_module_name(module: language_storage::ModuleId, spool: &SymbolPool) -> ModuleName {
    ModuleName::new(
        Address::Numerical(module.address),
        spool.make(module.name.as_str()),
    )
}

fn type_params_logical_equal(params1: &[TypeParameter], params2: &[TypeParameter]) -> bool {
    params1.len() == params2.len() && params1.iter().zip(params2).all(|(p1, p2)| p1.1 == p2.1)
}

fn params_logical_equal(params1: &[Parameter], params2: &[Parameter]) -> bool {
    params1.len() == params2.len() && params1.iter().zip(params2).all(|(p1, p2)| p1.1 == p2.1)
}

/// Add well-known functions for vectors to module data. Those functions are instructions in
/// the bytecode and do not have associated handles, but appear as regular functions in
/// stackless bytecode and source.
#[allow(non_snake_case)]
fn add_well_known_vector_funs(env: &GlobalEnv, module_data: &mut ModuleData) {
    // Vector module, add functions which are represented as bytecode
    let t_params = vec![TypeParameter(
        env.symbol_pool().make("T"),
        TypeParameterKind::new(AbilitySet::EMPTY),
        env.unknown_loc(),
    )];
    let t_T = Type::TypeParameter(0);
    let t_ref_T = Type::Reference(ReferenceKind::Immutable, Box::new(t_T.clone()));
    let t_mut_ref_T = Type::Reference(ReferenceKind::Mutable, Box::new(t_T.clone()));
    let t_vec = Type::Vector(Box::new(Type::TypeParameter(0)));
    let t_ref_vec = Type::Reference(ReferenceKind::Immutable, Box::new(t_vec.clone()));
    let t_mut_ref_vec = Type::Reference(ReferenceKind::Mutable, Box::new(t_vec.clone()));
    let t_u64 = Type::new_prim(PrimitiveType::U64);
    let mk_param = |n, t| Parameter(env.symbol_pool().make(n), t, env.unknown_loc());
    add_native_public_fun(
        env,
        module_data,
        "empty",
        t_params.clone(),
        vec![],
        t_vec.clone(),
    );
    add_native_public_fun(
        env,
        module_data,
        "length",
        t_params.clone(),
        vec![mk_param("self", t_ref_vec.clone())],
        t_u64.clone(),
    );
    add_native_public_fun(
        env,
        module_data,
        "borrow",
        t_params.clone(),
        vec![mk_param("self", t_ref_vec), mk_param("i", t_u64.clone())],
        t_ref_T.clone(),
    );
    add_native_public_fun(
        env,
        module_data,
        "borrow_mut",
        t_params.clone(),
        vec![
            mk_param("self", t_mut_ref_vec.clone()),
            mk_param("i", t_u64.clone()),
        ],
        t_mut_ref_T.clone(),
    );
    add_native_public_fun(
        env,
        module_data,
        "push_back",
        t_params.clone(),
        vec![
            mk_param("self", t_mut_ref_vec.clone()),
            mk_param("e", t_T.clone()),
        ],
        Type::unit(),
    );
    add_native_public_fun(
        env,
        module_data,
        "pop_back",
        t_params.clone(),
        vec![mk_param("self", t_mut_ref_vec.clone())],
        t_T.clone(),
    );
    add_native_public_fun(
        env,
        module_data,
        "destroy_empty",
        t_params.clone(),
        vec![mk_param("self", t_vec)],
        Type::unit(),
    );
    add_native_public_fun(
        env,
        module_data,
        "swap",
        t_params.clone(),
        vec![
            mk_param("self", t_mut_ref_vec),
            mk_param("i", t_u64.clone()),
            mk_param("j", t_u64),
        ],
        Type::unit(),
    );
}

fn add_native_public_fun(
    env: &GlobalEnv,
    module_data: &mut ModuleData,
    name: &str,
    type_params: Vec<TypeParameter>,
    params: Vec<Parameter>,
    result_type: Type,
) {
    let sym = env.symbol_pool().make(name);
    module_data
        .function_data
        .insert(FunId::new(sym), FunctionData {
            visibility: Visibility::Public,
            is_native: true,
            type_params,
            params,
            result_type,
            ..FunctionData::new(env.symbol_pool().make(name), env.unknown_loc())
        });
}
