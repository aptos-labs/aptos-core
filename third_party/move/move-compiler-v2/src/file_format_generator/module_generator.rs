// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::file_format_generator::{
    function_generator::FunctionGenerator, MAX_ADDRESS_COUNT, MAX_CONST_COUNT, MAX_FIELD_COUNT,
    MAX_FIELD_INST_COUNT, MAX_FUNCTION_COUNT, MAX_FUNCTION_INST_COUNT, MAX_IDENTIFIER_COUNT,
    MAX_MODULE_COUNT, MAX_SIGNATURE_COUNT, MAX_STRUCT_COUNT, MAX_STRUCT_DEF_COUNT,
    MAX_STRUCT_DEF_INST_COUNT,
};
use codespan_reporting::diagnostic::Severity;
use move_binary_format::{
    file_format as FF,
    file_format::{FunctionHandle, ModuleHandle, TableIndex},
    file_format_common,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_ir_types::ast as IR_AST;
use move_model::{
    ast::{AccessSpecifier, Address, AddressSpecifier, ResourceSpecifier},
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, Parameter, QualifiedId,
        StructEnv, StructId, TypeParameter, TypeParameterKind,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_stackless_bytecode::{
    function_target_pipeline::{FunctionTargetsHolder, FunctionVariant},
    stackless_bytecode::{Bytecode, Constant, Operation},
};
use move_symbol_pool::symbol as IR_SYMBOL;
use std::collections::{BTreeMap, BTreeSet};

/// Internal state of the module code generator
#[derive(Debug)]
pub struct ModuleGenerator {
    /// The module index for which we generate code.
    #[allow(unused)]
    module_idx: FF::ModuleHandleIndex,
    /// A mapping from modules to indices.
    module_to_idx: BTreeMap<ModuleId, FF::ModuleHandleIndex>,
    /// A mapping from symbols to indices.
    name_to_idx: BTreeMap<Symbol, FF::IdentifierIndex>,
    /// A mapping from addresses to indices.
    address_to_idx: BTreeMap<AccountAddress, FF::AddressIdentifierIndex>,
    /// A mapping from functions to indices.
    fun_to_idx: BTreeMap<QualifiedId<FunId>, FF::FunctionHandleIndex>,
    /// The special function handle of the `main` function of a script. This is not stored
    /// in `module.function_handles` because the file format does not maintain a handle
    /// for this function.
    main_handle: Option<FunctionHandle>,
    /// The special module handle for a script, see also `main_handle`.
    script_handle: Option<ModuleHandle>,
    /// A mapping from function instantiations to indices.
    fun_inst_to_idx:
        BTreeMap<(QualifiedId<FunId>, FF::SignatureIndex), FF::FunctionInstantiationIndex>,
    /// A mapping from structs to indices.
    struct_to_idx: BTreeMap<QualifiedId<StructId>, FF::StructHandleIndex>,
    /// A mapping from function instantiations to indices.
    struct_def_inst_to_idx:
        BTreeMap<(QualifiedId<StructId>, FF::SignatureIndex), FF::StructDefInstantiationIndex>,
    /// A mapping from fields to indices.
    field_to_idx: BTreeMap<(QualifiedId<StructId>, usize), FF::FieldHandleIndex>,
    /// A mapping from fields to indices.
    field_inst_to_idx:
        BTreeMap<(QualifiedId<StructId>, usize, FF::SignatureIndex), FF::FieldInstantiationIndex>,
    /// A mapping from type sequences to signature indices.
    types_to_signature: BTreeMap<Vec<Type>, FF::SignatureIndex>,
    /// A mapping from constants sequences to pool indices.
    cons_to_idx: BTreeMap<Constant, FF::ConstantPoolIndex>,
    /// The file-format module we are building.
    pub module: FF::CompiledModule,
    /// The source map for the module.
    pub source_map: SourceMap,
}

/// Immutable context for a module code generation, separated from the mutable generator
/// state to reduce borrow conflicts.
#[derive(Debug, Clone)]
pub struct ModuleContext<'env> {
    /// The global model.
    pub env: &'env GlobalEnv,
    /// A holder for function target data, containing stackless bytecode.
    pub targets: &'env FunctionTargetsHolder,
}

impl ModuleGenerator {
    /// Runs generation of `CompiledModule`.
    pub fn run(
        ctx: &ModuleContext,
        module_env: &ModuleEnv,
    ) -> (FF::CompiledModule, SourceMap, Option<FF::FunctionHandle>) {
        let module = move_binary_format::CompiledModule {
            version: file_format_common::VERSION_NEXT,
            self_module_handle_idx: FF::ModuleHandleIndex(0),
            ..Default::default()
        };
        let source_map = {
            let module_name_opt = if module_env.is_script_module() {
                None
            } else {
                let name = IR_AST::ModuleName(IR_SYMBOL::Symbol::from(
                    ctx.symbol_to_str(module_env.get_name().name()),
                ));
                Some(IR_AST::ModuleIdent::new(
                    name,
                    module_env.get_name().addr().expect_numerical(),
                ))
            };
            SourceMap::new(ctx.env.to_ir_loc(&module_env.get_loc()), module_name_opt)
        };
        let mut gen = Self {
            module_idx: FF::ModuleHandleIndex(0),
            module_to_idx: Default::default(),
            name_to_idx: Default::default(),
            address_to_idx: Default::default(),
            fun_to_idx: Default::default(),
            struct_to_idx: Default::default(),
            struct_def_inst_to_idx: Default::default(),
            field_to_idx: Default::default(),
            field_inst_to_idx: Default::default(),
            types_to_signature: Default::default(),
            cons_to_idx: Default::default(),
            fun_inst_to_idx: Default::default(),
            main_handle: None,
            script_handle: None,
            module,
            source_map,
        };
        gen.gen_module(ctx, module_env);
        (gen.module, gen.source_map, gen.main_handle)
    }

    /// Generates a module, visiting all of its members.
    fn gen_module(&mut self, ctx: &ModuleContext, module_env: &ModuleEnv<'_>) {
        // Create the self module handle, at well known handle index 0, but only if this is not
        // a script module.
        if !module_env.is_script_module() {
            let loc = &module_env.get_loc();
            self.module_index(ctx, loc, module_env);
        }

        for struct_env in module_env.get_structs() {
            self.gen_struct(ctx, &struct_env)
        }

        let acquires_map = ctx.generate_acquires_map(module_env);
        for fun_env in module_env.get_functions() {
            let acquires_list = &acquires_map[&fun_env.get_id()];
            FunctionGenerator::run(self, ctx, fun_env, acquires_list);
        }

        // At handles of friend modules
        for mid in module_env.get_friend_modules() {
            let handle = self.module_handle(ctx, &module_env.get_loc(), &ctx.env.get_module(mid));
            self.module.friend_decls.push(handle)
        }
    }

    /// Generate information for a struct.
    fn gen_struct(&mut self, ctx: &ModuleContext, struct_env: &StructEnv<'_>) {
        if struct_env.is_ghost_memory() {
            return;
        }
        let loc = &struct_env.get_loc();
        let struct_handle = self.struct_index(ctx, loc, struct_env);
        let field_information = FF::StructFieldInformation::Declared(
            struct_env
                .get_fields()
                .map(|f| {
                    let name = self.name_index(ctx, loc, f.get_name());
                    let signature =
                        FF::TypeSignature(self.signature_token(ctx, loc, &f.get_type()));
                    FF::FieldDefinition { name, signature }
                })
                .collect(),
        );
        let def = FF::StructDefinition {
            struct_handle,
            field_information,
        };
        ctx.checked_bound(
            loc,
            self.module.struct_defs.len(),
            MAX_STRUCT_DEF_COUNT,
            "struct",
        );
        self.module.struct_defs.push(def);
    }

    /// Obtains or creates an index for a signature, a sequence of types.
    pub fn signature(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        tys: Vec<Type>,
    ) -> FF::SignatureIndex {
        if let Some(idx) = self.types_to_signature.get(&tys) {
            return *idx;
        }
        let tokens = tys
            .iter()
            .map(|ty| self.signature_token(ctx, loc, ty))
            .collect::<Vec<_>>();
        let idx = FF::SignatureIndex(ctx.checked_bound(
            loc,
            self.module.signatures.len(),
            MAX_SIGNATURE_COUNT,
            "signature",
        ));
        self.module.signatures.push(FF::Signature(tokens));
        self.types_to_signature.insert(tys, idx);
        idx
    }

    /// Creates a signature token from a Move model type.
    pub fn signature_token(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        ty: &Type,
    ) -> FF::SignatureToken {
        use PrimitiveType::*;
        use Type::*;
        match ty {
            Primitive(kind) => match kind {
                Bool => FF::SignatureToken::Bool,
                U8 => FF::SignatureToken::U8,
                U16 => FF::SignatureToken::U16,
                U32 => FF::SignatureToken::U32,
                U64 => FF::SignatureToken::U64,
                U128 => FF::SignatureToken::U128,
                U256 => FF::SignatureToken::U256,
                Address => FF::SignatureToken::Address,
                Signer => FF::SignatureToken::Signer,
                Num | Range | EventStore => {
                    ctx.internal_error(loc, format!("unexpected specification type {:#?}", ty));
                    FF::SignatureToken::Bool
                },
            },
            Tuple(_) => {
                ctx.internal_error(loc, format!("unexpected tuple type {:#?}", ty));
                FF::SignatureToken::Bool
            },
            Vector(ty) => FF::SignatureToken::Vector(Box::new(self.signature_token(ctx, loc, ty))),
            Struct(mid, sid, inst) => {
                let handle = self.struct_index(ctx, loc, &ctx.env.get_struct(mid.qualified(*sid)));
                if inst.is_empty() {
                    FF::SignatureToken::Struct(handle)
                } else {
                    FF::SignatureToken::StructInstantiation(
                        handle,
                        inst.iter()
                            .map(|t| self.signature_token(ctx, loc, t))
                            .collect(),
                    )
                }
            },
            TypeParameter(p) => FF::SignatureToken::TypeParameter(*p),
            Reference(kind, target_ty) => {
                let target_ty = Box::new(self.signature_token(ctx, loc, target_ty));
                match kind {
                    ReferenceKind::Immutable => FF::SignatureToken::Reference(target_ty),
                    ReferenceKind::Mutable => FF::SignatureToken::MutableReference(target_ty),
                }
            },
            Fun(_, _) | TypeDomain(_) | ResourceDomain(_, _, _) | Error | Var(_) => {
                ctx.internal_error(
                    loc,
                    format!(
                        "unexpected type: {}",
                        ty.display(&ctx.env.get_type_display_ctx())
                    ),
                );
                FF::SignatureToken::Bool
            },
        }
    }

    /// Obtains or generates an identifier index for the given symbol.
    pub fn name_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        name: Symbol,
    ) -> FF::IdentifierIndex {
        if let Some(idx) = self.name_to_idx.get(&name) {
            return *idx;
        }
        let ident =
            if let Ok(ident) = Identifier::new(name.display(ctx.env.symbol_pool()).to_string()) {
                ident
            } else {
                ctx.internal_error(
                    loc,
                    format!("invalid identifier {}", name.display(ctx.env.symbol_pool())),
                );
                Identifier::new("error").unwrap()
            };
        let idx = FF::IdentifierIndex(ctx.checked_bound(
            loc,
            self.module.identifiers.len(),
            MAX_IDENTIFIER_COUNT,
            "identifier",
        ));
        self.module.identifiers.push(ident);
        self.name_to_idx.insert(name, idx);
        idx
    }

    /// Obtains or generates an identifier index for the given symbol.
    pub fn address_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        addr: AccountAddress,
    ) -> FF::AddressIdentifierIndex {
        if let Some(idx) = self.address_to_idx.get(&addr) {
            return *idx;
        }
        let idx = FF::AddressIdentifierIndex(ctx.checked_bound(
            loc,
            self.module.address_identifiers.len(),
            MAX_ADDRESS_COUNT,
            "address",
        ));
        self.module.address_identifiers.push(addr);
        self.address_to_idx.insert(addr, idx);
        idx
    }

    // Obtains or generates a module index.
    pub fn module_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        module_env: &ModuleEnv,
    ) -> FF::ModuleHandleIndex {
        let id = module_env.get_id();
        if let Some(idx) = self.module_to_idx.get(&id) {
            return *idx;
        }
        let handle = self.module_handle(ctx, loc, module_env);
        let idx = if module_env.is_script_module() {
            self.script_handle = Some(handle);
            FF::ModuleHandleIndex(TableIndex::MAX)
        } else {
            let idx = FF::ModuleHandleIndex(ctx.checked_bound(
                loc,
                self.module.module_handles.len(),
                MAX_MODULE_COUNT,
                "used module",
            ));
            self.module.module_handles.push(handle);
            idx
        };
        self.module_to_idx.insert(id, idx);
        idx
    }

    fn module_handle(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        module_env: &ModuleEnv,
    ) -> ModuleHandle {
        let name = module_env.get_name();
        let address = self.address_index(ctx, loc, name.addr().expect_numerical());
        let name = self.name_index(ctx, loc, name.name());
        FF::ModuleHandle { address, name }
    }

    /// Obtains or generates a function index.
    pub fn function_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        fun_env: &FunctionEnv,
    ) -> FF::FunctionHandleIndex {
        if let Some(idx) = self.fun_to_idx.get(&fun_env.get_qualified_id()) {
            return *idx;
        }
        let module = self.module_index(ctx, loc, &fun_env.module_env);
        let name = self.name_index(ctx, loc, fun_env.get_name());
        let type_parameters = fun_env
            .get_type_parameters()
            .into_iter()
            .map(|TypeParameter(_, TypeParameterKind { abilities, .. }, _)| abilities)
            .collect::<Vec<_>>();
        let parameters = self.signature(
            ctx,
            loc,
            fun_env
                .get_parameters()
                .iter()
                .map(|Parameter(_, ty, _)| ty.to_owned())
                .collect(),
        );
        let return_ = self.signature(
            ctx,
            loc,
            fun_env.get_result_type().flatten().into_iter().collect(),
        );
        let access_specifiers = fun_env.get_access_specifiers().as_ref().map(|v| {
            v.iter()
                .map(|s| self.access_specifier(ctx, fun_env, s))
                .collect()
        });
        let handle = FF::FunctionHandle {
            module,
            name,
            type_parameters,
            parameters,
            return_,
            access_specifiers,
        };
        let idx = if fun_env.module_env.is_script_module() {
            self.main_handle = Some(handle);
            FF::FunctionHandleIndex(TableIndex::MAX)
        } else {
            let idx = FF::FunctionHandleIndex(ctx.checked_bound(
                loc,
                self.module.function_handles.len(),
                MAX_FUNCTION_COUNT,
                "used function",
            ));
            self.module.function_handles.push(handle);
            idx
        };
        self.fun_to_idx.insert(fun_env.get_qualified_id(), idx);
        idx
    }

    pub fn access_specifier(
        &mut self,
        ctx: &ModuleContext,
        fun_env: &FunctionEnv,
        access_specifier: &AccessSpecifier,
    ) -> FF::AccessSpecifier {
        let resource = match &access_specifier.resource.1 {
            ResourceSpecifier::Any => FF::ResourceSpecifier::Any,
            ResourceSpecifier::DeclaredAtAddress(addr) => FF::ResourceSpecifier::DeclaredAtAddress(
                self.address_index(ctx, &access_specifier.resource.0, addr.expect_numerical()),
            ),
            ResourceSpecifier::DeclaredInModule(module_id) => {
                FF::ResourceSpecifier::DeclaredInModule(self.module_index(
                    ctx,
                    &access_specifier.resource.0,
                    &ctx.env.get_module(*module_id),
                ))
            },
            ResourceSpecifier::Resource(struct_id) => {
                let struct_env = ctx.env.get_struct(struct_id.to_qualified_id());
                if struct_id.inst.is_empty() {
                    FF::ResourceSpecifier::Resource(self.struct_index(
                        ctx,
                        &access_specifier.loc,
                        &struct_env,
                    ))
                } else {
                    FF::ResourceSpecifier::ResourceInstantiation(
                        self.struct_index(ctx, &access_specifier.loc, &struct_env),
                        self.signature(ctx, &access_specifier.loc, struct_id.inst.to_vec()),
                    )
                }
            },
        };
        let address =
            match &access_specifier.address.1 {
                AddressSpecifier::Any => FF::AddressSpecifier::Any,
                AddressSpecifier::Address(addr) => FF::AddressSpecifier::Literal(
                    self.address_index(ctx, &access_specifier.address.0, addr.expect_numerical()),
                ),
                AddressSpecifier::Parameter(name) => {
                    let param_index = fun_env
                        .get_parameters()
                        .iter()
                        .position(|Parameter(n, _ty, _)| n == name)
                        .expect("parameter defined") as u8;
                    FF::AddressSpecifier::Parameter(param_index, None)
                },
                AddressSpecifier::Call(fun, name) => {
                    let param_index = fun_env
                        .get_parameters()
                        .iter()
                        .position(|Parameter(n, _ty, _)| n == name)
                        .expect("parameter defined") as u8;
                    let fun_index = self.function_instantiation_index(
                        ctx,
                        &access_specifier.address.0,
                        &ctx.env.get_function(fun.to_qualified_id()),
                        fun.inst.clone(),
                    );
                    FF::AddressSpecifier::Parameter(param_index, Some(fun_index))
                },
            };
        FF::AccessSpecifier {
            kind: access_specifier.kind,
            negated: access_specifier.negated,
            resource,
            address,
        }
    }

    pub fn function_instantiation_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        fun_env: &FunctionEnv<'_>,
        inst: Vec<Type>,
    ) -> FF::FunctionInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let cache_key = (fun_env.get_qualified_id(), type_parameters);
        if let Some(idx) = self.fun_inst_to_idx.get(&cache_key) {
            return *idx;
        }
        let handle = self.function_index(ctx, loc, fun_env);
        let fun_inst = FF::FunctionInstantiation {
            handle,
            type_parameters,
        };
        let idx = FF::FunctionInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.function_instantiations.len(),
            MAX_FUNCTION_INST_COUNT,
            "function instantiation",
        ));
        self.module.function_instantiations.push(fun_inst);
        self.fun_inst_to_idx.insert(cache_key, idx);
        idx
    }

    /// Obtains or generates a struct index.
    pub fn struct_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv<'_>,
    ) -> FF::StructHandleIndex {
        if let Some(idx) = self.struct_to_idx.get(&struct_env.get_qualified_id()) {
            return *idx;
        }
        let name = self.name_index(ctx, loc, struct_env.get_name());
        let module = self.module_index(ctx, loc, &struct_env.module_env);
        let handle = FF::StructHandle {
            module,
            name,
            abilities: struct_env.get_abilities(),
            type_parameters: struct_env
                .get_type_parameters()
                .iter()
                .map(
                    |TypeParameter(
                        _sym,
                        TypeParameterKind {
                            abilities,
                            is_phantom,
                        },
                        _loc,
                    )| FF::StructTypeParameter {
                        constraints: *abilities,
                        is_phantom: *is_phantom,
                        // TODO: use _loc here?
                    },
                )
                .collect(),
        };
        let idx = FF::StructHandleIndex(ctx.checked_bound(
            loc,
            self.module.struct_handles.len(),
            MAX_STRUCT_COUNT,
            "used structs",
        ));
        self.module.struct_handles.push(handle);
        self.struct_to_idx
            .insert(struct_env.get_qualified_id(), idx);
        idx
    }

    /// Obtains a struct definition index.
    pub fn struct_def_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
    ) -> FF::StructDefinitionIndex {
        let struct_idx = self.struct_index(ctx, loc, struct_env);
        for (i, def) in self.module.struct_defs.iter().enumerate() {
            if def.struct_handle == struct_idx {
                return FF::StructDefinitionIndex::new(i as FF::TableIndex);
            }
        }
        ctx.internal_error(loc, "struct not defined");
        FF::StructDefinitionIndex(0)
    }

    /// Obtains or constructs a struct definition instantiation index.
    pub fn struct_def_instantiation_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
        inst: Vec<Type>,
    ) -> FF::StructDefInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let cache_key = (struct_env.get_qualified_id(), type_parameters);
        if let Some(idx) = self.struct_def_inst_to_idx.get(&cache_key) {
            return *idx;
        }
        let def = self.struct_def_index(ctx, loc, struct_env);
        let struct_inst = FF::StructDefInstantiation {
            def,
            type_parameters,
        };
        let idx = FF::StructDefInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.struct_def_instantiations.len(),
            MAX_STRUCT_DEF_INST_COUNT,
            "struct instantiation",
        ));
        self.module.struct_def_instantiations.push(struct_inst);
        self.struct_def_inst_to_idx.insert(cache_key, idx);
        idx
    }

    /// Obtains or creates a field handle index.
    pub fn field_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        field_env: &FieldEnv,
    ) -> FF::FieldHandleIndex {
        let key = (
            field_env.struct_env.get_qualified_id(),
            field_env.get_offset(),
        );
        if let Some(idx) = self.field_to_idx.get(&key) {
            return *idx;
        }
        let field_idx = FF::FieldHandleIndex(ctx.checked_bound(
            loc,
            self.module.field_handles.len(),
            MAX_FIELD_COUNT,
            "field",
        ));
        let owner = self.struct_def_index(ctx, loc, &field_env.struct_env);
        self.module.field_handles.push(FF::FieldHandle {
            owner,
            field: field_env.get_offset() as FF::MemberCount,
        });
        self.field_to_idx.insert(key, field_idx);
        field_idx
    }

    /// Obtains or creates a field instantiation handle index.
    pub fn field_inst_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        field_env: &FieldEnv,
        inst: Vec<Type>,
    ) -> FF::FieldInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let key = (
            field_env.struct_env.get_qualified_id(),
            field_env.get_offset(),
            type_parameters,
        );
        if let Some(idx) = self.field_inst_to_idx.get(&key) {
            return *idx;
        }
        let field_inst_idx = FF::FieldInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.field_instantiations.len(),
            MAX_FIELD_INST_COUNT,
            "field instantiation",
        ));
        let handle = self.field_index(ctx, loc, field_env);
        self.module
            .field_instantiations
            .push(FF::FieldInstantiation {
                handle,
                type_parameters,
            });
        self.field_inst_to_idx.insert(key, field_inst_idx);
        field_inst_idx
    }

    /// Obtains or generates a constant index.
    pub fn constant_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        cons: &Constant,
        ty: &Type,
    ) -> FF::ConstantPoolIndex {
        if let Some(idx) = self.cons_to_idx.get(cons) {
            return *idx;
        }
        let data = cons
            .to_move_value()
            .simple_serialize()
            .expect("serialization succeeds");
        let ff_cons = FF::Constant {
            type_: self.signature_token(ctx, loc, ty),
            data,
        };
        let idx = FF::ConstantPoolIndex(ctx.checked_bound(
            loc,
            self.module.constant_pool.len(),
            MAX_CONST_COUNT,
            "constant",
        ));
        self.module.constant_pool.push(ff_cons);
        self.cons_to_idx.insert(cons.clone(), idx);
        idx
    }
}

impl<'env> ModuleContext<'env> {
    /// Emits an error at the location.
    pub fn error(&self, loc: impl AsRef<Loc>, msg: impl AsRef<str>) {
        self.env.diag(Severity::Error, loc.as_ref(), msg.as_ref())
    }

    /// Emits an internal error at the location.
    pub fn internal_error(&self, loc: impl AsRef<Loc>, msg: impl AsRef<str>) {
        self.env.diag(Severity::Bug, loc.as_ref(), msg.as_ref())
    }

    /// Check for a bound table index and report an error if its out of bound. All bounds
    /// should be handled by this generator in a graceful and giving the user detail
    /// information of the location.
    pub fn checked_bound(
        &self,
        loc: impl AsRef<Loc>,
        value: usize,
        max: usize,
        msg: &str,
    ) -> FF::TableIndex {
        if value >= max {
            self.error(loc, format!("exceeded maximal {} count: {}", msg, max));
            0
        } else {
            value as FF::TableIndex
        }
    }

    /// Get the file format opcode for a well-known function. This applies currently to a set
    /// vector functions which have builtin opcodes. Gets passed an optional type instantiation
    /// in form of a signature.
    pub fn get_well_known_function_code(
        &self,
        loc: &Loc,
        qid: QualifiedId<FunId>,
        inst_sign: Option<FF::SignatureIndex>,
    ) -> Option<FF::Bytecode> {
        let fun = self.env.get_function(qid);
        let mod_name = fun.module_env.get_name();
        if mod_name.addr() != &Address::Numerical(AccountAddress::ONE) {
            return None;
        }
        let pool = self.env.symbol_pool();
        if pool.string(mod_name.name()).as_str() == "vector" {
            if let Some(inst) = inst_sign {
                match pool.string(fun.get_name()).as_str() {
                    "empty" => Some(FF::Bytecode::VecPack(inst, 0)),
                    "length" => Some(FF::Bytecode::VecLen(inst)),
                    "borrow" => Some(FF::Bytecode::VecImmBorrow(inst)),
                    "borrow_mut" => Some(FF::Bytecode::VecMutBorrow(inst)),
                    "push_back" => Some(FF::Bytecode::VecPushBack(inst)),
                    "pop_back" => Some(FF::Bytecode::VecPopBack(inst)),
                    "destroy_empty" => Some(FF::Bytecode::VecUnpack(inst, 0)),
                    "swap" => Some(FF::Bytecode::VecSwap(inst)),
                    _ => None,
                }
            } else {
                self.internal_error(loc, "expected type instantiation for vector operation");
                None
            }
        } else {
            None
        }
    }

    /// Convert the symbol into a string.
    pub fn symbol_to_str(&self, s: Symbol) -> String {
        s.display(self.env.symbol_pool()).to_string()
    }
}

impl<'env> ModuleContext<'env> {
    /// Acquires analysis. This is temporary until we have the full reference analysis.
    fn generate_acquires_map(&self, module: &ModuleEnv) -> BTreeMap<FunId, BTreeSet<StructId>> {
        // Compute map with direct usage of resources
        let mut usage_map = module
            .get_functions()
            .map(|f| (f.get_id(), self.get_direct_function_acquires(&f)))
            .collect::<BTreeMap<_, _>>();
        // Now run a fixed-point loop: add resources used by called functions until there are no
        // changes.
        loop {
            let mut changes = false;
            for fun in module.get_functions() {
                if let Some(callees) = fun.get_called_functions() {
                    let mut usage = usage_map[&fun.get_id()].clone();
                    let count = usage.len();
                    // Extend usage by that of callees from the same module. Acquires is only
                    // local to a module.
                    for callee in callees {
                        if callee.module_id == module.get_id() {
                            usage.extend(usage_map[&callee.id].iter().cloned());
                        }
                    }
                    if usage.len() > count {
                        *usage_map.get_mut(&fun.get_id()).unwrap() = usage;
                        changes = true;
                    }
                }
            }
            if !changes {
                break;
            }
        }
        usage_map
    }

    fn get_direct_function_acquires(&self, fun: &FunctionEnv) -> BTreeSet<StructId> {
        let mut result = BTreeSet::new();
        let target = self.targets.get_target(fun, &FunctionVariant::Baseline);
        for bc in target.get_bytecode() {
            use Bytecode::*;
            use Operation::*;
            match bc {
                Call(_, _, MoveFrom(mid, sid, ..), ..)
                | Call(_, _, BorrowGlobal(mid, sid, ..), ..)
                // GetGlobal from spec language, but cover it anyway.
                | Call(_, _, GetGlobal(mid, sid, ..), ..)
                if *mid == fun.module_env.get_id() =>
                    {
                        result.insert(*sid);
                    },
                _ => {},
            }
        }
        result
    }
}
