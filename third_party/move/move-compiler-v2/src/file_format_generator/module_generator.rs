// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    file_format_generator::{
        function_generator::FunctionGenerator, MAX_ADDRESS_COUNT, MAX_CONST_COUNT, MAX_FIELD_COUNT,
        MAX_FIELD_INST_COUNT, MAX_FUNCTION_COUNT, MAX_FUNCTION_INST_COUNT, MAX_IDENTIFIER_COUNT,
        MAX_MODULE_COUNT, MAX_SIGNATURE_COUNT, MAX_STRUCT_COUNT, MAX_STRUCT_DEF_COUNT,
        MAX_STRUCT_DEF_INST_COUNT, MAX_STRUCT_VARIANT_COUNT, MAX_STRUCT_VARIANT_INST_COUNT,
    },
    Options, COMPILER_BUG_REPORT_MSG,
};
use codespan_reporting::diagnostic::Severity;
use itertools::Itertools;
use move_binary_format::{file_format as FF, file_format::Visibility, file_format_common};
use move_bytecode_source_map::source_map::{SourceMap, SourceName};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, metadata::Metadata,
};
use move_ir_types::ast as IR_AST;
use move_model::{
    ast::{AccessSpecifier, AccessSpecifierKind, AddressSpecifier, Attribute, ResourceSpecifier},
    metadata::{CompilationMetadata, CompilerVersion, LanguageVersion, COMPILATION_METADATA_KEY},
    model::{
        FieldEnv, FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, ModuleId, Parameter, QualifiedId,
        StructEnv, StructId, TypeParameter, TypeParameterKind,
    },
    symbol::Symbol,
    ty::{PrimitiveType, ReferenceKind, Type},
    well_known,
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
    /// Whether to generate access specifiers
    gen_access_specifiers: bool,
    /// Whether to generate function attributes.
    pub(crate) gen_function_attributes: bool,
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
    main_handle: Option<FF::FunctionHandle>,
    /// The special module handle for a script, see also `main_handle`.
    script_handle: Option<FF::ModuleHandle>,
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
    /// A mapping from constants sequences (with the corresponding type information) to pool indices.
    cons_to_idx: BTreeMap<(Constant, Type), FF::ConstantPoolIndex>,
    variant_field_to_idx:
        BTreeMap<(QualifiedId<StructId>, Vec<Symbol>, usize), FF::VariantFieldHandleIndex>,
    variant_field_inst_to_idx: BTreeMap<
        (
            QualifiedId<StructId>,
            Vec<Symbol>,
            usize,
            FF::SignatureIndex,
        ),
        FF::VariantFieldInstantiationIndex,
    >,
    struct_variant_to_idx: BTreeMap<(QualifiedId<StructId>, Symbol), FF::StructVariantHandleIndex>,
    struct_variant_inst_to_idx: BTreeMap<
        (QualifiedId<StructId>, Symbol, FF::SignatureIndex),
        FF::StructVariantInstantiationIndex,
    >,
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

/// Source map operations deliver Result but are really not expected to fail.
/// The below message is used if they do anyway.
pub(crate) const SOURCE_MAP_OK: &str = "expected valid source map";

impl ModuleGenerator {
    /// Runs generation of `CompiledModule`.
    pub fn run(
        ctx: &ModuleContext,
        module_env: &ModuleEnv,
    ) -> (FF::CompiledModule, SourceMap, Option<FF::FunctionHandle>) {
        let options = module_env.env.get_extension::<Options>().expect("options");
        let language_version = options.language_version.unwrap_or_default();
        let compiler_version = options
            .compiler_version
            .unwrap_or(CompilerVersion::latest_stable());
        let gen_access_specifiers = language_version.is_at_least(LanguageVersion::V2_3);
        let gen_function_attributes = language_version.is_at_least(LanguageVersion::V2_2);
        let compilation_metadata = CompilationMetadata::new(compiler_version, language_version);
        let metadata = Metadata {
            key: COMPILATION_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&compilation_metadata)
                .expect("Serialization of CompilationMetadata should succeed"),
        };
        let module = move_binary_format::CompiledModule {
            version: file_format_common::VERSION_MAX,
            self_module_handle_idx: FF::ModuleHandleIndex(0),
            metadata: vec![metadata],
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
            gen_access_specifiers,
            gen_function_attributes,
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
            variant_field_to_idx: Default::default(),
            variant_field_inst_to_idx: Default::default(),
            struct_variant_to_idx: Default::default(),
            struct_variant_inst_to_idx: Default::default(),
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

        let options = ctx
            .env
            .get_extension::<Options>()
            .expect("Options is available");
        let compile_test_code = options.compile_test_code;

        for struct_env in module_env.get_structs() {
            assert!(compile_test_code || !struct_env.is_test_only());
            self.gen_struct(ctx, &struct_env)
        }

        let acquires_map = ctx.generate_acquires_map(module_env);
        for fun_env in module_env.get_functions() {
            // Do not need to generate code for inline functions
            if fun_env.is_inline() {
                continue;
            }
            assert!(compile_test_code || !fun_env.is_test_only());
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
        let def_idx = FF::StructDefinitionIndex::new(ctx.checked_bound(
            loc,
            self.module.struct_defs.len(),
            MAX_STRUCT_DEF_COUNT,
            "struct",
        ));
        self.source_map
            .add_top_level_struct_mapping(def_idx, ctx.env.to_ir_loc(loc))
            .expect(SOURCE_MAP_OK);
        for TypeParameter(name, _, loc) in struct_env.get_type_parameters() {
            self.source_map
                .add_struct_type_parameter_mapping(def_idx, ctx.source_name(name, loc))
                .expect(SOURCE_MAP_OK);
        }
        let struct_handle = self.struct_index(ctx, loc, struct_env);
        let field_information = if struct_env.has_variants() {
            let variants = struct_env
                .get_variants()
                .map(|v| FF::VariantDefinition {
                    name: self.name_index(ctx, struct_env.get_variant_loc(v), v),
                    fields: struct_env
                        .get_fields_of_variant(v)
                        .map(|f| self.field(ctx, def_idx, &f))
                        .collect_vec(),
                })
                .collect_vec();
            FF::StructFieldInformation::DeclaredVariants(variants)
        } else if struct_env.is_native() {
            FF::StructFieldInformation::Native
        } else {
            let fields = struct_env.get_fields();
            FF::StructFieldInformation::Declared(
                fields.map(|f| self.field(ctx, def_idx, &f)).collect(),
            )
        };
        let def = FF::StructDefinition {
            struct_handle,
            field_information,
        };
        self.module.struct_defs.push(def)
    }

    fn field(
        &mut self,
        ctx: &ModuleContext,
        struct_def_idx: FF::StructDefinitionIndex,
        field_env: &FieldEnv,
    ) -> FF::FieldDefinition {
        let field_loc = field_env.get_loc();
        let variant_idx = field_env
            .get_variant()
            .and_then(|v| field_env.struct_env.get_variant_idx(v));
        self.source_map
            .add_struct_field_mapping(struct_def_idx, variant_idx, ctx.env.to_ir_loc(field_loc))
            .expect(SOURCE_MAP_OK);
        let mut field_symbol = field_env.get_name();
        let field_name = ctx.symbol_to_str(field_symbol);
        // Append `_` if this is a positional field (digits), as the binary format expects proper identifiers
        if field_name.starts_with(|c: char| c.is_ascii_digit()) {
            field_symbol = ctx.env.symbol_pool().make(&format!("_{}", field_name));
        }
        let name = self.name_index(ctx, field_loc, field_symbol);
        let signature =
            FF::TypeSignature(self.signature_token(ctx, field_loc, &field_env.get_type()));
        FF::FieldDefinition { name, signature }
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
                I64 => FF::SignatureToken::I64,
                I128 => FF::SignatureToken::I128,
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
            Fun(param_ty, result_ty, abilities) => {
                let list = |gen: &mut ModuleGenerator, ts: Vec<Type>| {
                    ts.into_iter()
                        .map(|t| gen.signature_token(ctx, loc, &t))
                        .collect_vec()
                };
                FF::SignatureToken::Function(
                    list(self, param_ty.clone().flatten()),
                    list(self, result_ty.clone().flatten()),
                    *abilities,
                )
            },
            TypeDomain(_) | ResourceDomain(_, _, _) | Error | Var(_) => {
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
            FF::ModuleHandleIndex(FF::TableIndex::MAX)
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
    ) -> FF::ModuleHandle {
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
        let access_specifiers = fun_env
            .get_access_specifiers()
            .as_ref()
            .map(|v| {
                v.iter()
                    .filter_map(|s| self.access_specifier(ctx, fun_env, s))
                    .collect_vec()
            })
            .and_then(|specs| if specs.is_empty() { None } else { Some(specs) });
        if !self.gen_access_specifiers && access_specifiers.is_some() {
            ctx.error(loc, "access specifiers not enabled");
        }
        let attributes = if self.gen_function_attributes {
            ctx.function_attributes(fun_env)
        } else {
            vec![]
        };
        let handle = FF::FunctionHandle {
            module,
            name,
            type_parameters,
            parameters,
            return_,
            access_specifiers,
            attributes,
        };
        let idx = if fun_env.module_env.is_script_module() {
            self.main_handle = Some(handle);
            FF::FunctionHandleIndex(FF::TableIndex::MAX)
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
    ) -> Option<FF::AccessSpecifier> {
        let kind = match access_specifier.kind {
            AccessSpecifierKind::Reads => FF::AccessKind::Reads,
            AccessSpecifierKind::Writes => FF::AccessKind::Writes,
            AccessSpecifierKind::LegacyAcquires => {
                // Legacy acquires not represented in file format
                return None;
            },
        };
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
        Some(FF::AccessSpecifier {
            kind,
            negated: access_specifier.negated,
            resource,
            address,
        })
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

    /// Obtains or creates a variant field handle index.
    pub fn variant_field_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        variants: &[Symbol],
        field_env: &FieldEnv,
    ) -> FF::VariantFieldHandleIndex {
        let key = (
            field_env.struct_env.get_qualified_id(),
            variants.to_vec(),
            field_env.get_offset(),
        );
        if let Some(idx) = self.variant_field_to_idx.get(&key) {
            return *idx;
        }
        let field_idx = FF::VariantFieldHandleIndex(ctx.checked_bound(
            loc,
            self.module.variant_field_handles.len(),
            MAX_FIELD_COUNT,
            "variant field",
        ));
        let variant_offsets = variants
            .iter()
            .filter_map(|v| field_env.struct_env.get_variant_idx(*v))
            .collect_vec();
        let owner = self.struct_def_index(ctx, loc, &field_env.struct_env);
        self.module
            .variant_field_handles
            .push(FF::VariantFieldHandle {
                struct_index: owner,
                variants: variant_offsets,
                field: field_env.get_offset() as FF::MemberCount,
            });
        self.variant_field_to_idx.insert(key, field_idx);
        field_idx
    }

    /// Obtains or creates a variant field instantiation handle index.
    pub fn variant_field_inst_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        variants: &[Symbol],
        field_env: &FieldEnv,
        inst: Vec<Type>,
    ) -> FF::VariantFieldInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let key = (
            field_env.struct_env.get_qualified_id(),
            variants.to_vec(),
            field_env.get_offset(),
            type_parameters,
        );
        if let Some(idx) = self.variant_field_inst_to_idx.get(&key) {
            return *idx;
        }
        let idx = FF::VariantFieldInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.variant_field_instantiations.len(),
            MAX_FIELD_INST_COUNT,
            "variant field instantiation",
        ));
        let handle = self.variant_field_index(ctx, loc, variants, field_env);
        self.module
            .variant_field_instantiations
            .push(FF::VariantFieldInstantiation {
                handle,
                type_parameters,
            });
        self.variant_field_inst_to_idx.insert(key, idx);
        idx
    }

    /// Obtains or creates a struct variant handle index.
    pub fn struct_variant_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
        variant: Symbol,
    ) -> FF::StructVariantHandleIndex {
        let key = (struct_env.get_qualified_id(), variant);
        if let Some(idx) = self.struct_variant_to_idx.get(&key) {
            return *idx;
        }
        let idx = FF::StructVariantHandleIndex(ctx.checked_bound(
            loc,
            self.module.struct_variant_handles.len(),
            MAX_STRUCT_VARIANT_COUNT,
            "struct variant",
        ));
        let struct_index = self.struct_def_index(ctx, loc, struct_env);
        self.module
            .struct_variant_handles
            .push(FF::StructVariantHandle {
                struct_index,
                variant: struct_env.get_variant_idx(variant).expect("variant idx"),
            });
        self.struct_variant_to_idx.insert(key, idx);
        idx
    }

    /// Obtains or creates a struct variant instantiation index.
    pub fn struct_variant_inst_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
        variant: Symbol,
        inst: Vec<Type>,
    ) -> FF::StructVariantInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let key = (struct_env.get_qualified_id(), variant, type_parameters);
        if let Some(idx) = self.struct_variant_inst_to_idx.get(&key) {
            return *idx;
        }
        let idx = FF::StructVariantInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.struct_variant_instantiations.len(),
            MAX_STRUCT_VARIANT_INST_COUNT,
            "struct variant instantiation",
        ));
        let handle = self.struct_variant_index(ctx, loc, struct_env, variant);
        self.module
            .struct_variant_instantiations
            .push(FF::StructVariantInstantiation {
                handle,
                type_parameters,
            });
        self.struct_variant_inst_to_idx.insert(key, idx);
        idx
    }

    /// Obtains or generates a constant index.
    pub fn constant_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        cons: &Constant,
        ty: &Type,
    ) -> FF::ConstantPoolIndex {
        let canonical_const = cons.to_canonical();
        if let Some(idx) = self.cons_to_idx.get(&(canonical_const.clone(), ty.clone())) {
            return *idx;
        }
        let data = canonical_const
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
        self.cons_to_idx.insert((canonical_const, ty.clone()), idx);
        idx
    }
}

impl ModuleContext<'_> {
    /// Emits an error at the location.
    pub fn error(&self, loc: impl AsRef<Loc>, msg: impl AsRef<str>) {
        self.env.diag(Severity::Error, loc.as_ref(), msg.as_ref())
    }

    /// Emits an internal error at the location.
    pub fn internal_error(&self, loc: impl AsRef<Loc>, msg: impl AsRef<str> + ToString) {
        self.env.diag_with_notes(
            Severity::Bug,
            loc.as_ref(),
            format!("compiler internal error: {}", msg.to_string()).as_str(),
            vec![COMPILER_BUG_REPORT_MSG.to_string()],
        )
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
        if !fun.module_env.is_std_vector() {
            return None;
        }
        let pool = self.env.symbol_pool();
        let function_name = pool.string(fun.get_name());
        if !well_known::VECTOR_FUNCS_WITH_BYTECODE_INSTRS.contains(&function_name.as_str()) {
            // early return if vector function does not have a bytecode instruction
            return None;
        }

        if let Some(inst) = inst_sign {
            match function_name.as_str() {
                // note: the following matched strings should all be present in `well_known::VECTOR_FUNCS_WITH_BYTECODE_INSTRS`
                "empty" => Some(FF::Bytecode::VecPack(inst, 0)),
                "length" => Some(FF::Bytecode::VecLen(inst)),
                "borrow" => Some(FF::Bytecode::VecImmBorrow(inst)),
                "borrow_mut" => Some(FF::Bytecode::VecMutBorrow(inst)),
                "push_back" => Some(FF::Bytecode::VecPushBack(inst)),
                "pop_back" => Some(FF::Bytecode::VecPopBack(inst)),
                "destroy_empty" => Some(FF::Bytecode::VecUnpack(inst, 0)),
                "swap" => Some(FF::Bytecode::VecSwap(inst)),
                _ => {
                    self.internal_error(
                        loc,
                        format!("unexpected vector function `{}`", function_name),
                    );
                    None
                },
            }
        } else {
            self.internal_error(loc, "expected type instantiation for vector operation");
            None
        }
    }

    /// Convert the symbol into a string.
    pub fn symbol_to_str(&self, s: Symbol) -> String {
        s.display(self.env.symbol_pool()).to_string()
    }
}

impl ModuleContext<'_> {
    /// Acquires analysis. This is temporary until we have the full reference analysis.
    fn generate_acquires_map(&self, module: &ModuleEnv) -> BTreeMap<FunId, BTreeSet<StructId>> {
        // Compute map with direct usage of resources
        let mut usage_map = module
            .get_functions()
            .filter(|f| !f.is_inline())
            .map(|f| (f.get_id(), self.get_direct_function_acquires(&f)))
            .collect::<BTreeMap<_, _>>();
        // Now run a fixed-point loop: add resources used by called functions until there are no
        // changes.
        loop {
            let mut changes = false;
            for fun in module.get_functions() {
                if fun.is_inline() {
                    continue;
                }
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
                    }
                _ => {}
            }
        }
        result
    }

    /// Converts to a name with location as expected by the SourceMap format.
    pub(crate) fn source_name(&self, name: impl AsRef<Symbol>, loc: impl AsRef<Loc>) -> SourceName {
        (
            name.as_ref().display(self.env.symbol_pool()).to_string(),
            self.env.to_ir_loc(loc.as_ref()),
        )
    }

    /// Delivers the function attributes which are relevant for execution for the given
    /// function. This includes annotated ones as well as ones which are derived.
    /// Currently, a public function derives `Persistent`.
    pub(crate) fn function_attributes(&self, fun_env: &FunctionEnv) -> Vec<FF::FunctionAttribute> {
        let mut result = vec![];
        let mut has_persistent = false;
        for attr in fun_env.get_attributes() {
            match attr {
                Attribute::Apply(_, name, args) => {
                    let no_args = |attr: &str| {
                        if !args.is_empty() {
                            self.error(
                                fun_env.get_id_loc(),
                                format!("attribute `{}` cannot have arguments", attr),
                            )
                        }
                        if fun_env.module_env.is_script_module() {
                            self.error(
                                fun_env.get_id_loc(),
                                format!("attribute `{}` cannot be on script functions", attr),
                            )
                        }
                    };
                    let name = fun_env.symbol_pool().string(*name);
                    match name.as_str() {
                        well_known::PERSISTENT_ATTRIBUTE => {
                            no_args(name.as_str());
                            has_persistent = true;
                            result.push(FF::FunctionAttribute::Persistent)
                        },
                        well_known::MODULE_LOCK_ATTRIBUTE => {
                            no_args(name.as_str());
                            result.push(FF::FunctionAttribute::ModuleLock)
                        },
                        _ => {
                            // skip
                        },
                    }
                },
                Attribute::Assign(_, name, _) => {
                    let name = fun_env.symbol_pool().string(*name);
                    if matches!(
                        name.as_str(),
                        well_known::PERSISTENT_ATTRIBUTE | well_known::MODULE_LOCK_ATTRIBUTE
                    ) {
                        self.error(
                            fun_env.get_id_loc(),
                            format!("attribute `{}` cannot be assigned to", name),
                        )
                    }
                },
            }
        }
        if !has_persistent && fun_env.visibility() == Visibility::Public {
            // For a public function, derive the persistent attribute
            result.push(FF::FunctionAttribute::Persistent)
        }
        result
    }
}
