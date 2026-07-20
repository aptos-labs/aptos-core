// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interning APIs.

use crate::{
    types::{view_name, view_type, view_type_list, InternedType, InternedTypeList, Type},
    ExecutionErrorKind, IntoExecutionError,
};
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{StructTag, TypeTag},
};
use thiserror::Error;

/// Pointer to interned Move identifier allocated in global arena.
pub type InternedIdentifier = GlobalArenaPtr<str>;

/// Identifies a module or script by its address and name.
pub struct ModuleId {
    address: AccountAddress,
    name: InternedIdentifier,
}

impl ModuleId {
    /// Creates a new module ID.
    pub const fn new(address: AccountAddress, name: InternedIdentifier) -> Self {
        Self { address, name }
    }

    /// Returns the account address of this module.
    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    /// Returns the arena pointer to the name.
    pub fn name(&self) -> InternedIdentifier {
        self.name
    }
}

/// Pointer to interned module ID allocated in global arena.
pub type InternedModuleId = GlobalArenaPtr<ModuleId>;

/// Dereferences an interned module ID.
///
/// # Safety contract
///
/// Same as the other `view_*` helpers: the arena must be alive for as long as
/// the returned reference is used (holds during the execution phase).
pub fn view_module_id(ptr: InternedModuleId) -> &'static ModuleId {
    // SAFETY: see the safety contract above.
    unsafe { ptr.as_ref_unchecked() }
}

/// Symbolic identity of a function: the same `(module, name, type arguments)`
/// triple the loader keys function code on, bundled so a single thin arena
/// pointer can name a function for lazy resolution (e.g. a closure's target).
pub struct FunctionRef {
    pub module_id: InternedModuleId,
    pub func_name: InternedIdentifier,
    pub ty_args: InternedTypeList,
}

/// Pointer to an interned [`FunctionRef`] allocated in the global arena.
pub type InternedFunctionRef = GlobalArenaPtr<FunctionRef>;

/// Dereferences an interned function reference.
///
/// # Safety contract
///
/// Same as the other `view_*` helpers: the arena must be alive for as long as
/// the returned reference is used (holds during the execution phase).
pub fn view_function_ref(ptr: InternedFunctionRef) -> &'static FunctionRef {
    // SAFETY: see the safety contract above.
    unsafe { ptr.as_ref_unchecked() }
}

#[derive(Debug, Clone, Error)]
pub enum TypeSubstitutionError {
    #[error(
        "type parameter index {idx} out of bounds: substitution table has {table_len} entries"
    )]
    IndexOutOfBounds { idx: u16, table_len: usize },
}

impl IntoExecutionError for TypeSubstitutionError {
    fn kind(&self) -> ExecutionErrorKind {
        use TypeSubstitutionError::*;
        match self {
            IndexOutOfBounds { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Constructs interned values, turning each into its canonical,
/// arena-allocated handle, and derives new types by substituting type
/// parameters.
///
/// # Invariant
///
/// Implementations deduplicate allocations, so that pointer equality implies
/// structural equality.
///
/// TODO(metering): enforce a type-size limit (potentially, when interning) so
/// that very large types are not allowed to be created.
pub trait Interner {
    /// Returns a type parameter with the specified index. Note that pointer
    /// equality of any two interned type parameters is structural only. Two
    /// parameters with index 0 but at different scope may represent different
    /// types (but intern to the same pointer).
    fn type_param_of(&self, idx: u16) -> InternedType;

    /// Returns a vector of the specified type.
    fn vector_of(&self, elem: InternedType) -> InternedType;

    /// Returns an immutable reference to the specified type.
    fn immut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a mutable reference to the specified type.
    fn mut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a function type with the given argument and result type lists
    /// and ability set.
    fn function_of(
        &self,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    ) -> InternedType;

    /// Returns an interned list of types.
    fn type_list_of(&self, types: &[InternedType]) -> InternedTypeList;

    /// Returns the interned nominal (struct or enum) identity.
    fn nominal_of(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedType;

    /// Returns the interned function reference identity `(module, name, type
    /// arguments)`. This is the loader's function-code key bundled behind one
    /// thin arena pointer.
    fn function_ref_of(
        &self,
        module_id: InternedModuleId,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedFunctionRef;

    /// Returns the interned IR corresponding to (address, module name) pair
    /// that identifies a module.
    fn module_id_of(&self, address: &AccountAddress, name: &IdentStr) -> InternedModuleId;

    /// Returns an interned string identifier.
    fn identifier_of(&self, identifier: &IdentStr) -> InternedIdentifier;

    /// Substitutes type parameters in the given type using type arguments as
    /// the substitution (indexed by indices in type param nodes). Returns an
    /// error if substitution fails.
    ///
    /// # Invariants
    ///
    /// 1. Every type as index `i` in type argument list corresponds to type
    ///    parameter `i` in the generic type.
    /// 2. Size of the type argument list can be greater than the largest type
    ///    parameter `i` in the generic type. It should never be smaller. If
    ///    so, then substitution fails.
    fn subst_type(
        &self,
        ty: InternedType,
        ty_args: InternedTypeList,
    ) -> Result<InternedType, TypeSubstitutionError>;

    /// Substitutes type parameters in every element of the given type list.
    /// Returns an error if substitution fails.
    ///
    /// # Invariants
    ///
    /// 1. Every type as index `i` in type argument list corresponds to type
    ///    parameter `i` in the generic type list.
    /// 2. Size of the type argument list can be greater than the largest type
    ///    parameter `i` in the generic type list. It should never be smaller.
    ///    If so, then substitution fails.
    fn subst_type_list(
        &self,
        tys: InternedTypeList,
        ty_args: InternedTypeList,
    ) -> Result<InternedTypeList, TypeSubstitutionError>;
}

/// The [`TypeTag`] for an interned type, or [`None`] for types that have no tag
/// representation (references, functions, and unsubstituted type parameters).
///
/// TODO(perf): cache the tag per interned type (e.g. in the global context)
/// instead of re-walking the type graph on every call.
///
/// TODO(correctness): add runtime bounds here, or ensure the transaction already
/// charges gas for the tag and guarantees this conversion is infallible.
pub fn type_tag_of(ty: InternedType) -> Option<TypeTag> {
    Some(match view_type(ty) {
        Type::Bool => TypeTag::Bool,
        Type::U8 => TypeTag::U8,
        Type::U16 => TypeTag::U16,
        Type::U32 => TypeTag::U32,
        Type::U64 => TypeTag::U64,
        Type::U128 => TypeTag::U128,
        Type::U256 => TypeTag::U256,
        Type::I8 => TypeTag::I8,
        Type::I16 => TypeTag::I16,
        Type::I32 => TypeTag::I32,
        Type::I64 => TypeTag::I64,
        Type::I128 => TypeTag::I128,
        Type::I256 => TypeTag::I256,
        Type::Address => TypeTag::Address,
        Type::Signer => TypeTag::Signer,
        Type::Vector { elem } => TypeTag::Vector(Box::new(type_tag_of(*elem)?)),
        Type::Nominal { .. } => TypeTag::Struct(Box::new(struct_tag_of(ty)?)),
        Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => return None,
    })
}

/// The [`StructTag`] for an interned nominal (struct/enum) type, or [`None`] if
/// `ty` is not nominal.
pub fn struct_tag_of(ty: InternedType) -> Option<StructTag> {
    let Type::Nominal {
        module_id,
        name,
        ty_args,
    } = view_type(ty)
    else {
        return None;
    };
    let module_id = view_module_id(*module_id);
    let type_args = view_type_list(*ty_args)
        .iter()
        .map(|arg| type_tag_of(*arg))
        .collect::<Option<Vec<_>>>()?;
    Some(StructTag {
        address: *module_id.address(),
        module: Identifier::new(view_name(module_id.name())).ok()?,
        name: Identifier::new(view_name(*name)).ok()?,
        type_args,
    })
}
