// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Defines a wrapper for [`CompiledModule`] with all its types pre-interned.

use crate::{
    interner::{InternedIdentifier, InternedModuleId, Interner, TypeSubstitutionError},
    types::{view_type, view_type_list, InternedType, InternedTypeList, Type, EMPTY_TYPE_LIST},
    ExecutionErrorKind, IntoExecutionError,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        ConstantPoolIndex, FieldHandleIndex, FunctionHandleIndex, FunctionInstantiationIndex,
        IdentifierIndex, ModuleHandleIndex, SignatureIndex, SignatureToken, StructDefinitionIndex,
        StructFieldInformation, StructHandle, StructHandleIndex, VariantFieldHandleIndex,
        VariantIndex,
    },
    CompiledModule,
};
use move_core_types::language_storage::{FunctionParamOrReturnTag, StructTag, TypeTag};
use shared_dsa::UnorderedMap;
use std::ops::Deref;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PreparedModuleError {
    #[error("native struct fields are deprecated")]
    NativeFieldsDeprecated,

    #[error(transparent)]
    TypeSubstitution(#[from] TypeSubstitutionError),
}

impl IntoExecutionError for PreparedModuleError {
    fn kind(&self) -> ExecutionErrorKind {
        use PreparedModuleError::*;
        match self {
            NativeFieldsDeprecated => ExecutionErrorKind::InvariantViolation,
            TypeSubstitution(err) => err.kind(),
        }
    }
}

/// Wraps deserialized and verified [`CompiledModule`] with pre-interned type
/// pools. Users can use interned type representation directly using same table
/// indices, without any runtime interning.
pub struct PreparedModule {
    /// Original compiled module. Kept so that its tables can be accessed.
    module: CompiledModule,
    /// Interned ID of this module.
    id: InternedModuleId,
    /// Interned signatures from compiled module. Note that below tables are
    /// still needed because file format does not deduplicate all signatures.
    /// For example, fields carry their own signature and not an index into
    /// signatures pool.
    ///
    /// Indexed by [`SignatureIndex`].
    signatures: Vec<Vec<InternedType>>,
    /// Interned nominal types (structs or enums) from compiled module (both
    /// declared and imported).
    ///
    /// Indexed by [`StructHandleIndex`].
    nominal_types: Vec<InternedType>,
    /// Interned identity -> the [`StructHandle`] declaring it, for every
    /// nominal this module can name, local or imported.
    ///
    /// We just need an index: the handle already lives in [`Self::module`] and
    /// carries both the declared abilities and the phantom markers. A module may
    /// declare an imported nominal with fewer of each than its definition, and
    /// that narrower declaration governs the module's own code, so it is the
    /// right basis for checking it.
    nominal_handles: UnorderedMap<(InternedModuleId, InternedIdentifier), StructHandleIndex>,
    /// Maps struct or enum name to its local [`StructDefinitionIndex`].
    nominal_definitions: UnorderedMap<InternedIdentifier, StructDefinitionIndex>,
    /// Interned struct or enum field types from compiled module. Only for
    /// declared structs or enums.
    ///
    /// Indexed by [`StructDefinitionIndex`].
    field_types: Vec<FieldTypes>,
    /// Interned constant types from compiled module.
    ///
    /// Indexed by [`ConstantPoolIndex`].
    constant_types: Vec<InternedType>,
    /// Interned identifiers from compiled module.
    ///
    /// Indexed by [`IdentifierIndex`].
    interned_identifiers: Vec<InternedIdentifier>,
    /// Interned module IDs for every module handle in the compiled module.
    ///
    /// Indexed by [`ModuleHandleIndex`]. Self-handle resolves to the same id
    /// as [`Self::id`].
    module_ids: Vec<InternedModuleId>,
    /// Parameter and return types for every function handle referenced in
    /// the module. For a generic function, these may contain [`Type::TypeParam`]
    /// nodes referencing the function's own type parameters — they are not
    /// guaranteed to be fully concrete.
    ///
    /// Indexed by [`FunctionHandleIndex`].
    function_signatures: Vec<FunctionSignature>,
    /// Parameter and return types, as well as type arguments, for every
    /// function instantiation in this module. The type arguments have been
    /// applied to the underlying handle's params/returns. They may still
    /// contain [`Type::TypeParam`] nodes referencing type parameters of the
    /// *enclosing* (caller) function — substitution here only resolves the
    /// call site's own type arguments, not the caller's free type parameters.
    ///
    /// Indexed by [`FunctionInstantiationIndex`].
    function_instantiation_signatures: Vec<FunctionInstantiationSignature>,
}

/// Parameter and return types of a function handle, interned. Note that for a
/// generic function these may contain free type parameters of the function
/// itself, and are therefore not necessarily fully concrete.
#[derive(Clone, Copy)]
pub struct FunctionSignature {
    pub params: InternedTypeList,
    pub returns: InternedTypeList,
}

/// Parameter and return types of a function instantiation, together with the
/// type arguments applied at the call site. The type arguments have already
/// been substituted into `params` and `returns`. The result may still contain
/// free type parameters of the enclosing function, so this is not guaranteed
/// to be fully concrete either.
#[derive(Clone, Copy)]
pub struct FunctionInstantiationSignature {
    pub params: InternedTypeList,
    pub returns: InternedTypeList,
    pub ty_args: InternedTypeList,
}

/// Field types of any struct or enum definition in this module.
#[derive(Clone)]
pub enum FieldTypes {
    Struct(Vec<InternedType>),
    Enum(Vec<Vec<InternedType>>),
}

// All compiled module pools are still accessible.
impl Deref for PreparedModule {
    type Target = CompiledModule;

    fn deref(&self) -> &Self::Target {
        &self.module
    }
}

impl PreparedModule {
    /// Returns interned module ID of this module.
    pub fn id(&self) -> InternedModuleId {
        self.id
    }

    /// Returns the interned module ID for the given module handle.
    pub fn module_id_at(&self, idx: ModuleHandleIndex) -> InternedModuleId {
        self.module_ids[idx.0 as usize]
    }

    /// Returns interned types corresponding to the compiled module's
    /// signature.
    pub fn interned_types_at(&self, idx: SignatureIndex) -> &[InternedType] {
        &self.signatures[idx.0 as usize]
    }

    /// Returns all interned types corresponding to the compiled module's
    /// nominal types (structs or enums). Note that these types can be imported
    /// from other modules.
    pub fn interned_nominal_types_at(&self) -> &[InternedType] {
        &self.nominal_types
    }

    /// Returns interned type corresponding to the compiled module's nominal
    /// type (struct or enum). Note that this type can be imported from other
    /// module.
    pub fn interned_nominal_type_at(&self, idx: StructHandleIndex) -> InternedType {
        self.nominal_types[idx.0 as usize]
    }

    /// Returns interned type corresponding to the compiled module's nominal
    /// type (struct or enum) definition.
    pub fn interned_nominal_def_type_at(&self, def_idx: StructDefinitionIndex) -> InternedType {
        let idx = self.module.struct_def_at(def_idx).struct_handle;
        self.interned_nominal_type_at(idx)
    }

    /// Looks up a struct or enum definition by its interned name, and returns
    /// its index.
    pub fn interned_nominal_type_def_idx(
        &self,
        name: InternedIdentifier,
    ) -> Option<StructDefinitionIndex> {
        let idx = self.nominal_definitions.get(&name).copied()?;
        Some(idx)
    }

    /// Returns interned types corresponding to the compiled module's struct
    /// definition. Returns [`None`] if not a struct.
    pub fn interned_struct_field_types_at(
        &self,
        def_idx: StructDefinitionIndex,
    ) -> Option<&[InternedType]> {
        match &self.field_types[def_idx.0 as usize] {
            FieldTypes::Struct(field_types) => Some(field_types.as_slice()),
            FieldTypes::Enum(..) => None,
        }
    }

    /// Returns interned types corresponding to the compiled module's enum
    /// variant definition. Returns [`None`] if not an enum.
    pub fn interned_variant_field_types_at(
        &self,
        def_idx: StructDefinitionIndex,
        variant_idx: VariantIndex,
    ) -> Option<&[InternedType]> {
        match &self.field_types[def_idx.0 as usize] {
            FieldTypes::Enum(variants) => Some(variants[variant_idx as usize].as_slice()),
            FieldTypes::Struct(..) => None,
        }
    }

    /// The [`StructHandle`] by which this module declares a nominal type,
    /// carrying its abilities and type-parameter metadata. Resolves local and
    /// imported nominals alike; returns [`None`] for a nominal this module
    /// cannot name.
    pub fn nominal_handle(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
    ) -> Option<&StructHandle> {
        let idx = self.nominal_handles.get(&(module_id, name))?;
        Some(self.module.struct_handle_at(*idx))
    }

    /// Looks up a struct or enum definition by its interned name, and returns
    /// its fields (or variants with fields).
    pub fn interned_field_types(&self, name: InternedIdentifier) -> Option<&FieldTypes> {
        let idx = self.interned_nominal_type_def_idx(name)?;
        Some(&self.field_types[idx.0 as usize])
    }

    /// Whether a byte copy of `ty`'s frame bytes is by itself a complete,
    /// faithful copy. False when the value owns heap storage (vectors,
    /// closures, heap-boxed enums): both copies would then own the same
    /// allocation, so a faithful copy must also clone that storage. True for
    /// references: a copied reference must alias the same target, so copying
    /// the fat pointer bytes is correct.
    ///
    /// Conservatively `false` whenever this cannot be established from this
    /// module alone: type parameters, imported nominals, and type arguments
    /// that are not directly substitutable.
    ///
    /// The last case makes the `false` answer an over-approximation. Given
    ///
    /// ```move
    /// struct C<V> has copy { z: V }
    /// struct B<U> has copy { y: U }
    /// struct A<T> has copy { x: B<C<T>> }
    /// ```
    ///
    /// `is_bitwise_copy_type(A<u64>)` returns `false`: `B`'s type argument
    /// `C<T>` is a nominal rather than a bare type parameter, so `T` is never
    /// bound to `u64` and answers `false` once `C`'s field is reached.
    ///
    /// Inherits safety contract of [`view_type`].
    /// TODO(metering, perf): the uncached recursion revisits shared subtypes,
    /// so an all-bitwise struct DAG is traversed exponentially at module load,
    /// and every caller repeats the walk. Fix: return `false` for non-closed
    /// types, substitute field types via the interner, and iterate instead of
    /// recursing, caching the `InternedType -> bool` answer in the global
    /// context beside the other type-keyed caches. Substitution makes every
    /// key closed and hence module-independent (a free [`Type::TypeParam`] is
    /// not a valid key: equal indices at different scopes intern to the same
    /// pointer). Answering from the global context also gives the imported
    /// nominal arm below the cross-module field types it lacks today, and
    /// removes the over-approximation above.
    pub fn is_bitwise_copy_type(&self, ty: InternedType) -> bool {
        self.is_bitwise_copy_type_in_ty_env(ty, &[])
    }

    /// `ty_env` binds [`Type::TypeParam`] nodes to the enclosing nominal's type
    /// arguments, and is empty at top level. A binding may itself contain type
    /// parameters; those are looked up in an empty `ty_env`, so they answer
    /// `false` instead of binding to the wrong parameter.
    fn is_bitwise_copy_type_in_ty_env(&self, ty: InternedType, ty_env: &[InternedType]) -> bool {
        match view_type(ty) {
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. } => true,

            // Owned heap pointers.
            Type::Vector { .. } | Type::Function { .. } => false,

            Type::TypeParam { idx } => ty_env
                .get(*idx as usize)
                .is_some_and(|arg| self.is_bitwise_copy_type_in_ty_env(*arg, &[])),

            Type::Nominal {
                module_id,
                name,
                ty_args,
            } => {
                if *module_id != self.id {
                    // An imported module's field types are not reachable from
                    // here; see the TODO(metering, perf) on
                    // [`Self::is_bitwise_copy_type`].
                    return false;
                }
                let Some(FieldTypes::Struct(fields)) = self.interned_field_types(*name) else {
                    // Enums are heap-boxed regardless of payload.
                    return false;
                };
                // Resolve type arguments that are directly a type parameter of
                // the current environment; anything deeper stays unresolved and
                // fails conservatively inside the recursion (a precision loss
                // removed by the substitution rewrite in the
                // TODO(metering, perf) on [`Self::is_bitwise_copy_type`]).
                let args: Vec<InternedType> = view_type_list(*ty_args)
                    .iter()
                    .map(|arg| {
                        if let Type::TypeParam { idx } = view_type(*arg) {
                            ty_env.get(*idx as usize).copied().unwrap_or(*arg)
                        } else {
                            *arg
                        }
                    })
                    .collect();
                fields
                    .iter()
                    .all(|field| self.is_bitwise_copy_type_in_ty_env(*field, &args))
            },
        }
    }

    /// Returns interned type corresponding to the compiled module's struct
    /// field.
    pub fn interned_field_type_at(&self, idx: FieldHandleIndex) -> InternedType {
        let h = self.module.field_handle_at(idx);
        self.interned_struct_field_types_at(h.owner)
            .expect("Must be a struct")[h.field as usize]
    }

    /// Returns the field's position within its owning struct.
    pub fn field_position_at(&self, idx: FieldHandleIndex) -> u16 {
        self.module.field_handle_at(idx).field
    }

    /// Returns interned type corresponding to the compiled module's enum
    /// variant field.
    pub fn interned_variant_field_type_at(&self, idx: VariantFieldHandleIndex) -> InternedType {
        let h = self.module.variant_field_handle_at(idx);
        let fields = self
            .interned_variant_field_types_at(h.struct_index, h.variants[0])
            .expect("Must be an enum");
        fields[h.field as usize]
    }

    /// Returns interned type corresponding to the compiled module's constant.
    pub fn interned_constant_type_at(&self, idx: ConstantPoolIndex) -> InternedType {
        self.constant_types[idx.0 as usize]
    }

    /// Raw (BCS-encoded) bytes of the constant at `idx`.
    pub fn constant_data_at(&self, idx: ConstantPoolIndex) -> &[u8] {
        &self.module.constant_pool()[idx.0 as usize].data
    }

    /// Returns interned type corresponding to the compiled module's constant.
    pub fn interned_identifier_at(&self, idx: IdentifierIndex) -> InternedIdentifier {
        self.interned_identifiers[idx.0 as usize]
    }

    /// Returns parameter and return types for the given function handle.
    pub fn function_signature_at(&self, idx: FunctionHandleIndex) -> FunctionSignature {
        self.function_signatures[idx.0 as usize]
    }

    /// Returns parameter and return types, as well as type arguments for the
    /// given function instantiation. Parameters and returns have already
    /// been substituted under type arguments.
    pub fn function_instantiation_signature_at(
        &self,
        idx: FunctionInstantiationIndex,
    ) -> FunctionInstantiationSignature {
        self.function_instantiation_signatures[idx.0 as usize]
    }

    /// Builds resolved module from compiled one, interning all signatures,
    /// field and constant types.
    pub fn build(
        module: CompiledModule,
        interner: &impl Interner,
    ) -> Result<Self, PreparedModuleError> {
        let id = interner.module_id_of(module.self_addr(), module.self_name());

        let interned_identifiers = module
            .identifiers
            .iter()
            .map(|identifier| interner.identifier_of(identifier.as_ident_str()))
            .collect::<Vec<_>>();

        let module_ids = module
            .module_handles()
            .iter()
            .map(|mh| {
                let addr = module.address_identifier_at(mh.address);
                let name = module.identifier_at(mh.name);
                interner.module_id_of(addr, name)
            })
            .collect::<Vec<_>>();

        let signatures = module
            .signatures()
            .iter()
            .map(|sig| {
                sig.0
                    .iter()
                    .map(|tok| intern_sig_token(tok, &module, interner))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        // TODO(perf): intern the nominals first and pass a &[InternedType], indexed by struct handle
        // index, to intern_sig_token. That way, we could avoid re-interning the nominal in the
        // Struct case, or the module_id + struct_name in the StructInstantiation case. It saves
        // a few hashmap lookups per struct reference, which may add up across the signature.
        let mut nominal_types = Vec::with_capacity(module.struct_handles.len());
        let mut nominal_handles = UnorderedMap::with_capacity(module.struct_handles.len());
        for (handle_idx, handle) in module.struct_handles.iter().enumerate() {
            let (module_id, name) = intern_struct_handle(handle, &module, interner);
            let ty_args = if handle.type_parameters.is_empty() {
                EMPTY_TYPE_LIST
            } else {
                let params = (0..handle.type_parameters.len())
                    .map(|idx| interner.type_param_of(idx as u16))
                    .collect::<Vec<_>>();
                interner.type_list_of(&params)
            };
            nominal_types.push(interner.nominal_of(module_id, name, ty_args));

            // Uniqueness is a trusted bytecode-verification result.
            // Debug-asserted rather than re-checked.
            let previous =
                nominal_handles.insert((module_id, name), StructHandleIndex(handle_idx as u16));
            debug_assert!(
                previous.is_none(),
                "duplicate struct handle for one (module, name)"
            );
        }

        let mut nominal_definitions = UnorderedMap::with_capacity(module.struct_defs().len());
        let field_types = module
            .struct_defs()
            .iter()
            .enumerate()
            .map(|(idx, def)| {
                let field_types = match &def.field_information {
                    StructFieldInformation::Native => {
                        return Err(PreparedModuleError::NativeFieldsDeprecated);
                    },
                    StructFieldInformation::Declared(fields) => {
                        let fields = fields
                            .iter()
                            .map(|f| intern_sig_token(&f.signature.0, &module, interner))
                            .collect::<Vec<_>>();
                        FieldTypes::Struct(fields)
                    },
                    StructFieldInformation::DeclaredVariants(variants) => {
                        let variants = variants
                            .iter()
                            .map(|v| {
                                v.fields
                                    .iter()
                                    .map(|f| intern_sig_token(&f.signature.0, &module, interner))
                                    .collect::<Vec<_>>()
                            })
                            .collect::<Vec<_>>();
                        FieldTypes::Enum(variants)
                    },
                };

                let handle = module.struct_handle_at(def.struct_handle);
                let name = interned_identifiers[handle.name.0 as usize];
                nominal_definitions.insert(name, StructDefinitionIndex(idx as u16));

                Ok(field_types)
            })
            .collect::<Result<Vec<_>, _>>()?;

        let constant_types = module
            .constant_pool()
            .iter()
            .map(|c| intern_sig_token(&c.type_, &module, interner))
            .collect::<Vec<_>>();

        let function_signatures = module
            .function_handles()
            .iter()
            .map(|h| FunctionSignature {
                params: interner.type_list_of(&signatures[h.parameters.0 as usize]),
                returns: interner.type_list_of(&signatures[h.return_.0 as usize]),
            })
            .collect::<Vec<_>>();

        let function_instantiation_signatures = module
            .function_instantiations()
            .iter()
            .map(|inst| {
                let handle_sig = function_signatures[inst.handle.0 as usize];
                let ty_args = interner.type_list_of(&signatures[inst.type_parameters.0 as usize]);
                let params = interner.subst_type_list(handle_sig.params, ty_args)?;
                let returns = interner.subst_type_list(handle_sig.returns, ty_args)?;
                Ok(FunctionInstantiationSignature {
                    params,
                    returns,
                    ty_args,
                })
            })
            .collect::<Result<Vec<_>, PreparedModuleError>>()?;

        Ok(Self {
            module,
            id,
            signatures,
            nominal_types,
            nominal_handles,
            nominal_definitions,
            field_types,
            constant_types,
            interned_identifiers,
            module_ids,
            function_signatures,
            function_instantiation_signatures,
        })
    }
}

/// Recursively interns `token` into the global type arena. Composite leaves
/// go through `interner`; struct/enum tokens delegate to `resolver`.
///
/// TODO(metering): non-recursive implementation. Coordinate with the similar TODO on
/// `TypeInternerKey`'s `Hash` impl in `types.rs`.
///
/// TODO(perf): probe-before-allocate for composite tokens.
///
/// Right now, every composite variant (Vector, Reference, MutableReference,
/// Function, and the StructInstantiation path through the resolver) allocates a
/// fresh `Type` node in the arena and then hands it to the interner, which
/// discards the new allocation whenever an equivalent entry already exists. For
/// modules with shared signatures (common: many handles reference the same
/// `SignatureIndex`, and `vector<T>` / `&T` appear repeatedly), this means the
/// fast path pays one arena allocation + a dedup probe per occurrence instead
/// of a single probe.
pub fn intern_sig_token(
    token: &SignatureToken,
    module: &CompiledModule,
    interner: &impl Interner,
) -> InternedType {
    use crate::types as ty;
    match token {
        SignatureToken::Bool => ty::BOOL_TY,
        SignatureToken::U8 => ty::U8_TY,
        SignatureToken::U16 => ty::U16_TY,
        SignatureToken::U32 => ty::U32_TY,
        SignatureToken::U64 => ty::U64_TY,
        SignatureToken::U128 => ty::U128_TY,
        SignatureToken::U256 => ty::U256_TY,
        SignatureToken::I8 => ty::I8_TY,
        SignatureToken::I16 => ty::I16_TY,
        SignatureToken::I32 => ty::I32_TY,
        SignatureToken::I64 => ty::I64_TY,
        SignatureToken::I128 => ty::I128_TY,
        SignatureToken::I256 => ty::I256_TY,
        SignatureToken::Address => ty::ADDRESS_TY,
        SignatureToken::Signer => ty::SIGNER_TY,
        SignatureToken::TypeParameter(idx) => interner.type_param_of(*idx),
        SignatureToken::Vector(inner) => {
            let elem = intern_sig_token(inner, module, interner);
            interner.vector_of(elem)
        },
        SignatureToken::Reference(inner) => {
            let inner = intern_sig_token(inner, module, interner);
            interner.immut_ref_of(inner)
        },
        SignatureToken::MutableReference(inner) => {
            let inner = intern_sig_token(inner, module, interner);
            interner.mut_ref_of(inner)
        },
        SignatureToken::Function(args, results, abilities) => {
            let arg_ptrs = args
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<Vec<_>>();
            let result_ptrs = results
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<Vec<_>>();
            let args = interner.type_list_of(&arg_ptrs);
            let results = interner.type_list_of(&result_ptrs);
            interner.function_of(args, results, *abilities)
        },
        SignatureToken::Struct(sh_idx) => {
            let (module_id, struct_name) = intern_struct_info(*sh_idx, module, interner);
            interner.nominal_of(module_id, struct_name, ty::EMPTY_TYPE_LIST)
        },
        SignatureToken::StructInstantiation(sh_idx, ty_args) => {
            let (module_id, struct_name) = intern_struct_info(*sh_idx, module, interner);
            let ty_args = ty_args
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<Vec<_>>();
            interner.nominal_of(module_id, struct_name, interner.type_list_of(&ty_args))
        },
    }
}

/// Interns a runtime [`TypeTag`] (e.g. a transaction's type argument, or a
/// resource's struct tag).
//
// TODO(metering): decide if this construction requires metering.
pub fn intern_type_tag(tag: &TypeTag, interner: &impl Interner) -> anyhow::Result<InternedType> {
    use crate::types as ty;
    Ok(match tag {
        TypeTag::Bool => ty::BOOL_TY,
        TypeTag::U8 => ty::U8_TY,
        TypeTag::U16 => ty::U16_TY,
        TypeTag::U32 => ty::U32_TY,
        TypeTag::U64 => ty::U64_TY,
        TypeTag::U128 => ty::U128_TY,
        TypeTag::U256 => ty::U256_TY,
        TypeTag::I8 => ty::I8_TY,
        TypeTag::I16 => ty::I16_TY,
        TypeTag::I32 => ty::I32_TY,
        TypeTag::I64 => ty::I64_TY,
        TypeTag::I128 => ty::I128_TY,
        TypeTag::I256 => ty::I256_TY,
        TypeTag::Address => ty::ADDRESS_TY,
        TypeTag::Signer => ty::SIGNER_TY,
        TypeTag::Vector(elem) => interner.vector_of(intern_type_tag(elem, interner)?),
        TypeTag::Struct(struct_tag) => intern_struct_tag(struct_tag, interner)?,
        TypeTag::Function(function_tag) => {
            let args = intern_function_param_tags(&function_tag.args, interner)?;
            let results = intern_function_param_tags(&function_tag.results, interner)?;
            interner.function_of(
                interner.type_list_of(&args),
                interner.type_list_of(&results),
                function_tag.abilities,
            )
        },
    })
}

/// Interns a function tag's parameter or return tags, applying each one's
/// reference kind.
fn intern_function_param_tags(
    tags: &[FunctionParamOrReturnTag],
    interner: &impl Interner,
) -> anyhow::Result<Vec<InternedType>> {
    tags.iter()
        .map(|tag| {
            Ok(match tag {
                FunctionParamOrReturnTag::Value(tag) => intern_type_tag(tag, interner)?,
                FunctionParamOrReturnTag::Reference(tag) => {
                    interner.immut_ref_of(intern_type_tag(tag, interner)?)
                },
                FunctionParamOrReturnTag::MutableReference(tag) => {
                    interner.mut_ref_of(intern_type_tag(tag, interner)?)
                },
            })
        })
        .collect()
}

/// Interns a struct tag into its nominal type.
pub fn intern_struct_tag(
    struct_tag: &StructTag,
    interner: &impl Interner,
) -> anyhow::Result<InternedType> {
    let module_id = interner.module_id_of(&struct_tag.address, struct_tag.module.as_ident_str());
    let name = interner.identifier_of(struct_tag.name.as_ident_str());
    let args = struct_tag
        .type_args
        .iter()
        .map(|arg| intern_type_tag(arg, interner))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(interner.nominal_of(module_id, name, interner.type_list_of(&args)))
}

fn intern_struct_info(
    idx: StructHandleIndex,
    module: &CompiledModule,
    interner: &impl Interner,
) -> (InternedModuleId, InternedIdentifier) {
    let struct_handle = module.struct_handle_at(idx);
    intern_struct_handle(struct_handle, module, interner)
}

fn intern_struct_handle(
    struct_handle: &StructHandle,
    module: &CompiledModule,
    interner: &impl Interner,
) -> (InternedModuleId, InternedIdentifier) {
    let module_handle = module.module_handle_at(struct_handle.module);
    let address = module.address_identifier_at(module_handle.address);
    let module_name = module.identifier_at(module_handle.name);
    let struct_name = module.identifier_at(struct_handle.name);

    let module_id = interner.module_id_of(address, module_name);
    let struct_name = interner.identifier_of(struct_name);
    (module_id, struct_name)
}
