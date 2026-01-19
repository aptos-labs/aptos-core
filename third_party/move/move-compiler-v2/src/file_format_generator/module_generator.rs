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
use move_binary_format::{
    file_format as FF,
    file_format::{VariantIndex, Visibility},
    file_format_common,
};
use move_bytecode_source_map::source_map::{SourceMap, SourceName};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{
        BORROW, BORROW_MUT, PACK, PACK_VARIANT, PUBLIC_STRUCT_DELIMITER, TEST_VARIANT, UNPACK,
        UNPACK_VARIANT,
    },
    metadata::Metadata,
};
use move_ir_types::ast as IR_AST;
use move_model::{
    ast::{
        AccessSpecifier, AccessSpecifierKind, AddressSpecifier, Attribute, AttributeValue,
        ResourceSpecifier, Value,
    },
    metadata::{
        lang_feature_versions::LANGUAGE_VERSION_FOR_RAC, CompilationMetadata, CompilerVersion,
        LanguageVersion, COMPILATION_METADATA_KEY,
    },
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
use num::ToPrimitive;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

/// Data structure to store indices for struct APIs.
#[derive(Debug, Default)]
struct StructAPIIndex {
    /// For each enum, maintaining a map from variant to corresponding handle index of test variant API.
    enum_to_test_variant_api_idx:
        BTreeMap<QualifiedId<StructId>, BTreeMap<Symbol, FF::FunctionHandleIndex>>,
    /// For struct, there is one pack API (variant is None);
    /// For each enum, maintaining a map from variant to corresponding handle index of pack API.
    struct_to_pack_api_idx:
        BTreeMap<QualifiedId<StructId>, BTreeMap<Option<Symbol>, FF::FunctionHandleIndex>>,
    /// For struct, there is one unpack API (variant is None);
    /// For each enum, maintaining a map from variant to corresponding handle index of unpack API.
    struct_to_unpack_api_idx:
        BTreeMap<QualifiedId<StructId>, BTreeMap<Option<Symbol>, FF::FunctionHandleIndex>>,
    /// For struct, each offset corresponds to one immutable borrow field API;
    /// For each enum, maintaining a vector of tuples (variants, offset, type, handle_index of borrow field API)
    struct_to_immutable_borrow_field_api_idx: BTreeMap<
        QualifiedId<StructId>,
        Vec<((Option<Vec<Symbol>>, usize, Type), FF::FunctionHandleIndex)>,
    >,
    /// For struct, each offset corresponds to one immutable borrow field API;
    /// For each enum, maintaining a vector of tuples (variants, offset, type, handle_index of borrow field API)
    struct_to_mutable_borrow_field_api_idx: BTreeMap<
        QualifiedId<StructId>,
        Vec<((Option<Vec<Symbol>>, usize, Type), FF::FunctionHandleIndex)>,
    >,
}

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
    /// A mapping from function handles to instantiation indices.
    fun_handle_idx_to_inst_idx:
        BTreeMap<(FF::FunctionHandleIndex, FF::SignatureIndex), FF::FunctionInstantiationIndex>,
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
    /// Collection of index maps for struct APIs.
    struct_api_index: StructAPIIndex,
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
        let gen_access_specifiers = language_version.is_at_least(LANGUAGE_VERSION_FOR_RAC);
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
        let mut genr = Self {
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
            fun_handle_idx_to_inst_idx: Default::default(),
            main_handle: None,
            script_handle: None,
            module,
            source_map,
            struct_api_index: Default::default(),
        };
        genr.gen_module(ctx, module_env);
        (genr.module, genr.source_map, genr.main_handle)
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
            self.gen_struct(ctx, &struct_env);
            // Generate non-private struct APIs
            if ctx
                .env
                .language_version()
                .language_version_for_public_struct()
                && struct_env.get_visibility().is_public_or_friend()
            {
                FunctionGenerator::gen_struct_test_variant_api(self, ctx, &struct_env);
                FunctionGenerator::gen_struct_pack_api(self, ctx, &struct_env);
                FunctionGenerator::gen_struct_unpack_api(self, ctx, &struct_env);
                FunctionGenerator::gen_struct_borrow_field_api(self, ctx, &struct_env, true);
                FunctionGenerator::gen_struct_borrow_field_api(self, ctx, &struct_env, false);
            }
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
                I8 => FF::SignatureToken::I8,
                I16 => FF::SignatureToken::I16,
                I32 => FF::SignatureToken::I32,
                I64 => FF::SignatureToken::I64,
                I128 => FF::SignatureToken::I128,
                I256 => FF::SignatureToken::I256,
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
                let list = |genr: &mut ModuleGenerator, ts: Vec<Type>| {
                    ts.into_iter()
                        .map(|t| genr.signature_token(ctx, loc, &t))
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

    /// Common builder for generating function handles for struct APIs
    fn build_struct_api_index_common<K: Ord>(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
        op_prefix: &str, // e.g., PACK / UNPACK / TEST_VARIANT
        param_sig_index: Option<FF::SignatureIndex>, // When None, the parameters are the fields of the struct
        return_sig_index: Option<FF::SignatureIndex>, // When None, the return type is the list of fields of the struct
        key_conv: fn(Option<Symbol>) -> K,
    ) -> BTreeMap<K, FF::FunctionHandleIndex> {
        let module = self.module_index(ctx, loc, &struct_env.module_env);
        let struct_name = struct_env.get_name_str();
        let fun_name_prefix = format!("{}{}{}", op_prefix, PUBLIC_STRUCT_DELIMITER, struct_name);

        let type_parameters = struct_env
            .get_type_parameters()
            .iter()
            .map(|TypeParameter(_, TypeParameterKind { abilities, .. }, _)| *abilities)
            .collect_vec();

        let pool = struct_env.env().symbol_pool();
        let cases = if struct_env.has_variants() {
            struct_env
                .get_variants()
                .map(|variant| {
                    let name = format!(
                        "{}{}{}",
                        fun_name_prefix,
                        PUBLIC_STRUCT_DELIMITER,
                        variant.display(pool)
                    );
                    let field_types = struct_env
                        .get_fields_of_variant(variant)
                        .map(|f| f.get_type())
                        .collect::<Vec<_>>();
                    (Some(variant), name, field_types)
                })
                .collect()
        } else {
            let field_types = struct_env
                .get_fields()
                .map(|f| f.get_type())
                .collect::<Vec<_>>();
            vec![(None, fun_name_prefix, field_types)]
        };

        let mut result = BTreeMap::new();
        for (variant_opt, name, field_types) in cases {
            let params_sig =
                param_sig_index.unwrap_or_else(|| self.signature(ctx, loc, field_types.clone()));
            let return_sig =
                return_sig_index.unwrap_or_else(|| self.signature(ctx, loc, field_types));

            let idx = FF::FunctionHandleIndex(ctx.checked_bound(
                loc,
                self.module.function_handles.len(),
                MAX_FUNCTION_COUNT,
                "used function",
            ));

            let name_sym = struct_env.env().symbol_pool().make(&name);
            let name_idx = self.name_index(ctx, loc, name_sym);

            // Attach attributes according to the operation
            let attributes = match op_prefix {
                PACK => match variant_opt {
                    Some(variant) => {
                        let idx = struct_env
                            .get_variants()
                            .position(|v| v == variant)
                            .unwrap();
                        vec![FF::FunctionAttribute::PackVariant(idx as VariantIndex)]
                    },
                    None => vec![FF::FunctionAttribute::Pack],
                },

                UNPACK => match variant_opt {
                    Some(variant) => {
                        let idx = struct_env
                            .get_variants()
                            .position(|v| v == variant)
                            .unwrap();
                        vec![FF::FunctionAttribute::UnpackVariant(idx as VariantIndex)]
                    },
                    None => vec![FF::FunctionAttribute::Unpack],
                },

                TEST_VARIANT => {
                    let variant = variant_opt.expect("test_variant must be a concrete variant");
                    let idx = struct_env
                        .get_variants()
                        .position(|v| v == variant)
                        .unwrap();
                    vec![FF::FunctionAttribute::TestVariant(idx as VariantIndex)]
                },

                _ => vec![],
            };

            let handle = FF::FunctionHandle {
                module,
                name: name_idx,
                type_parameters: type_parameters.clone(),
                parameters: params_sig,
                return_: return_sig,
                access_specifiers: None,
                attributes,
            };

            self.module.function_handles.push(handle);
            result.insert(key_conv(variant_opt), idx);
        }

        result
    }

    fn pack_or_unpack_struct_api_index<const IS_PACK: bool>(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
    ) -> BTreeMap<Option<Symbol>, FF::FunctionHandleIndex> {
        let qid = struct_env.get_qualified_id();

        if IS_PACK {
            if let Some(v) = self.struct_api_index.struct_to_pack_api_idx.get(&qid) {
                return v.clone();
            }
        } else if let Some(v) = self.struct_api_index.struct_to_unpack_api_idx.get(&qid) {
            return v.clone();
        }

        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            TypeParameter::vec_to_formals(struct_env.get_type_parameters()),
        );
        let struct_sig = self.signature(ctx, loc, vec![struct_ty]);

        let (params, ret) = if !IS_PACK {
            (Some(struct_sig), None)
        } else {
            (None, Some(struct_sig))
        };

        let op_prefix = if IS_PACK { PACK } else { UNPACK };

        let built = self.build_struct_api_index_common(
            ctx,
            loc,
            struct_env,
            op_prefix,
            params, // params
            ret,    // return
            std::convert::identity,
        );

        if IS_PACK {
            self.struct_api_index
                .struct_to_pack_api_idx
                .insert(qid, built.clone());
        } else {
            self.struct_api_index
                .struct_to_unpack_api_idx
                .insert(qid, built.clone());
        }
        built
    }

    /// Generates function handle index for pack API
    /// API naming example:
    /// public struct S {
    ///     x: u64,
    /// }
    /// pack$S(x: u64): S
    ///
    /// public enum S {
    ///    V1 { x: u64 },
    ///    V2 { x: u64, y: u64 },
    /// }
    /// pack$S$V1(x: u64): S
    /// pack$S$V2(x: u64, y: u64, ...): S
    pub fn struct_api_pack_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
    ) -> BTreeMap<Option<Symbol>, FF::FunctionHandleIndex> {
        self.pack_or_unpack_struct_api_index::<true>(ctx, loc, struct_env)
    }

    /// Generates function handle index for unpack api
    /// API naming example:
    /// public struct S {
    ///     x: u64,
    /// }
    /// unpack$S(_s: S): u64
    ///
    /// public enum S {
    ///    V1 { x: u64 },
    ///    V2 { x: u64, y: u64 },
    /// }
    /// unpack$S$V1(_s: S): u64
    /// unpack$S$V2(_s: S): (u64, u64)
    pub fn struct_api_unpack_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
    ) -> BTreeMap<Option<Symbol>, FF::FunctionHandleIndex> {
        self.pack_or_unpack_struct_api_index::<false>(ctx, loc, struct_env)
    }

    /// Generates function handle index for test variant api
    /// API naming example:
    /// public enum S {
    ///    V1 { x: u64 },
    ///    V2 { x: u64, y: u64 },
    /// }
    /// test_variant$S$V1(_s: &S): bool
    /// test_variant$S$V2(_s: &S): bool
    pub fn struct_api_test_variant_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
    ) -> BTreeMap<Symbol, FF::FunctionHandleIndex> {
        let qid = struct_env.get_qualified_id();
        if let Some(v) = self.struct_api_index.enum_to_test_variant_api_idx.get(&qid) {
            return v.clone();
        }
        fn must_some(opt: Option<Symbol>) -> Symbol {
            opt.expect("test_variant key must be a concrete variant")
        }
        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            TypeParameter::vec_to_formals(struct_env.get_type_parameters()),
        );
        let ref_struct_ty = Type::Reference(ReferenceKind::Immutable, Box::new(struct_ty));
        let ref_struct_sig = self.signature(ctx, loc, vec![ref_struct_ty]);
        let bool_sig = self.signature(ctx, loc, vec![Type::Primitive(PrimitiveType::Bool)]);
        let built = self.build_struct_api_index_common(
            ctx,
            loc,
            struct_env,
            TEST_VARIANT,
            Some(ref_struct_sig),
            Some(bool_sig),
            must_some,
        );
        self.struct_api_index
            .enum_to_test_variant_api_idx
            .insert(qid, built.clone());
        built
    }

    /// Helper function to construct:
    /// - A mapping from (offset, type) to variants
    /// - A mapping from offset and type to a number which represents the order for the type at that offset.
    /// note that the order is across the whole enum type.
    fn construct_map_for_borrow_field_api_with_type(
        struct_env: &StructEnv<'_>,
    ) -> (
        BTreeMap<(usize, Type), Option<Vec<Symbol>>>,
        BTreeMap<(usize, Type), usize>,
    ) {
        let mut map: BTreeMap<(usize, Type), Option<Vec<Symbol>>> = BTreeMap::new();
        let mut order_map: BTreeMap<(usize, Type), usize> = BTreeMap::new();
        let mut next_order = 0;
        // get_variants guarantees the order of variants is the order of the enum.
        for variant in struct_env.get_variants() {
            for field in struct_env.get_fields_of_variant(variant) {
                let ty: Type = field.get_type();
                let offset = field.get_offset();
                match map.entry((offset, ty.clone())) {
                    Entry::Vacant(e) => {
                        e.insert(Some(vec![variant]));
                    },
                    Entry::Occupied(mut e) => {
                        if let Some(variant_vec) = e.get_mut() {
                            variant_vec.push(variant);
                        }
                    },
                }
                if let Entry::Vacant(e) = order_map.entry((offset, ty.clone())) {
                    e.insert(next_order);
                    next_order += 1;
                }
            }
        }
        (map, order_map)
    }

    /// Generates function handle index for borrow field API
    /// Returns a map (Option<variants>, offset, type) to function handle index
    /// API naming example:
    /// public struct S {
    ///     x: u64,
    /// }
    /// borrow$S$0(_s: &S): &u64
    /// borrow_mut$S$0(_s: &mut S): &mut u64
    ///
    /// public enum S {
    ///    V1 { x: u64 },
    ///    V2 { x: u64, y: u64 },
    /// }
    /// borrow$S$0$u64(_s: &S): &u64
    /// borrow$S$1$u64(_s: &S): &u64
    /// borrow_mut$S$0$u64(_s: &mut S): &mut u64
    /// borrow_mut$S$1$u64(_s: &mut S): &mut u64
    pub fn struct_api_borrow_index(
        &mut self,
        ctx: &ModuleContext,
        loc: &Loc,
        struct_env: &StructEnv,
        is_imm: bool,
    ) -> Vec<((Option<Vec<Symbol>>, usize, Type), FF::FunctionHandleIndex)> {
        let module = self.module_index(ctx, loc, &struct_env.module_env);
        if is_imm {
            if let Some(ret) = self
                .struct_api_index
                .struct_to_immutable_borrow_field_api_idx
                .get(&struct_env.get_qualified_id())
            {
                return ret.clone();
            }
        }
        if !is_imm {
            if let Some(ret) = self
                .struct_api_index
                .struct_to_mutable_borrow_field_api_idx
                .get(&struct_env.get_qualified_id())
            {
                return ret.clone();
            }
        }

        let struct_name = struct_env.get_name_str();
        let fun_name_prefix = format!(
            "{}{}{}",
            if is_imm { BORROW } else { BORROW_MUT },
            PUBLIC_STRUCT_DELIMITER,
            struct_name
        );
        let struct_ty = Type::Struct(
            struct_env.module_env.get_id(),
            struct_env.get_id(),
            TypeParameter::vec_to_formals(struct_env.get_type_parameters()),
        );
        let ref_struct_ty = struct_ty.wrap_in_reference(!is_imm);
        let type_parameters = struct_env
            .get_type_parameters()
            .iter()
            .map(|TypeParameter(_, TypeParameterKind { abilities, .. }, _)| *abilities)
            .collect::<Vec<_>>();
        let parameters = self.signature(ctx, loc, vec![ref_struct_ty]);
        let mut ret = vec![];
        let attributes = |offset: usize| {
            vec![
                if is_imm {
                    FF::FunctionAttribute::BorrowFieldImmutable(offset as FF::MemberCount)
                } else {
                    FF::FunctionAttribute::BorrowFieldMutable(offset as FF::MemberCount)
                },
            ]
        };
        if struct_env.has_variants() {
            let (ty_offset_to_variant_map, ty_offset_to_order_map) =
                Self::construct_map_for_borrow_field_api_with_type(struct_env);
            let mut handle_elements = vec![];
            for ((offset, ty), variant_vec) in ty_offset_to_variant_map.iter() {
                let ty_order = ty_offset_to_order_map.get(&(*offset, ty.clone())).unwrap();
                let name = format!(
                    "{}{}{}{}{}",
                    fun_name_prefix,
                    PUBLIC_STRUCT_DELIMITER,
                    offset,
                    PUBLIC_STRUCT_DELIMITER,
                    ty_order
                );
                handle_elements.push((name, ty.clone(), variant_vec.clone(), offset));
            }
            for (name, ty, variant_vec, offset) in handle_elements {
                let return_: FF::SignatureIndex =
                    self.signature(ctx, loc, vec![ty.wrap_in_reference(!is_imm)]);
                // max field number is constant FF::MemberCount so we can safely use MemberCount for offset
                // as long as the program is well-formed.
                let handle: FF::FunctionHandle = FF::FunctionHandle {
                    module,
                    name: self.name_index(ctx, loc, struct_env.env().symbol_pool().make(&name)),
                    type_parameters: type_parameters.clone(),
                    parameters,
                    return_,
                    access_specifiers: None,
                    attributes: attributes(*offset),
                };
                let idx = FF::FunctionHandleIndex(ctx.checked_bound(
                    loc,
                    self.module.function_handles.len(),
                    MAX_FUNCTION_COUNT,
                    "used function",
                ));
                self.module.function_handles.push(handle);
                ret.push(((variant_vec, *offset, ty), idx));
            }
        } else {
            for field in struct_env.get_fields() {
                let offset = field.get_offset();
                let ref_type = field.get_type().wrap_in_reference(!is_imm);
                let return_: FF::SignatureIndex = self.signature(ctx, loc, vec![ref_type.clone()]);
                let name = format!("{}{}{}", fun_name_prefix, PUBLIC_STRUCT_DELIMITER, offset);
                let idx = FF::FunctionHandleIndex(ctx.checked_bound(
                    loc,
                    self.module.function_handles.len(),
                    MAX_FUNCTION_COUNT,
                    "used function",
                ));
                let handle: FF::FunctionHandle = FF::FunctionHandle {
                    module,
                    name: self.name_index(ctx, loc, struct_env.env().symbol_pool().make(&name)),
                    type_parameters: type_parameters.clone(),
                    parameters,
                    return_,
                    access_specifiers: None,
                    attributes: attributes(offset),
                };
                self.module.function_handles.push(handle);
                ret.push(((None, offset, ref_type.clone()), idx));
            }
        }
        if is_imm {
            self.struct_api_index
                .struct_to_immutable_borrow_field_api_idx
                .insert(struct_env.get_qualified_id(), ret.clone());
        } else {
            self.struct_api_index
                .struct_to_mutable_borrow_field_api_idx
                .insert(struct_env.get_qualified_id(), ret.clone());
        }
        ret
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

    /// Obtains or generates instantiation index from handle index and type parameters
    pub fn function_instantiation_index_from_handle_index(
        &mut self,
        handle: FF::FunctionHandleIndex,
        ctx: &ModuleContext,
        loc: &Loc,
        inst: Vec<Type>,
    ) -> FF::FunctionInstantiationIndex {
        let type_parameters = self.signature(ctx, loc, inst);
        let fun_inst = FF::FunctionInstantiation {
            handle,
            type_parameters,
        };
        if self
            .fun_handle_idx_to_inst_idx
            .contains_key(&(handle, type_parameters))
        {
            return *self
                .fun_handle_idx_to_inst_idx
                .get(&(handle, type_parameters))
                .unwrap();
        }
        let idx = FF::FunctionInstantiationIndex(ctx.checked_bound(
            loc,
            self.module.function_instantiations.len(),
            MAX_FUNCTION_INST_COUNT,
            "function instantiation",
        ));
        self.module.function_instantiations.push(fun_inst);
        self.fun_handle_idx_to_inst_idx
            .insert((handle, type_parameters), idx);
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
    /// Get the called functions from the same module by traversing the stackless bytecode.
    fn get_same_module_called_functions(
        &self,
        module: &ModuleEnv,
    ) -> BTreeMap<FunId, BTreeSet<FunId>> {
        let mut called_functions_map = BTreeMap::new();
        for fun in module.get_functions() {
            if fun.is_inline() {
                continue;
            }
            let mut called_functions = BTreeSet::new();
            let target = self.targets.get_target(&fun, &FunctionVariant::Baseline);
            for bc in target.get_bytecode() {
                use Bytecode::*;
                use Operation::*;
                if let Call(_, _, Function(mid, fid, _), ..) = bc {
                    // only add functions from the same module and not the function itself
                    if *mid == module.get_id() && *fid != fun.get_id() {
                        called_functions.insert(*fid);
                    }
                }
            }
            called_functions_map.insert(fun.get_id(), called_functions);
        }
        called_functions_map
    }

    /// Acquires analysis. This is temporary until we have the full reference analysis.
    fn generate_acquires_map(&self, module: &ModuleEnv) -> BTreeMap<FunId, BTreeSet<StructId>> {
        // Compute map with direct usage of resources
        let mut usage_map = module
            .get_functions()
            .filter(|f| !f.is_inline())
            .map(|f| (f.get_id(), self.get_direct_function_acquires(&f)))
            .collect::<BTreeMap<_, _>>();
        let called_functions_map = self.get_same_module_called_functions(module);
        // Now run a fixed-point loop: add resources used by called functions until there are no
        // changes.
        loop {
            let mut changes = false;
            for fun in module.get_functions() {
                if fun.is_inline() {
                    continue;
                }
                let mut usage = usage_map[&fun.get_id()].clone();
                let count = usage.len();
                for called_fun in called_functions_map[&fun.get_id()].iter() {
                    // Extend usage by that of callees from the same module. Acquires is only
                    // local to a module.
                    usage.extend(usage_map[called_fun].iter().cloned());
                }
                if usage.len() > count {
                    *usage_map.get_mut(&fun.get_id()).unwrap() = usage;
                    changes = true;
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

        // Validate that the attribute is not on script functions.
        let validate_not_script = |attr_name: &str, this: &Self| {
            if fun_env.module_env.is_script_module() {
                this.error(
                    fun_env.get_id_loc(),
                    format!("attribute `{}` cannot be on script functions", attr_name),
                );
            }
        };

        // Validate that the attribute does not have arguments and is not on script functions.
        let validate_no_args_and_not_script = |attr_name: &str, args: &[Attribute], this: &Self| {
            if !args.is_empty() {
                this.error(
                    fun_env.get_id_loc(),
                    format!("attribute `{}` cannot have arguments", attr_name),
                );
            }
            validate_not_script(attr_name, this);
        };

        // Parse the numeric value of the attribute.
        let parse_u16 =
            |attr_name: &str, attribute_value: &AttributeValue, this: &Self| -> Option<u16> {
                match attribute_value {
                    AttributeValue::Value(_, Value::Number(number)) => {
                        if let Some(v) = number.to_u16() {
                            Some(v)
                        } else {
                            this.error(
                                fun_env.get_id_loc(),
                                format!(
                                    "attribute `{}` must have a number value between 0 and 65535",
                                    attr_name
                                ),
                            );
                            None
                        }
                    },
                    _ => {
                        this.error(
                            fun_env.get_id_loc(),
                            format!("attribute `{}` must have a number value", attr_name),
                        );
                        None
                    },
                }
            };

        for attr in fun_env.get_attributes() {
            match attr {
                Attribute::Apply(_, name, args) => {
                    let name = fun_env.symbol_pool().string(*name);
                    match name.as_str() {
                        PACK_VARIANT | UNPACK_VARIANT | TEST_VARIANT | BORROW | BORROW_MUT => {
                            self.error(
                                fun_env.get_id_loc(),
                                format!("attribute `{}` cannot be applied", name),
                            );
                        },
                        well_known::PERSISTENT_ATTRIBUTE => {
                            validate_no_args_and_not_script(name.as_str(), args, self);
                            has_persistent = true;
                            result.push(FF::FunctionAttribute::Persistent);
                        },
                        well_known::MODULE_LOCK_ATTRIBUTE => {
                            validate_no_args_and_not_script(name.as_str(), args, self);
                            result.push(FF::FunctionAttribute::ModuleLock);
                        },
                        PACK => {
                            validate_no_args_and_not_script(name.as_str(), args, self);
                            result.push(FF::FunctionAttribute::Pack);
                        },
                        UNPACK => {
                            validate_no_args_and_not_script(name.as_str(), args, self);
                            result.push(FF::FunctionAttribute::Unpack);
                        },
                        _ => { /* skip */ },
                    }
                },

                Attribute::Assign(_, name, attribute_value) => {
                    let name = fun_env.symbol_pool().string(*name);

                    let ctor: Option<fn(u16) -> FF::FunctionAttribute> = match name.as_str() {
                        well_known::PERSISTENT_ATTRIBUTE
                        | well_known::MODULE_LOCK_ATTRIBUTE
                        | PACK
                        | UNPACK => {
                            self.error(
                                fun_env.get_id_loc(),
                                format!("attribute `{}` cannot be assigned to", name),
                            );
                            None
                        },
                        PACK_VARIANT => {
                            validate_not_script(name.as_str(), self);
                            Some(FF::FunctionAttribute::PackVariant)
                        },
                        UNPACK_VARIANT => {
                            validate_not_script(name.as_str(), self);
                            Some(FF::FunctionAttribute::UnpackVariant)
                        },
                        TEST_VARIANT => {
                            validate_not_script(name.as_str(), self);
                            Some(FF::FunctionAttribute::TestVariant)
                        },
                        BORROW => {
                            validate_not_script(name.as_str(), self);
                            Some(FF::FunctionAttribute::BorrowFieldImmutable)
                        },
                        BORROW_MUT => {
                            validate_not_script(name.as_str(), self);
                            Some(FF::FunctionAttribute::BorrowFieldMutable)
                        },
                        _ => None,
                    };

                    if let Some(ctor) = ctor {
                        if let Some(v) = parse_u16(name.as_str(), attribute_value, self) {
                            result.push(ctor(v));
                        }
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
