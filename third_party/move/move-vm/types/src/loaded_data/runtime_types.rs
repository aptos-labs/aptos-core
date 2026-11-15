// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::non_canonical_partial_ord_impl)]

use crate::{
    loaded_data::struct_name_indexing::StructNameIndex,
    module_id_interner::{InternedModuleId, InternedModuleIdPool},
    ty_interner::TypeId,
};
use derivative::Derivative;
use itertools::Itertools;
use move_binary_format::{
    errors::{excess_function_type_abilities_error, PartialVMError, PartialVMResult},
    file_format::{
        SignatureToken, StructHandle, StructTypeParameter, TypeParameterIndex, VariantIndex,
    },
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    function::ClosureMask,
    identifier::Identifier,
    language_storage::{FunctionParamOrReturnTag, FunctionTag, ModuleId, StructTag, TypeTag},
    vm_status::{sub_status::unknown_invariant_violation::EPARANOID_FAILURE, StatusCode},
};
use once_cell::sync::OnceCell;
use serde::Serialize;
use smallbitvec::SmallBitVec;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::max,
    collections::{btree_map, BTreeMap},
    fmt,
    fmt::Debug,
    sync::Arc,
};
use triomphe::Arc as TriompheArc;

/// A formula describing the value depth of a type, using (the depths of) the type parameters as inputs.
///
/// It has the form of `max(CBase, T1 + C1, T2 + C2, ..)` where `Ti` is the depth of the ith type parameter
/// and `Ci` is just some constant.
///
/// This form has a special property: when you compute the max of multiple formulae, you can normalize
/// them into a single formula.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug)]
pub struct DepthFormula {
    pub terms: Vec<(TypeParameterIndex, u64)>, // Ti + Ci
    pub constant: Option<u64>,                 // Cbase
}

impl DepthFormula {
    pub fn constant(constant: u64) -> Self {
        Self {
            terms: vec![],
            constant: Some(constant),
        }
    }

    pub fn type_parameter(tparam: TypeParameterIndex) -> Self {
        Self {
            terms: vec![(tparam, 0)],
            constant: None,
        }
    }

    pub fn normalize(formulas: Vec<Self>) -> Self {
        let mut var_map = BTreeMap::new();
        let mut constant_acc = None;
        for formula in formulas {
            let Self { terms, constant } = formula;
            for (var, cur_factor) in terms {
                var_map
                    .entry(var)
                    .and_modify(|prev_factor| *prev_factor = max(cur_factor, *prev_factor))
                    .or_insert(cur_factor);
            }
            match (constant_acc, constant) {
                (_, None) => (),
                (None, Some(_)) => constant_acc = constant,
                (Some(c1), Some(c2)) => constant_acc = Some(max(c1, c2)),
            }
        }
        Self {
            terms: var_map.into_iter().collect(),
            constant: constant_acc,
        }
    }

    pub fn subst(
        &self,
        mut map: BTreeMap<TypeParameterIndex, DepthFormula>,
    ) -> PartialVMResult<DepthFormula> {
        let Self { terms, constant } = self;
        let mut formulas = vec![];
        if let Some(constant) = constant {
            formulas.push(DepthFormula::constant(*constant))
        }
        for (t_i, c_i) in terms {
            let Some(u_form) = map.remove(t_i) else {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("{t_i:?} missing mapping")),
                );
            };
            formulas.push(u_form.scale(*c_i))
        }
        Ok(DepthFormula::normalize(formulas))
    }

    pub fn solve(&self, tparam_depths: &[u64]) -> PartialVMResult<u64> {
        let Self { terms, constant } = self;
        let mut depth = constant.as_ref().copied().unwrap_or(0);
        for (t_i, c_i) in terms {
            let tparam_depth = tparam_depths.get(*t_i as usize).ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "type parameter index {} out of bounds (len {}) when solving depth formula",
                        t_i,
                        tparam_depths.len(),
                    ),
                )
            })?;
            depth = max(depth, tparam_depth.saturating_add(*c_i))
        }
        Ok(depth)
    }

    pub fn scale(mut self, c: u64) -> Self {
        let Self { terms, constant } = &mut self;
        for (_t_i, c_i) in terms {
            *c_i = (*c_i).saturating_add(c);
        }
        if let Some(cbase) = constant.as_mut() {
            *cbase = (*cbase).saturating_add(c);
        }
        self
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct StructType {
    pub idx: StructNameIndex,
    pub layout: StructLayout,
    pub phantom_ty_params_mask: SmallBitVec,
    pub abilities: AbilitySet,
    pub ty_params: Vec<StructTypeParameter>,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum StructLayout {
    Single(Vec<(Identifier, Type)>),
    Variants(Vec<(Identifier, Vec<(Identifier, Type)>)>),
}

impl StructType {
    /// Get the fields from this struct type. If this is a proper struct, the `variant`
    /// must be None. Otherwise if its a variant struct, the variant for which the fields
    /// are requested must be given. For non-matching parameters, the function returns
    /// an empty list.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn fields(&self, variant: Option<VariantIndex>) -> PartialVMResult<&[(Identifier, Type)]> {
        match (&self.layout, variant) {
            (StructLayout::Single(fields), None) => Ok(fields.as_slice()),
            (StructLayout::Variants(variants), Some(variant))
                if (variant as usize) < variants.len() =>
            {
                Ok(variants[variant as usize].1.as_slice())
            },
            _ => Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "inconsistent struct field query: not a variant struct, or variant index out bounds"
                        .to_string(),
                ),
            ),
        }
    }

    /// Selects the field information from this struct type at the given offset. Returns
    /// error if field is not defined.
    pub fn field_at(
        &self,
        variant: Option<VariantIndex>,
        offset: usize,
    ) -> PartialVMResult<&(Identifier, Type)> {
        let slice = self.fields(variant)?;
        if offset < slice.len() {
            Ok(&slice[offset])
        } else {
            Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "field offset out of bounds -- len {} got {}",
                        slice.len(),
                        offset
                    ),
                ),
            )
        }
    }

    /// Same as `struct_type.fields(variant_opt).len()`
    pub fn field_count(&self, variant: Option<VariantIndex>) -> u16 {
        match (&self.layout, variant) {
            (StructLayout::Single(fields), None) => fields.len() as u16,
            (StructLayout::Variants(variants), Some(variant))
                if (variant as usize) < variants.len() =>
            {
                variants[variant as usize].1.len() as u16
            },
            _ => 0,
        }
    }

    /// Returns a string for the variant for error messages. If this is
    /// not a type with this variant, returns a string anyway indicating
    /// its undefined.
    pub fn variant_name_for_message(&self, variant: VariantIndex) -> String {
        let variant = variant as usize;
        match &self.layout {
            StructLayout::Variants(variants) if variant < variants.len() => {
                variants[variant].0.to_string()
            },
            _ => "<undefined>".to_string(),
        }
    }

    pub fn ty_param_constraints(&self) -> impl ExactSizeIterator<Item = &AbilitySet> {
        self.ty_params.iter().map(|param| &param.constraints)
    }

    // Check if the local struct handle is compatible with the defined struct type.
    pub fn check_compatibility(&self, struct_handle: &StructHandle) -> PartialVMResult<()> {
        if !struct_handle.abilities.is_subset(self.abilities) {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Ability definition of module mismatch".to_string()),
            );
        }

        if self.phantom_ty_params_mask.len() != struct_handle.type_parameters.len()
            || !self
                .phantom_ty_params_mask
                .iter()
                .zip(struct_handle.type_parameters.iter())
                .all(|(defined_is_phantom, local_type_parameter)| {
                    !local_type_parameter.is_phantom || defined_is_phantom
                })
        {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "Phantom type parameter definition of module mismatch".to_string(),
                ),
            );
        }

        Ok(())
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn for_test() -> StructType {
        Self {
            idx: StructNameIndex::new(0),
            layout: StructLayout::Single(vec![]),
            phantom_ty_params_mask: SmallBitVec::new(),
            abilities: AbilitySet::EMPTY,
            ty_params: vec![],
        }
    }
}

// `interned_module_id` is a pool-local cache key, not part of struct identity.
// Equality, hashing, and ordering use the canonical `(module, name)` pair.
#[derive(Derivative)]
#[derivative(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct StructIdentifier {
    module: ModuleId,
    #[derivative(
        Debug = "ignore",
        PartialEq = "ignore",
        Hash = "ignore",
        Ord = "ignore",
        PartialOrd = "ignore"
    )]
    interned_module_id: InternedModuleId,
    name: Identifier,
}

impl StructIdentifier {
    pub fn new(module_id_pool: &InternedModuleIdPool, module: ModuleId, name: Identifier) -> Self {
        let interned_module_id = module_id_pool.intern_by_ref(&module);
        Self {
            module,
            interned_module_id,
            name,
        }
    }

    pub fn module(&self) -> &ModuleId {
        &self.module
    }

    pub fn interned_module_id(&self) -> InternedModuleId {
        self.interned_module_id
    }

    pub fn name(&self) -> &Identifier {
        &self.name
    }

    pub fn into_module_and_name(self) -> (ModuleId, Identifier) {
        (self.module, self.name)
    }
}

/// Size, depth and abilities of a type, computed when the type is constructed so runtime checks
/// never walk it. Derived from the structure, so ignored by equality and hashing.
///
/// `num_nodes` and `max_depth` count every node, while [TypeBuilder] limits do not count some
/// references, and they saturate at `u16::MAX`. So a limit check may accept a type based on
/// them, but must recount exactly before rejecting it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TypeInfo {
    num_nodes: u16,
    max_depth: u16,
    abilities: AbilitySet,
    /// Bit set of [Self::HAS_TY_PARAMS] and [Self::ABILITIES_UNKNOWN].
    flags: u8,
}

impl TypeInfo {
    /// Abilities could not be determined; [Type::abilities] computes them.
    const ABILITIES_UNKNOWN: u8 = 2;
    /// The type contains a [Type::TyParam].
    const HAS_TY_PARAMS: u8 = 1;

    const fn leaf(abilities: AbilitySet) -> Self {
        Self {
            num_nodes: 1,
            max_depth: 1,
            abilities,
            flags: 0,
        }
    }

    const fn ty_param() -> Self {
        Self {
            num_nodes: 1,
            max_depth: 1,
            abilities: AbilitySet::EMPTY,
            flags: Self::HAS_TY_PARAMS | Self::ABILITIES_UNKNOWN,
        }
    }

    fn compound<'a>(
        children: impl Iterator<Item = &'a Type>,
        abilities: Option<AbilitySet>,
    ) -> Self {
        let mut num_nodes: u16 = 1;
        let mut max_depth: u16 = 0;
        let mut flags = 0;
        for child in children {
            let child = child.type_info();
            num_nodes = num_nodes.saturating_add(child.num_nodes);
            max_depth = max_depth.max(child.max_depth);
            flags |= child.flags & Self::HAS_TY_PARAMS;
        }
        let abilities = match abilities {
            Some(abilities) => abilities,
            None => {
                flags |= Self::ABILITIES_UNKNOWN;
                AbilitySet::EMPTY
            },
        };
        Self {
            num_nodes,
            max_depth: max_depth.saturating_add(1),
            abilities,
            flags,
        }
    }

    #[inline(always)]
    pub fn num_nodes(&self) -> usize {
        self.num_nodes as usize
    }

    #[inline(always)]
    pub fn max_depth(&self) -> usize {
        self.max_depth as usize
    }

    #[inline(always)]
    pub fn has_ty_params(&self) -> bool {
        self.flags & Self::HAS_TY_PARAMS != 0
    }

    /// [None] for generic or malformed types.
    #[inline(always)]
    pub fn cached_abilities(&self) -> Option<AbilitySet> {
        if self.flags & Self::ABILITIES_UNKNOWN == 0 {
            Some(self.abilities)
        } else {
            None
        }
    }

    #[inline(always)]
    fn is_saturated(&self) -> bool {
        self.num_nodes == u16::MAX || self.max_depth == u16::MAX
    }
}

/// Shared payload of a vector or reference type. Only the child type takes part in equality,
/// hashing and formatting; the rest is derived.
#[derive(Clone)]
pub struct TypeNode {
    ty: Type,
    info: TypeInfo,
    /// Interned id, filled lazily and used as an equality fast path. Never filled for generic
    /// types. The pool is only flushed together with the module cache, between block executions
    /// (`RuntimeEnvironment::flush_all_caches`), which drops every loaded type; types must not
    /// outlive that.
    cached_ty_id: OnceCell<TypeId>,
}

impl TypeNode {
    #[inline(always)]
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    #[inline(always)]
    pub fn cached_ty_id(&self) -> Option<TypeId> {
        self.cached_ty_id.get().copied()
    }
}

impl std::ops::Deref for TypeNode {
    type Target = Type;

    #[inline(always)]
    fn deref(&self) -> &Type {
        &self.ty
    }
}

impl PartialEq for TypeNode {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        // Interned ids are canonical: two filled ids decide equality.
        if let (Some(a), Some(b)) = (self.cached_ty_id.get(), other.cached_ty_id.get()) {
            return a == b;
        }
        self.ty == other.ty
    }
}

impl Eq for TypeNode {}

impl std::hash::Hash for TypeNode {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ty.hash(state)
    }
}

impl fmt::Debug for TypeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.ty, f)
    }
}

impl fmt::Display for TypeNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.ty, f)
    }
}

/// Shared payload of a struct instantiation; cached values as in [TypeNode].
#[derive(Clone)]
pub struct StructInstNode {
    idx: StructNameIndex,
    ty_args: Vec<Type>,
    ability: AbilityInfo,
    info: TypeInfo,
    cached_ty_id: OnceCell<TypeId>,
}

impl StructInstNode {
    #[inline(always)]
    pub fn idx(&self) -> StructNameIndex {
        self.idx
    }

    #[inline(always)]
    pub fn ty_args(&self) -> &[Type] {
        &self.ty_args
    }

    #[inline(always)]
    pub fn ability(&self) -> &AbilityInfo {
        &self.ability
    }

    #[inline(always)]
    pub fn cached_ty_id(&self) -> Option<TypeId> {
        self.cached_ty_id.get().copied()
    }
}

impl PartialEq for StructInstNode {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.cached_ty_id.get(), other.cached_ty_id.get()) {
            return a == b;
        }
        self.idx == other.idx && self.ty_args == other.ty_args
    }
}

impl Eq for StructInstNode {}

impl std::hash::Hash for StructInstNode {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.ty_args.hash(state)
    }
}

impl fmt::Debug for StructInstNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StructInstantiation")
            .field("idx", &self.idx)
            .field("ty_args", &self.ty_args)
            .finish()
    }
}

/// Shared payload of a function type; cached values as in [TypeNode].
#[derive(Clone)]
pub struct FunctionNode {
    args: Vec<Type>,
    results: Vec<Type>,
    abilities: AbilitySet,
    info: TypeInfo,
    cached_ty_id: OnceCell<TypeId>,
}

impl FunctionNode {
    #[inline(always)]
    pub fn args(&self) -> &[Type] {
        &self.args
    }

    #[inline(always)]
    pub fn results(&self) -> &[Type] {
        &self.results
    }

    #[inline(always)]
    pub fn abilities(&self) -> AbilitySet {
        self.abilities
    }

    #[inline(always)]
    pub fn cached_ty_id(&self) -> Option<TypeId> {
        self.cached_ty_id.get().copied()
    }
}

impl PartialEq for FunctionNode {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.cached_ty_id.get(), other.cached_ty_id.get()) {
            return a == b;
        }
        self.abilities == other.abilities
            && self.args == other.args
            && self.results == other.results
    }
}

impl Eq for FunctionNode {}

impl std::hash::Hash for FunctionNode {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.abilities.hash(state);
        self.args.hash(state);
        self.results.hash(state)
    }
}

impl fmt::Debug for FunctionNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Function")
            .field("args", &self.args)
            .field("results", &self.results)
            .field("abilities", &self.abilities)
            .finish()
    }
}

/// The interner's access to a node's id cell. Never fill a generic node.
pub(crate) trait InternedNode {
    fn interned_info(&self) -> TypeInfo;
    fn cached_id(&self) -> Option<TypeId>;
    fn fill_id(&self, id: TypeId);
}

macro_rules! impl_interned_node {
    ($node:ty) => {
        impl InternedNode for $node {
            #[inline(always)]
            fn interned_info(&self) -> TypeInfo {
                self.info
            }

            #[inline(always)]
            fn cached_id(&self) -> Option<TypeId> {
                self.cached_ty_id.get().copied()
            }

            #[inline(always)]
            fn fill_id(&self, id: TypeId) {
                let _ = self.cached_ty_id.set(id);
            }
        }
    };
}

impl_interned_node!(TypeNode);
impl_interned_node!(StructInstNode);
impl_interned_node!(FunctionNode);

// CAUTION: Do not derive/implement `Ord`/`PartialOrd` because `StructNameIndex`
// is not guaranteed to be deterministic.
//
// Compound variants must only be built through the `Type::*` constructors below, which compute
// the cached metadata.
#[derive(Derivative)]
#[derivative(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Type {
    Bool,
    U8,
    U64,
    U128,
    Address,
    Signer,
    Vector(TriompheArc<TypeNode>),
    Struct {
        idx: StructNameIndex,
        ability: AbilityInfo,
    },
    StructInstantiation(TriompheArc<StructInstNode>),
    Function(TriompheArc<FunctionNode>),
    Reference(TriompheArc<TypeNode>),
    MutableReference(TriompheArc<TypeNode>),
    TyParam(u16),
    U16,
    U32,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
}

struct TypePreorderTraversalIterWithDepth<'a> {
    stack: SmallVec<[(&'a Type, usize); 32]>,
}

impl<'a> Iterator for TypePreorderTraversalIterWithDepth<'a> {
    type Item = (&'a Type, usize);

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn next(&mut self) -> Option<Self::Item> {
        use Type::*;

        match self.stack.pop() {
            Some((ty, depth)) => {
                match ty {
                    Signer
                    | Bool
                    | Address
                    | U8
                    | U16
                    | U32
                    | U64
                    | U128
                    | U256
                    | I8
                    | I16
                    | I32
                    | I64
                    | I128
                    | I256
                    | Struct { .. }
                    | TyParam(..) => (),

                    Reference(inner_ty) | MutableReference(inner_ty) => {
                        self.stack.push((inner_ty, depth + 1));
                    },

                    Vector(inner_ty) => {
                        self.stack.push((inner_ty, depth + 1));
                    },

                    StructInstantiation(node) => {
                        for ty_arg in node.ty_args.iter().rev() {
                            self.stack.push((ty_arg, depth + 1));
                        }
                    },

                    Function(node) => {
                        for arg in node.args.iter() {
                            self.stack.push((arg, depth + 1));
                        }
                        for result in node.results.iter() {
                            self.stack.push((result, depth + 1));
                        }
                    },
                }
                Some((ty, depth))
            },
            None => None,
        }
    }
}

// Cache for the ability of struct. They will be ignored when comparing equality or Ord as they are just used for caching purpose.
#[derive(Derivative)]
#[derivative(Debug, Clone, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct AbilityInfo {
    #[derivative(
        PartialEq = "ignore",
        Hash = "ignore",
        Ord = "ignore",
        PartialOrd = "ignore"
    )]
    base_ability_set: AbilitySet,

    #[derivative(
        PartialEq = "ignore",
        Hash = "ignore",
        Ord = "ignore",
        PartialOrd = "ignore"
    )]
    phantom_ty_args_mask: SmallBitVec,
}

impl AbilityInfo {
    pub fn struct_(ability: AbilitySet) -> Self {
        Self {
            base_ability_set: ability,
            phantom_ty_args_mask: SmallBitVec::new(),
        }
    }

    pub fn generic_struct(base_ability_set: AbilitySet, phantom_ty_args_mask: SmallBitVec) -> Self {
        Self {
            base_ability_set,
            phantom_ty_args_mask,
        }
    }
}

macro_rules! paranoid_failure {
    ($msg:ident) => {
        Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message($msg)
                .with_sub_status(EPARANOID_FAILURE),
        )
    };
}

impl Type {
    pub fn to_short_string(&self) -> &'static str {
        use Type::*;
        match self {
            Bool => "bool",
            U8 => "u8",
            U16 => "u16",
            U32 => "u32",
            U64 => "u64",
            U128 => "u128",
            U256 => "u256",
            I8 => "i8",
            I16 => "i16",
            I32 => "i32",
            I64 => "i64",
            I128 => "i128",
            I256 => "i256",
            Address => "address",
            Signer => "signer",
            Vector(_) => "vector",
            Struct { .. } | StructInstantiation { .. } => "struct",
            Function { .. } => "function",
            Reference(_) => "reference",
            MutableReference(_) => "mutable reference",
            TyParam(_) => "type parameter",
        }
    }

    #[inline(always)]
    pub fn type_info(&self) -> TypeInfo {
        match self {
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
            | Type::Address => TypeInfo::leaf(AbilitySet::PRIMITIVES),
            Type::Signer => TypeInfo::leaf(AbilitySet::SIGNER),
            Type::TyParam(_) => TypeInfo::ty_param(),
            Type::Struct { ability, .. } => TypeInfo::leaf(ability.base_ability_set),
            Type::Vector(node) | Type::Reference(node) | Type::MutableReference(node) => node.info,
            Type::StructInstantiation(node) => node.info,
            Type::Function(node) => node.info,
        }
    }

    pub fn vector(elem_ty: Type) -> Type {
        let abilities = elem_ty
            .type_info()
            .cached_abilities()
            .map(|elem_abilities| AbilitySet::VECTOR.intersect(Self::required_by(elem_abilities)));
        let info = TypeInfo::compound(std::iter::once(&elem_ty), abilities);
        Type::Vector(TriompheArc::new(TypeNode {
            ty: elem_ty,
            info,
            cached_ty_id: OnceCell::new(),
        }))
    }

    pub fn reference(inner_ty: Type) -> Type {
        let info = TypeInfo::compound(std::iter::once(&inner_ty), Some(AbilitySet::REFERENCES));
        Type::Reference(TriompheArc::new(TypeNode {
            ty: inner_ty,
            info,
            cached_ty_id: OnceCell::new(),
        }))
    }

    pub fn mut_reference(inner_ty: Type) -> Type {
        let info = TypeInfo::compound(std::iter::once(&inner_ty), Some(AbilitySet::REFERENCES));
        Type::MutableReference(TriompheArc::new(TypeNode {
            ty: inner_ty,
            info,
            cached_ty_id: OnceCell::new(),
        }))
    }

    pub fn struct_instantiation(
        idx: StructNameIndex,
        ty_args: Vec<Type>,
        ability: AbilityInfo,
    ) -> Type {
        let abilities = Self::struct_instantiation_abilities_from_cache(&ability, &ty_args);
        let info = TypeInfo::compound(ty_args.iter(), abilities);
        Type::StructInstantiation(TriompheArc::new(StructInstNode {
            idx,
            ty_args,
            ability,
            info,
            cached_ty_id: OnceCell::new(),
        }))
    }

    pub fn function(args: Vec<Type>, results: Vec<Type>, abilities: AbilitySet) -> Type {
        let info = TypeInfo::compound(args.iter().chain(results.iter()), Some(abilities));
        Type::Function(TriompheArc::new(FunctionNode {
            args,
            results,
            abilities,
            info,
            cached_ty_id: OnceCell::new(),
        }))
    }

    #[inline(always)]
    fn required_by(abilities: AbilitySet) -> AbilitySet {
        abilities
            .into_iter()
            .map(|a| a.required_by())
            .fold(AbilitySet::EMPTY, AbilitySet::union)
    }

    /// [None] for generic or malformed types, which [Type::abilities] handles as before.
    fn struct_instantiation_abilities_from_cache(
        ability: &AbilityInfo,
        ty_args: &[Type],
    ) -> Option<AbilitySet> {
        if ability.phantom_ty_args_mask.len() != ty_args.len() {
            return None;
        }
        let mut abilities = ability.base_ability_set;
        for (ty_arg, is_phantom) in ty_args.iter().zip(ability.phantom_ty_args_mask.iter()) {
            let arg_abilities = ty_arg.type_info().cached_abilities()?;
            if !is_phantom {
                abilities = abilities.intersect(Self::required_by(arg_abilities));
            }
        }
        Some(abilities)
    }

    #[cfg(any(test, debug_assertions))]
    pub fn has_consistent_type_info(&self) -> bool {
        self.preorder_traversal_with_depth().all(|(ty, _)| {
            let info = ty.type_info();
            if info.is_saturated() {
                return true;
            }
            let (num_nodes, max_depth) = ty.num_nodes_with_max_depth_uncached();
            let has_ty_params = ty
                .preorder_traversal_with_depth()
                .any(|(t, _)| matches!(t, Type::TyParam(_)));
            let abilities_ok = match info.cached_abilities() {
                Some(abilities) => ty.abilities_uncached().ok() == Some(abilities),
                None => true,
            };
            info.num_nodes() == num_nodes
                && info.max_depth() == max_depth
                && info.has_ty_params() == has_ty_params
                && abilities_ok
        })
    }

    pub fn verify_ty_arg_abilities<'a, I>(
        ty_param_abilities: I,
        ty_args: &[Self],
    ) -> PartialVMResult<()>
    where
        I: IntoIterator<Item = &'a AbilitySet>,
        I::IntoIter: ExactSizeIterator,
    {
        let ty_param_abilities = ty_param_abilities.into_iter();
        if ty_param_abilities.len() != ty_args.len() {
            return Err(PartialVMError::new(
                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
            ));
        }
        for (ty, expected_ability_set) in ty_args.iter().zip(ty_param_abilities) {
            if !expected_ability_set.is_subset(ty.abilities()?) {
                return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED));
            }
        }
        Ok(())
    }

    /// Returns true if the type is a signer or an immutable signer reference type. Returns false
    /// otherwise.
    pub fn is_signer_or_signer_ref(&self) -> bool {
        use Type::*;
        match self {
            Signer => true,
            Reference(inner_ty) => matches!(inner_ty.ty(), Signer),
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | I8
            | I16
            | I32
            | I64
            | I128
            | I256
            | Address
            | Vector(_)
            | Struct { .. }
            | StructInstantiation { .. }
            | Function { .. }
            | MutableReference(_)
            | TyParam(_) => false,
        }
    }

    /// Returns true if the type is a nominal datatype, i.e. a struct or an enum (instantiated or
    /// not). At the runtime-type level enums share the `Struct`/`StructInstantiation`
    /// representation, so both source-level structs and enums are covered.
    pub fn is_struct_or_enum(&self) -> bool {
        use Type::*;
        match self {
            Struct { .. } | StructInstantiation { .. } => true,
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | I8
            | I16
            | I32
            | I64
            | I128
            | I256
            | Address
            | Signer
            | Vector(_)
            | Function { .. }
            | Reference(_)
            | MutableReference(_)
            | TyParam(_) => false,
        }
    }

    pub fn paranoid_check_is_no_ref(&self, msg: &str) -> PartialVMResult<()> {
        if matches!(self, Type::Reference(_) | Type::MutableReference(_)) {
            let msg = format!("{} `{}` cannot be a reference", msg, self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_bool_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::Bool) {
            let msg = format!("Expected boolean type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_is_sint_ty(&self) -> PartialVMResult<()> {
        if !matches!(
            self,
            Self::I8 | Self::I16 | Self::I32 | Self::I64 | Self::I128 | Self::I256
        ) {
            let msg = format!("Expected signed integer type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_is_u64_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::U64) {
            let msg = format!("Expected U64 type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_address_ty(&self) -> PartialVMResult<()> {
        if !matches!(self, Self::Address) {
            let msg = format!("Expected address type, got {}", self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    pub fn paranoid_check_is_signer_ref_ty(&self) -> PartialVMResult<()> {
        if let Self::Reference(inner_ty) = self {
            if matches!(inner_ty.ty(), Self::Signer) {
                return Ok(());
            }
        }
        let msg = format!("Expected &signer type, got {}", self);
        paranoid_failure!(msg)
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_has_ability(&self, ability: Ability) -> PartialVMResult<()> {
        if !self.abilities()?.has_ability(ability) {
            let msg = format!("Type {} does not have expected ability {}", self, ability);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_abilities(&self, expected_abilities: AbilitySet) -> PartialVMResult<()> {
        let abilities = self.abilities()?;
        if !expected_abilities.is_subset(abilities) {
            let msg = format!(
                "Type {} has unexpected ability: expected {}, got {}",
                self, expected_abilities, abilities
            );
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_eq(&self, expected_ty: &Self) -> PartialVMResult<()> {
        if self != expected_ty {
            let msg = format!("Expected type {}, got {}", expected_ty, self);
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_assignable(&self, expected_ty: &Self) -> PartialVMResult<()> {
        let ok = match (expected_ty, self) {
            (Type::Function(expected), Type::Function(given)) => {
                expected.abilities.is_subset(given.abilities)
                    && (TriompheArc::ptr_eq(expected, given)
                        || matches!(
                            (expected.cached_ty_id(), given.cached_ty_id()),
                            (Some(a), Some(b)) if a == b
                        )
                        || (expected.args == given.args && expected.results == given.results))
            },
            (Type::Reference(ty), Type::Reference(given)) => {
                given.paranoid_check_assignable(ty)?;
                true
            },
            _ => expected_ty == self,
        };
        if !ok {
            let msg = format!(
                "Expected type {}, got {} which is not assignable ",
                expected_ty, self
            );
            return paranoid_failure!(msg);
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_is_vec_ty(&self, expected_elem_ty: &Self) -> PartialVMResult<()> {
        if let Self::Vector(elem_ty) = self {
            return elem_ty.paranoid_check_eq(expected_elem_ty);
        }

        let msg = format!("Expected vector type, got {}", self);
        paranoid_failure!(msg)
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_is_vec_ref_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<()> {
        if let Self::MutableReference(inner_ty) = self {
            if let Self::Vector(elem_ty) = inner_ty.ty() {
                elem_ty.paranoid_check_eq(expected_elem_ty)?;
                return Ok(());
            }
        }

        if let Self::Reference(inner_ty) = self {
            if !IS_MUT {
                if let Self::Vector(elem_ty) = inner_ty.ty() {
                    elem_ty.paranoid_check_eq(expected_elem_ty)?;
                    return Ok(());
                }
            }
        }

        let msg = format!(
            "Expected a (mutable: {}) vector reference, got {}",
            IS_MUT, self
        );
        paranoid_failure!(msg)
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// a (mutable) reference to its element type.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_and_get_vec_elem_ref_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(expected_elem_ty)?;
        let elem_ty = self.get_vec_ref_elem_ty();

        // SAFETY: This type construction satisfies all constraints on size/depth because the parent
        //         vector reference type has been safely constructed.
        Ok(if IS_MUT {
            Type::mut_reference(elem_ty)
        } else {
            Type::reference(elem_ty)
        })
    }

    /// Returns an error if the type is not a (mutable) vector reference. Otherwise, returns
    /// its element type.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_and_get_vec_elem_ty<const IS_MUT: bool>(
        &self,
        expected_elem_ty: &Self,
    ) -> PartialVMResult<Self> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(expected_elem_ty)?;
        Ok(self.get_vec_ref_elem_ty())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    fn get_vec_ref_elem_ty(&self) -> Self {
        match self {
            Self::Reference(inner_ty) | Self::MutableReference(inner_ty) => match inner_ty.ty() {
                Self::Vector(elem_ty) => elem_ty.ty().clone(),
                _ => unreachable!("The inner type must be a vector"),
            },
            _ => unreachable!("The top-level type must be a reference"),
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_freeze_ref_ty(self) -> PartialVMResult<Type> {
        match self {
            Type::MutableReference(ty) => Ok(Type::Reference(ty)),
            _ => {
                let msg = format!("Expected a mutable reference to freeze, got {}", self);
                paranoid_failure!(msg)
            },
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_read_ref(self) -> PartialVMResult<Type> {
        match self {
            Type::Reference(inner_ty) | Type::MutableReference(inner_ty) => {
                inner_ty.paranoid_check_has_ability(Ability::Copy)?;
                Ok(inner_ty.ty().clone())
            },
            _ => {
                let msg = format!("Expected a reference to read, got {}", self);
                paranoid_failure!(msg)
            },
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_write_ref(&self, val_ty: &Type) -> PartialVMResult<()> {
        if let Type::MutableReference(inner_ty) = self {
            val_ty.paranoid_check_assignable(inner_ty)?;
            inner_ty.paranoid_check_has_ability(Ability::Drop)
        } else {
            let msg = format!("Cannot write type {} to immutable type {}", val_ty, self);
            paranoid_failure!(msg)
        }
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn paranoid_check_ref_eq(
        &self,
        expected_inner_ty: &Self,
        is_mut: bool,
    ) -> PartialVMResult<()> {
        match self {
            Type::MutableReference(inner_ty) => inner_ty.paranoid_check_eq(expected_inner_ty),
            Type::Reference(inner_ty) if !is_mut => inner_ty.paranoid_check_eq(expected_inner_ty),
            _ => {
                let msg = format!(
                    "Expected a (mutable: {}) reference type, got {}",
                    is_mut, self
                );
                paranoid_failure!(msg)
            },
        }
    }

    /// If the type is a mutable or immutable reference, returns the inner type it points to.
    /// Otherwise, returns [None].
    pub fn get_ref_inner_ty(&self) -> Option<&Self> {
        match self {
            Type::Reference(ty) | Type::MutableReference(ty) => Some(ty.ty()),
            Type::Bool
            | Type::U8
            | Type::U64
            | Type::U16
            | Type::U32
            | Type::U256
            | Type::U128
            | Type::I8
            | Type::I64
            | Type::I16
            | Type::I32
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::Vector(_)
            | Type::Struct { .. }
            | Type::StructInstantiation { .. }
            | Type::Function { .. }
            | Type::TyParam(_) => None,
        }
    }

    // Note(inline): Recursive function, but `#[cfg_attr(feature = "force-inline", inline(always))]` seems to improve perf slightly
    //               and doesn't add much compile time.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn abilities(&self) -> PartialVMResult<AbilitySet> {
        if let Some(abilities) = self.type_info().cached_abilities() {
            return Ok(abilities);
        }
        self.abilities_uncached()
    }

    /// Slow path for generic or malformed types, keeping their error reporting unchanged.
    fn abilities_uncached(&self) -> PartialVMResult<AbilitySet> {
        match self {
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
            | Type::Address => Ok(AbilitySet::PRIMITIVES),

            // Technically unreachable but, no point in erroring if we don't have to
            Type::Reference(_) | Type::MutableReference(_) => Ok(AbilitySet::REFERENCES),
            Type::Signer => Ok(AbilitySet::SIGNER),

            Type::TyParam(_) => Err(PartialVMError::new(StatusCode::UNREACHABLE).with_message(
                "Unexpected TyParam type after translating from TypeTag to Type".to_string(),
            )),

            Type::Vector(ty) => {
                AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                    ty.abilities_uncached()?
                ])
                .map_err(|e| {
                    PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                        .with_message(e.to_string())
                })
            },
            Type::Struct { ability, .. } => Ok(ability.base_ability_set),
            Type::StructInstantiation(node) => {
                let AbilityInfo {
                    base_ability_set,
                    phantom_ty_args_mask,
                } = &node.ability;
                let type_argument_abilities = node
                    .ty_args
                    .iter()
                    .map(|arg| arg.abilities_uncached())
                    .collect::<PartialVMResult<Vec<_>>>()?;
                AbilitySet::polymorphic_abilities(
                    *base_ability_set,
                    phantom_ty_args_mask.iter(),
                    type_argument_abilities,
                )
                .map_err(|e| {
                    PartialVMError::new(StatusCode::VERIFIER_INVARIANT_VIOLATION)
                        .with_message(e.to_string())
                })
            },
            Type::Function(node) => Ok(node.abilities),
        }
    }

    fn preorder_traversal_with_depth(&self) -> TypePreorderTraversalIterWithDepth<'_> {
        TypePreorderTraversalIterWithDepth {
            stack: smallvec![(self, 1)],
        }
    }

    /// Returns the number of nodes the type has.
    ///
    /// For example
    ///   - `u64` has one node
    ///   - `vector<u64>` has two nodes -- one for the vector and one for the element type u64.
    ///   - `Foo<u64, Bar<u8, bool>>` has 5 nodes.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn num_nodes(&self) -> usize {
        self.num_nodes_with_max_depth().0
    }

    /// Returns the number of nodes the type has and the maximum depth.
    ///
    /// For example
    ///   - `u64` has 1 node and has depth of 1.
    ///   - `vector<u64>` has 2 nodes -- one for the vector and one for the element type u64.
    ///      Its depth is 2.
    ///   - `Foo<u64, Bar<u8, bool>>` has 5 nodes with depth of 3.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn num_nodes_with_max_depth(&self) -> (usize, usize) {
        let info = self.type_info();
        if info.is_saturated() {
            return self.num_nodes_with_max_depth_uncached();
        }
        (info.num_nodes(), info.max_depth())
    }

    fn num_nodes_with_max_depth_uncached(&self) -> (usize, usize) {
        self.preorder_traversal_with_depth()
            .fold((0, 0), |(count, max_depth), (_, depth)| {
                (count + 1, max_depth.max(depth))
            })
    }

    /// Calculates the number of nodes and maximum depth in the substituted type.
    pub fn num_nodes_in_subst(&self, ty_args: &[Type]) -> PartialVMResult<(usize, usize)> {
        use Type::*;

        let num_nodes_in_arg = |idx: usize| -> PartialVMResult<(usize, usize)> {
            let ty = ty_args.get(idx).ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "type substitution failed: index out of bounds -- len {} got {}",
                        ty_args.len(),
                        idx
                    ),
                )
            })?;
            Ok(ty.num_nodes_with_max_depth())
        };

        let mut n = 0;
        let mut max_depth = 0;
        for (ty, depth) in self.preorder_traversal_with_depth() {
            match ty {
                TyParam(idx) => {
                    let (nodes, arg_depth) = num_nodes_in_arg(*idx as usize)?;
                    n += nodes;
                    // When substituting type parameter, we need to subtract 1 from the depth
                    // of the type we substitute in to account for the current depth of the
                    // type parameter node.
                    max_depth = max_depth.max(depth + arg_depth - 1);
                },
                Address
                | Bool
                | Signer
                | U8
                | U16
                | U32
                | U64
                | U128
                | U256
                | I8
                | I16
                | I32
                | I64
                | I128
                | I256
                | Vector(..)
                | Struct { .. }
                | Reference(..)
                | MutableReference(..)
                | StructInstantiation { .. }
                | Function { .. } => {
                    n += 1;
                    max_depth = max_depth.max(depth);
                },
            }
        }

        Ok((n, max_depth))
    }
}

impl fmt::Display for StructIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{}",
            self.module.short_str_lossless(),
            self.name.as_str()
        )
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Type::*;
        match self {
            Bool => f.write_str("bool"),
            U8 => f.write_str("u8"),
            U16 => f.write_str("u16"),
            U32 => f.write_str("u32"),
            U64 => f.write_str("u64"),
            U128 => f.write_str("u128"),
            U256 => f.write_str("u256"),
            I8 => f.write_str("i8"),
            I16 => f.write_str("i16"),
            I32 => f.write_str("i32"),
            I64 => f.write_str("i64"),
            I128 => f.write_str("i128"),
            I256 => f.write_str("i256"),
            Address => f.write_str("address"),
            Signer => f.write_str("signer"),
            Vector(et) => write!(f, "vector<{}>", et),
            Struct { idx, ability: _ } => write!(f, "s#{}", idx),
            StructInstantiation(node) => write!(
                f,
                "s#{}<{}>",
                node.idx,
                node.ty_args.iter().map(|t| t.to_string()).join(",")
            ),
            Function(node) => write!(
                f,
                "|{}|{}{}",
                node.args.iter().map(|t| t.to_string()).join(","),
                node.results.iter().map(|t| t.to_string()).join(","),
                node.abilities.display_postfix()
            ),
            Reference(t) => write!(f, "&{}", t),
            MutableReference(t) => write!(f, "&mut {}", t),
            TyParam(no) => write!(f, "_{}", no),
        }
    }
}

/// Constructs runtime types while enforcing configured size and depth limits.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct TypeBuilder {
    // Maximum number of nodes a fully-instantiated type has.
    max_ty_size: u64,
    // Maximum depth (in terms of number of nodes) a fully-instantiated type has.
    max_ty_depth: u64,
    // Gates changes to type construction algorithm:
    //   - Reference nodes are not counted towards the final node count and depth.
    pub check_depth_on_type_counts_v2: bool,
    // When set, the function-type node is counted towards type size and depth limits.
    pub count_function_type_node: bool,
    ref_ty_count_decrement: u64,
    ref_ty_depth_increment: u64,
    // Enables function-ability validation for `TypeTag` inputs.
    check_function_type_abilities: bool,
}

impl TypeBuilder {
    pub const fn with_limits(
        max_ty_size: u64,
        max_ty_depth: u64,
        check_depth_on_type_counts_v2: bool,
        count_function_type_node: bool,
        check_function_type_abilities: bool,
    ) -> Self {
        Self {
            max_ty_size,
            max_ty_depth,
            check_depth_on_type_counts_v2,
            count_function_type_node,
            // We set count to 0 and depth to 1, which means we do not account for reference node
            // in count/depth calculation. This is fine as we have no nested references, so we only
            // allow 1 extra hop, in the worst case. In return, this guarantees that at runtime
            // taking a reference to some generic T never fails due to exceeding limits: instead,
            // either T exceeds limits and type instantiation fails, or reference can be created
            // successfully.
            ref_ty_count_decrement: if check_depth_on_type_counts_v2 { 1 } else { 0 },
            ref_ty_depth_increment: if check_depth_on_type_counts_v2 { 0 } else { 1 },
            check_function_type_abilities,
        }
    }

    #[inline(always)]
    pub fn create_bool_ty(&self) -> Type {
        Type::Bool
    }

    #[inline(always)]
    pub fn create_u8_ty(&self) -> Type {
        Type::U8
    }

    #[inline(always)]
    pub fn create_u16_ty(&self) -> Type {
        Type::U16
    }

    #[inline(always)]
    pub fn create_u32_ty(&self) -> Type {
        Type::U32
    }

    #[inline(always)]
    pub fn create_u64_ty(&self) -> Type {
        Type::U64
    }

    #[inline(always)]
    pub fn create_u128_ty(&self) -> Type {
        Type::U128
    }

    #[inline(always)]
    pub fn create_u256_ty(&self) -> Type {
        Type::U256
    }

    #[inline(always)]
    pub fn create_i8_ty(&self) -> Type {
        Type::I8
    }

    #[inline(always)]
    pub fn create_i16_ty(&self) -> Type {
        Type::I16
    }

    #[inline(always)]
    pub fn create_i32_ty(&self) -> Type {
        Type::I32
    }

    #[inline(always)]
    pub fn create_i64_ty(&self) -> Type {
        Type::I64
    }

    #[inline(always)]
    pub fn create_i128_ty(&self) -> Type {
        Type::I128
    }

    #[inline(always)]
    pub fn create_i256_ty(&self) -> Type {
        Type::I256
    }

    #[inline(always)]
    pub fn create_address_ty(&self) -> Type {
        Type::Address
    }

    #[inline(always)]
    pub fn create_signer_ty(&self) -> Type {
        Type::Signer
    }

    /// Creates a (possibly mutable) reference type from the given inner type.
    /// Returns an error if the type size or depth are too large.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn create_ref_ty(&self, inner_ty: &Type, is_mut: bool) -> PartialVMResult<Type> {
        self.check_existing_ty(
            inner_ty,
            1 - self.ref_ty_count_decrement,
            1 + self.ref_ty_depth_increment,
        )
        .map_err(|e| {
            e.append_message_with_separator(
                '.',
                format!(
                    "Failed to create a (mutable: {}) reference type with inner type {}",
                    is_mut, inner_ty
                ),
            )
        })?;
        Ok(if is_mut {
            Type::mut_reference(inner_ty.clone())
        } else {
            Type::reference(inner_ty.clone())
        })
    }

    /// Creates a vector type with the given element type, returning an error
    /// if the type size or depth are too large.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn create_vec_ty(&self, elem_ty: &Type) -> PartialVMResult<Type> {
        self.check_existing_ty(elem_ty, 1, 2).map_err(|e| {
            e.append_message_with_separator(
                '.',
                format!(
                    "Failed to create a vector type with element type {}",
                    elem_ty
                ),
            )
        })?;
        Ok(Type::vector(elem_ty.clone()))
    }

    /// Checks that an existing type fits within the limits at `depth` with `count` nodes already
    /// used. The cached count may over-count (reference nodes may be free), so it is only used
    /// to accept; otherwise the exact walk runs, so verdicts and errors are unchanged.
    fn check_existing_ty(&self, ty: &Type, count: u64, depth: u64) -> PartialVMResult<()> {
        let info = ty.type_info();
        if !info.is_saturated() && !info.has_ty_params() {
            let total_count = count.saturating_add(info.num_nodes() as u64);
            let total_depth = depth.saturating_add(info.max_depth() as u64 - 1);
            if total_count <= self.max_ty_size && total_depth <= self.max_ty_depth {
                return Ok(());
            }
        }
        let mut count = count;
        self.check_existing_ty_exact(ty, &mut count, depth)
    }

    /// The limit checks of `clone_impl`, without building.
    fn check_existing_ty_exact(
        &self,
        ty: &Type,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<()> {
        use Type::*;

        self.check_before_count_increment(count, depth)?;
        *count += 1;
        match ty {
            TyParam(idx) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                        "There is an unresolved type parameter (index: {}) when cloning type {}",
                        idx, ty
                    )),
                );
            },
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | I8
            | I16
            | I32
            | I64
            | I128
            | I256
            | Address
            | Signer
            | Struct { .. } => {},
            Vector(elem_ty) => {
                self.check_existing_ty_exact(elem_ty, count, depth + 1)?;
            },
            Reference(inner_ty) | MutableReference(inner_ty) => {
                *count -= self.ref_ty_count_decrement;
                self.check_existing_ty_exact(inner_ty, count, depth + self.ref_ty_depth_increment)?;
            },
            StructInstantiation(node) => {
                for ty_arg in node.ty_args.iter() {
                    self.check_existing_ty_exact(ty_arg, count, depth + 1)?;
                }
            },
            Function(node) => {
                for ty in node.args.iter().chain(node.results.iter()) {
                    self.check_existing_ty_exact(ty, count, depth + 1)?;
                }
            },
        }
        Ok(())
    }

    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn create_struct_ty(&self, idx: StructNameIndex, ability: AbilityInfo) -> Type {
        Type::Struct { idx, ability }
    }

    /// Creates a fully-instantiated struct type, performing the type substitution.
    /// Returns an error if the type size or depth are too large.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn create_struct_instantiation_ty(
        &self,
        struct_ty: &StructType,
        ty_params: &[Type],
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        // We cannot call substitution API directly because we have to take into
        // account struct type itself. We simply shift count and depth by 1 and
        // call inner APIs, to save extra cloning.
        let mut count = 1;
        let check = |c: &mut u64, d: u64| self.check_before_count_increment(c, d);

        let ty_args = ty_params
            .iter()
            .map(|ty| {
                // Note that depth is 2 because we accounted for the parent struct type.
                self.subst_impl(ty, ty_args, &mut count, 2, check)
                    .map_err(|e| {
                        e.append_message_with_separator(
                            '.',
                            format!(
                                "Failed to instantiate a type {} with type arguments {:?}",
                                ty, ty_args
                            ),
                        )
                    })
            })
            .collect::<PartialVMResult<Vec<_>>>()?;

        Ok(Type::struct_instantiation(
            struct_ty.idx,
            ty_args,
            AbilityInfo::generic_struct(
                struct_ty.abilities,
                struct_ty.phantom_ty_params_mask.clone(),
            ),
        ))
    }

    /// Creates a type for a Move constant. Note that constant types can be
    /// more restrictive and therefore have their own creation API.
    #[cfg_attr(feature = "force-inline", inline(always))]
    pub fn create_constant_ty(&self, const_tok: &SignatureToken) -> PartialVMResult<Type> {
        let mut count = 0;
        self.create_constant_ty_impl(const_tok, &mut count, 1)
            .map_err(|e| {
                e.append_message_with_separator(
                    '.',
                    format!(
                        "Failed to construct a type for constant token {:?}",
                        const_tok
                    ),
                )
            })
    }

    /// Creates a fully-instantiated type from its storage representation.
    pub fn create_ty<F>(&self, ty_tag: &TypeTag, mut resolver: F) -> PartialVMResult<Type>
    where
        F: FnMut(&StructTag) -> PartialVMResult<Arc<StructType>>,
    {
        let mut count = 0;
        self.create_ty_impl(ty_tag, &mut resolver, &mut count, 1)
    }

    /// Clones the given type, at the same time instantiating all its type parameters.
    pub fn create_ty_with_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<Type> {
        let mut count = 0;
        let check = |c: &mut u64, d: u64| self.check_before_count_increment(c, d);
        self.subst_impl(ty, ty_args, &mut count, 1, check)
    }

    /// Creates the function type of closure with:
    ///   - non-masked parameter types,
    ///   - all return types.
    /// If type arguments are passed, the type is instantiated.
    ///
    /// Returns an error if type creation fails (e.g., type is too large).
    pub fn create_function_ty(
        &self,
        param_tys: &[Type],
        mask: ClosureMask,
        return_tys: &[Type],
        abilities: AbilitySet,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        // Count the function-type node itself; its arguments and results sit
        // one level below, at depth 2.
        let mut count = 1;
        let check = |c: &mut u64, d: u64| self.check_before_count_increment(c, d);

        let args = mask
            .extract(param_tys, false)
            .into_iter()
            .map(|ty| self.subst_impl(ty, ty_args, &mut count, 2, check))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let results = return_tys
            .iter()
            .map(|ty| self.subst_impl(ty, ty_args, &mut count, 2, check))
            .collect::<PartialVMResult<Vec<_>>>()?;

        Ok(Type::function(args, results, abilities))
    }

    /// Given [`Type`]'s node count and depth, returns an error if these values exceed limits.
    pub fn check_final_size_and_depth(&self, count: u64, depth: u64) -> PartialVMResult<()> {
        // Note: must be greater than because we check for final node count.
        if count > self.max_ty_size {
            return self.too_many_nodes_error();
        }
        if depth > self.max_ty_depth {
            return self.too_large_depth_error();
        }
        Ok(())
    }

    #[inline]
    fn check_before_count_increment(&self, count: &mut u64, depth: u64) -> PartialVMResult<()> {
        // Note: must be greater than or equal to because we check for node count prior to
        // incrementing it.
        if *count >= self.max_ty_size {
            return self.too_many_nodes_error();
        }
        if depth > self.max_ty_depth {
            return self.too_large_depth_error();
        }
        Ok(())
    }

    #[cold]
    fn too_many_nodes_error(&self) -> PartialVMResult<()> {
        Err(
            PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                "Type size is larger than maximum {}",
                self.max_ty_size
            )),
        )
    }

    #[cold]
    fn too_large_depth_error(&self) -> PartialVMResult<()> {
        Err(
            PartialVMError::new(StatusCode::VM_MAX_TYPE_DEPTH_REACHED).with_message(format!(
                "Type depth is larger than maximum {}",
                self.max_ty_depth
            )),
        )
    }

    fn create_constant_ty_impl(
        &self,
        const_tok: &SignatureToken,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<Type> {
        use SignatureToken as S;
        use Type::*;

        self.check_before_count_increment(count, depth)?;
        *count += 1;
        Ok(match const_tok {
            S::Bool => Bool,
            S::U8 => U8,
            S::U16 => U16,
            S::U32 => U32,
            S::U64 => U64,
            S::U128 => U128,
            S::U256 => U256,
            S::I8 => I8,
            S::I16 => I16,
            S::I32 => I32,
            S::I64 => I64,
            S::I128 => I128,
            S::I256 => I256,
            S::Address => Address,
            S::Vector(elem_tok) => {
                let elem_ty = self.create_constant_ty_impl(elem_tok, count, depth + 1)?;
                Type::vector(elem_ty)
            },

            S::Struct(_) | S::StructInstantiation(_, _) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message("Struct constants are not supported".to_string()),
                );
            },

            S::Signer
            | S::Function(..)
            | S::Reference(_)
            | S::MutableReference(_)
            | S::TypeParameter(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                            "{:?} is not allowed or is not a meaningful token for a constant",
                            const_tok
                        )),
                );
            },
        })
    }

    fn subst_impl<G>(
        &self,
        ty: &Type,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        self.apply_subst(
            ty,
            |idx, c, d| match ty_args.get(idx as usize) {
                Some(ty) => {
                    self.check_existing_ty_exact(ty, c, d)?;
                    Ok(ty.clone())
                },
                None => Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                        "Type substitution failed: index {} is out of bounds for {} type arguments",
                        idx,
                        ty_args.len()
                    )),
                ),
            },
            count,
            depth,
            check,
        )
    }

    #[cfg(test)]
    fn clone_impl<G>(
        &self,
        ty: &Type,
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        self.apply_subst(
            ty,
            |idx, _, _| {
                // The type cannot contain type parameters anymore (it also does not make
                // sense to have them!), and so it is the caller's responsibility to ensure
                // type substitution has been performed.
                Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!(
                        "There is an unresolved type parameter (index: {}) when cloning type {}",
                        idx, ty
                    )),
                )
            },
            count,
            depth,
            check,
        )
    }

    fn apply_subst<F, G>(
        &self,
        ty: &Type,
        subst: F,
        count: &mut u64,
        depth: u64,
        check: G,
    ) -> PartialVMResult<Type>
    where
        F: Fn(u16, &mut u64, u64) -> PartialVMResult<Type> + Copy,
        G: Fn(&mut u64, u64) -> PartialVMResult<()> + Copy,
    {
        use Type::*;

        check(count, depth)?;
        *count += 1;
        Ok(match ty {
            TyParam(idx) => {
                // To avoid double-counting, revert counting the type parameter.
                *count -= 1;
                subst(*idx, count, depth)?
            },

            Bool => Bool,
            U8 => U8,
            U16 => U16,
            U32 => U32,
            U64 => U64,
            U128 => U128,
            U256 => U256,
            I8 => I8,
            I16 => I16,
            I32 => I32,
            I64 => I64,
            I128 => I128,
            I256 => I256,
            Address => Address,
            Signer => Signer,
            Vector(elem_ty) => {
                let elem_ty = self.apply_subst(elem_ty, subst, count, depth + 1, check)?;
                Type::vector(elem_ty)
            },
            Reference(inner_ty) => {
                *count -= self.ref_ty_count_decrement;
                let inner_ty = self.apply_subst(
                    inner_ty,
                    subst,
                    count,
                    depth + self.ref_ty_depth_increment,
                    check,
                )?;
                Type::reference(inner_ty)
            },
            MutableReference(inner_ty) => {
                *count -= self.ref_ty_count_decrement;
                let inner_ty = self.apply_subst(
                    inner_ty,
                    subst,
                    count,
                    depth + self.ref_ty_depth_increment,
                    check,
                )?;
                Type::mut_reference(inner_ty)
            },
            Struct { idx, ability } => Struct {
                idx: *idx,
                ability: ability.clone(),
            },
            StructInstantiation(node) => {
                let mut instantiated_tys = vec![];
                for ty in node.ty_args.iter() {
                    let ty = self.apply_subst(ty, subst, count, depth + 1, check)?;
                    instantiated_tys.push(ty);
                }
                Type::struct_instantiation(node.idx, instantiated_tys, node.ability.clone())
            },
            Function(node) => {
                let subs_elem = |count: &mut u64, ty: &Type| -> PartialVMResult<Type> {
                    self.apply_subst(ty, subst, count, depth + 1, check)
                };
                let args = node
                    .args
                    .iter()
                    .map(|ty| subs_elem(count, ty))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                let results = node
                    .results
                    .iter()
                    .map(|ty| subs_elem(count, ty))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                Type::function(args, results, node.abilities)
            },
        })
    }

    fn create_ty_impl<F>(
        &self,
        ty_tag: &TypeTag,
        resolver: &mut F,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<Type>
    where
        F: FnMut(&StructTag) -> PartialVMResult<Arc<StructType>>,
    {
        use Type::*;
        use TypeTag as T;

        self.check_before_count_increment(count, depth)?;
        *count += 1;
        Ok(match ty_tag {
            T::Bool => Bool,
            T::U8 => U8,
            T::U16 => U16,
            T::U32 => U32,
            T::U64 => U64,
            T::U128 => U128,
            T::U256 => U256,
            T::I8 => I8,
            T::I16 => I16,
            T::I32 => I32,
            T::I64 => I64,
            T::I128 => I128,
            T::I256 => I256,
            T::Address => Address,
            T::Signer => Signer,
            T::Vector(elem_ty_tag) => {
                let elem_ty = self.create_ty_impl(elem_ty_tag, resolver, count, depth + 1)?;
                Type::vector(elem_ty)
            },
            T::Struct(struct_tag) => {
                let struct_ty = resolver(struct_tag.as_ref())?;

                if struct_ty.ty_params.is_empty() && struct_tag.type_args.is_empty() {
                    Struct {
                        idx: struct_ty.idx,
                        ability: AbilityInfo::struct_(struct_ty.abilities),
                    }
                } else {
                    let mut ty_args = vec![];
                    for ty_arg in &struct_tag.type_args {
                        let ty_arg = self.create_ty_impl(ty_arg, resolver, count, depth + 1)?;
                        ty_args.push(ty_arg);
                    }
                    Type::verify_ty_arg_abilities(struct_ty.ty_param_constraints(), &ty_args)?;
                    Type::struct_instantiation(
                        struct_ty.idx,
                        ty_args,
                        AbilityInfo::generic_struct(
                            struct_ty.abilities,
                            struct_ty.phantom_ty_params_mask.clone(),
                        ),
                    )
                }
            },
            T::Function(fun) => {
                let FunctionTag {
                    args,
                    results,
                    abilities,
                } = fun.as_ref();
                if self.check_function_type_abilities && !abilities.is_valid_for_function_type() {
                    return Err(excess_function_type_abilities_error());
                }
                let mut to_list = |ts: &[FunctionParamOrReturnTag]| {
                    ts.iter()
                        .map(|t| {
                            // Note: for reference or mutable reference tags, we add 1 more level
                            // of depth, hence adding 2 to the counter.
                            Ok(match t {
                                FunctionParamOrReturnTag::Reference(t) => Type::reference(
                                    self.create_ty_impl(t, resolver, count, depth + 2)?,
                                ),
                                FunctionParamOrReturnTag::MutableReference(t) => {
                                    Type::mut_reference(self.create_ty_impl(
                                        t,
                                        resolver,
                                        count,
                                        depth + 2,
                                    )?)
                                },
                                FunctionParamOrReturnTag::Value(t) => {
                                    self.create_ty_impl(t, resolver, count, depth + 1)?
                                },
                            })
                        })
                        .collect::<PartialVMResult<Vec<_>>>()
                };
                Type::function(to_list(args)?, to_list(results)?, *abilities)
            },
        })
    }

    #[cfg(test)]
    fn num_nodes_in_subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<usize> {
        let mut count = 0;

        let check = |c: &mut u64, d: u64| self.check_before_count_increment(c, d);
        self.subst_impl(ty, ty_args, &mut count, 1, check)?;
        Ok(count as usize)
    }
}

/// Stores a map from type parameter indices to actual type instantiations. Allows to match a type
/// against some other, possibly generic type. Used for transaction argument construction with
/// constructor functions, e.g., when an empty vector can be treated as None via option::none()
/// function, which return type can be matched against the intended type of the argument.
#[derive(Default)]
pub struct TypeParamMap<'a> {
    map: BTreeMap<u16, &'a Type>,
}

impl<'a> TypeParamMap<'a> {
    /// Returns the type from parameter map if it exists, and [None] otherwise.
    pub fn get_ty_param(&self, idx: u16) -> Option<Type> {
        self.map.get(&idx).map(|ty| (*ty).clone())
    }

    pub fn arity(&self) -> usize {
        self.map.len()
    }

    /// Matches the actual type to the expected type, binding any type args to the necessary type
    /// as stored in the map. The expected type must be a concrete type (no [Type::TyParam]).
    ///
    /// Returns true if a successful match is made.
    pub fn match_ty(&mut self, ty: &Type, expected_ty: &'a Type) -> bool {
        match (ty, expected_ty) {
            // The important case, deduce the type params.
            (Type::TyParam(idx), _) => {
                // A type parameter can never be instantiated with a reference type. Reject such a
                // binding here so that the inferred instantiation stays well-formed (e.g., it can
                // be converted to a type tag) rather than producing an invariant violation in a
                // later stage.
                if matches!(expected_ty, Type::Reference(_) | Type::MutableReference(_)) {
                    return false;
                }
                use btree_map::Entry::*;
                match self.map.entry(*idx) {
                    Occupied(occupied_entry) => *occupied_entry.get() == expected_ty,
                    Vacant(vacant_entry) => {
                        vacant_entry.insert(expected_ty);
                        true
                    },
                }
            },
            // Recursive types we need to recurse the matching types.
            (Type::Reference(inner), Type::Reference(expected_inner))
            | (Type::MutableReference(inner), Type::MutableReference(expected_inner)) => {
                self.match_ty(inner, expected_inner)
            },
            (Type::Vector(inner), Type::Vector(expected_inner)) => {
                self.match_ty(inner, expected_inner)
            },
            // Function types, the expected abilities need to be equal to the provided ones,
            // and recursively argument and result types need to match.
            (Type::Function(node), Type::Function(exp_node))
                if node.abilities == exp_node.abilities
                    && node.args.len() == exp_node.args.len()
                    && node.results.len() == exp_node.results.len() =>
            {
                node.args
                    .iter()
                    .zip(exp_node.args.iter())
                    .all(|(t, e)| self.match_ty(t, e))
                    && node
                        .results
                        .iter()
                        .zip(exp_node.results.iter())
                        .all(|(t, e)| self.match_ty(t, e))
            },
            // Abilities should not contribute to the equality check as they just serve for caching
            // computations. For structs the both need to be the same struct.
            (
                Type::Struct { idx, .. },
                Type::Struct {
                    idx: expected_idx, ..
                },
            ) => *idx == *expected_idx,
            // For struct instantiations we need to additionally match all type arguments.
            (Type::StructInstantiation(node), Type::StructInstantiation(exp_node)) => {
                node.idx == exp_node.idx
                    && node.ty_args.len() == exp_node.ty_args.len()
                    && node
                        .ty_args
                        .iter()
                        .zip(exp_node.ty_args.iter())
                        .all(|types| self.match_ty(types.0, types.1))
            },
            // For primitive types we need to assure the types match.
            (Type::U8, Type::U8)
            | (Type::U16, Type::U16)
            | (Type::U32, Type::U32)
            | (Type::U64, Type::U64)
            | (Type::U128, Type::U128)
            | (Type::U256, Type::U256)
            | (Type::I8, Type::I8)
            | (Type::I16, Type::I16)
            | (Type::I32, Type::I32)
            | (Type::I64, Type::I64)
            | (Type::I128, Type::I128)
            | (Type::I256, Type::I256)
            | (Type::Bool, Type::Bool)
            | (Type::Address, Type::Address)
            | (Type::Signer, Type::Signer) => true,
            // Otherwise the types do not match, and we can't match return type to the expected type.
            // Note we don't use the _ pattern but spell out all cases, so that the compiler will
            // bark when a case is missed upon future updates to the types.
            (Type::U8, _)
            | (Type::U16, _)
            | (Type::U32, _)
            | (Type::U64, _)
            | (Type::U128, _)
            | (Type::U256, _)
            | (Type::I8, _)
            | (Type::I16, _)
            | (Type::I32, _)
            | (Type::I64, _)
            | (Type::I128, _)
            | (Type::I256, _)
            | (Type::Bool, _)
            | (Type::Address, _)
            | (Type::Signer, _)
            | (Type::Struct { .. }, _)
            | (Type::StructInstantiation { .. }, _)
            | (Type::Function { .. }, _)
            | (Type::Vector(_), _)
            | (Type::MutableReference(_), _)
            | (Type::Reference(_), _) => false,
        }
    }

    /// Matches sequences of types using `match_ty`.
    pub fn match_tys(
        &mut self,
        actuals: impl ExactSizeIterator<Item = &'a Type>,
        expected: impl ExactSizeIterator<Item = &'a Type>,
    ) -> bool {
        if actuals.len() != expected.len() {
            return false;
        }
        for (actual, formal) in actuals.zip(expected) {
            if !self.match_ty(actual, formal) {
                return false;
            }
        }
        true
    }

    /// Produces a type parameter instantiation from the type params in the map,
    /// verifying abilities against type parameter declarations.
    pub fn verify_and_extract_type_args(
        &self,
        param_decls: &[AbilitySet],
    ) -> PartialVMResult<Vec<Type>> {
        let args = (0..param_decls.len())
            .filter_map(|idx| self.map.get(&(idx as u16)).cloned().cloned())
            .collect_vec();
        Type::verify_ty_arg_abilities(param_decls, &args)?;
        Ok(args)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use claims::{assert_err, assert_matches, assert_ok};
    use move_binary_format::file_format::StructHandleIndex;

    #[test]
    fn unknown_function_ability_bits_satisfy_known_ability_constraints() {
        let expected = AbilitySet::from_u8(0x07).expect("0x07 is a valid ability set");
        let malformed =
            bcs::from_bytes::<AbilitySet>(&[0x17]).expect("Deserialize accepts any ability byte");
        let function_type = Type::function(vec![], vec![], malformed);

        assert_ne!(expected, malformed);
        assert!(expected.is_subset(malformed));
        assert!(Type::verify_ty_arg_abilities([&expected], &[function_type]).is_ok());
    }

    fn function_tag_with_abilities(abilities_byte: u8) -> TypeTag {
        TypeTag::Function(Box::new(FunctionTag {
            args: vec![],
            results: vec![],
            abilities: bcs::from_bytes::<AbilitySet>(&[abilities_byte])
                .expect("Deserialize accepts any ability byte"),
        }))
    }

    #[test]
    fn create_ty_rejects_function_types_with_excess_abilities() {
        let ty_builder = TypeBuilder::with_limits(11, 5, true, true, true);
        let no_op = |_: &StructTag| unreachable!("Should not be called");

        // 0x08 and 0x0f include `key`; 0x10, 0x17, and 0xff include undefined bits.
        for abilities_byte in [0x0F, 0x08, 0x17, 0x10, 0xFF] {
            let err = assert_err!(
                ty_builder.create_ty(&function_tag_with_abilities(abilities_byte), no_op)
            );
            assert_eq!(
                err.major_status(),
                StatusCode::CONSTRAINT_NOT_SATISFIED,
                "expected ability byte {:#x} to be rejected",
                abilities_byte
            );
        }

        // Representative subsets of `PUBLIC_FUNCTIONS` remain valid.
        for abilities_byte in [0x00, 0x01, 0x02, 0x03, 0x04, 0x07] {
            assert_ok!(ty_builder.create_ty(&function_tag_with_abilities(abilities_byte), no_op));
        }
    }

    /// Rejects excess abilities in nested function types as well as top-level tags.
    #[test]
    fn create_ty_rejects_nested_function_types_with_excess_abilities() {
        let ty_builder = TypeBuilder::with_limits(20, 10, true, true, true);
        let no_op = |_: &StructTag| unreachable!("Should not be called");

        let bad = function_tag_with_abilities(0x0F);
        let nested = [
            TypeTag::Vector(Box::new(bad.clone())),
            TypeTag::Vector(Box::new(TypeTag::Vector(Box::new(bad.clone())))),
            TypeTag::Function(Box::new(FunctionTag {
                args: vec![FunctionParamOrReturnTag::Value(bad.clone())],
                results: vec![],
                abilities: AbilitySet::PUBLIC_FUNCTIONS,
            })),
            TypeTag::Function(Box::new(FunctionTag {
                args: vec![],
                results: vec![FunctionParamOrReturnTag::Reference(bad)],
                abilities: AbilitySet::PUBLIC_FUNCTIONS,
            })),
        ];

        for ty_tag in nested {
            let err = assert_err!(ty_builder.create_ty(&ty_tag, no_op));
            assert_eq!(
                err.major_status(),
                StatusCode::CONSTRAINT_NOT_SATISFIED,
                "nested function type must be rejected in {:?}",
                ty_tag
            );
        }
    }

    /// Retains raw ability bytes when function-ability validation is disabled.
    #[test]
    fn create_ty_preserves_excess_abilities_when_the_check_is_disabled() {
        let ty_builder = TypeBuilder::with_limits(11, 5, true, true, false);
        let no_op = |_: &StructTag| unreachable!("Should not be called");

        for abilities_byte in [0x0F, 0x17] {
            let ty = assert_ok!(
                ty_builder.create_ty(&function_tag_with_abilities(abilities_byte), no_op)
            );
            assert_eq!(assert_ok!(ty.abilities()).into_u8(), abilities_byte);
        }
    }

    /// Subset-based type-argument checks accept a function type that claims `key`. Because `0x0f`
    /// is a valid `AbilitySet` encoding, encoding validation alone cannot prevent this.
    #[test]
    fn function_type_claiming_key_satisfies_a_key_constraint() {
        let claims_key = AbilitySet::from_u8(0x0F).expect("0x0F is a well-formed ability set");
        assert!(claims_key.has_key());
        assert!(!claims_key.is_subset(AbilitySet::PUBLIC_FUNCTIONS));

        let function_type = Type::function(vec![], vec![], claims_key);
        assert_eq!(function_type.abilities().unwrap(), claims_key);

        let key_constraint = AbilitySet::singleton(Ability::Key);
        assert_ok!(Type::verify_ty_arg_abilities(
            [&key_constraint],
            std::slice::from_ref(&function_type)
        ));
        assert_ok!(function_type.paranoid_check_abilities(key_constraint));

        let honest_function_type = Type::function(vec![], vec![], AbilitySet::PUBLIC_FUNCTIONS);
        assert_err!(Type::verify_ty_arg_abilities([&key_constraint], &[
            honest_function_type
        ]));
    }

    #[test]
    fn test_struct_identifier_identity_is_interner_history_independent() {
        use move_core_types::account_address::AccountAddress;
        use std::{
            cmp::Ordering,
            hash::{DefaultHasher, Hash, Hasher},
        };

        fn hash_of(identifier: &StructIdentifier) -> u64 {
            let mut hasher = DefaultHasher::new();
            identifier.hash(&mut hasher);
            hasher.finish()
        }

        let alpha = ModuleId::new(AccountAddress::ONE, Identifier::new("alpha").unwrap());
        let beta = ModuleId::new(AccountAddress::TWO, Identifier::new("beta").unwrap());
        let name = Identifier::new("S").unwrap();

        // Warm two pools in opposite orders, so the same module gets different interned ids.
        let cold_pool = InternedModuleIdPool::new();
        let cold_beta = StructIdentifier::new(&cold_pool, beta.clone(), name.clone());
        let cold_alpha = StructIdentifier::new(&cold_pool, alpha.clone(), name.clone());

        let warm_pool = InternedModuleIdPool::new();
        let warm_alpha = StructIdentifier::new(&warm_pool, alpha, name.clone());
        let warm_beta = StructIdentifier::new(&warm_pool, beta, name);

        assert_ne!(
            cold_alpha.interned_module_id(),
            warm_alpha.interned_module_id()
        );

        // Identity, hashing, and ordering must not depend on interner history.
        assert_eq!(cold_alpha, warm_alpha);
        assert_eq!(cold_beta, warm_beta);
        assert_eq!(hash_of(&cold_alpha), hash_of(&warm_alpha));
        assert_eq!(cold_alpha.cmp(&warm_alpha), Ordering::Equal);
        assert_eq!(cold_alpha.partial_cmp(&warm_alpha), Some(Ordering::Equal));
        assert_eq!(cold_alpha.cmp(&cold_beta), warm_alpha.cmp(&warm_beta));
    }

    fn wide_fn_ty(abilities: AbilitySet) -> Type {
        let args = (0..64u32)
            .map(|i| {
                Type::struct_instantiation(
                    StructNameIndex::new(i),
                    vec![Type::U64, Type::Bool],
                    AbilityInfo::struct_(AbilitySet::EMPTY),
                )
            })
            .collect();
        Type::function(args, vec![Type::U8], abilities)
    }

    #[test]
    fn function_assignability_short_circuits_on_node_identity_and_interned_ids() {
        use crate::ty_interner::InternedTypePool;

        let copy_drop =
            AbilitySet::singleton(Ability::Copy).union(AbilitySet::singleton(Ability::Drop));
        let f = wide_fn_ty(copy_drop);

        // Same node on both sides (a value assigned to a local of its own declared type): the
        // check must pass through pointer identity without touching the argument list.
        assert_ok!(f.paranoid_check_assignable(&f));

        // Separately built, structurally equal: passes via the structural fallback. Intern
        // both and the equal ids take over; either way the verdict is the same.
        let g = wide_fn_ty(copy_drop);
        assert_ok!(g.paranoid_check_assignable(&f));
        let pool = InternedTypePool::new();
        pool.intern_ty(&f);
        pool.intern_ty(&g);
        assert_ok!(g.paranoid_check_assignable(&f));

        // Abilities are inclusion, not equality: a copy+drop value is assignable to a
        // drop-only slot (fewer abilities expected), but not the reverse.
        let drop_only = wide_fn_ty(AbilitySet::singleton(Ability::Drop));
        assert_ok!(f.paranoid_check_assignable(&drop_only));
        assert_err!(drop_only.paranoid_check_assignable(&f));

        // A different signature is never assignable, interned or not.
        let mut other_args = vec![Type::U64];
        other_args.extend((0..63u32).map(|i| {
            Type::struct_instantiation(
                StructNameIndex::new(i),
                vec![Type::U64, Type::Bool],
                AbilityInfo::struct_(AbilitySet::EMPTY),
            )
        }));
        let h = Type::function(other_args, vec![Type::U8], copy_drop);
        pool.intern_ty(&h);
        assert_err!(h.paranoid_check_assignable(&f));
    }

    #[test]
    fn substitution_shares_the_type_argument_node() {
        let ty_builder = TypeBuilder::with_limits(128, 20, true, true, true);
        let arg = Type::struct_instantiation(
            StructNameIndex::new(3),
            vec![Type::vector(Type::U8), Type::Bool],
            AbilityInfo::struct_(AbilitySet::EMPTY),
        );
        let Type::StructInstantiation(arg_node) = &arg else {
            panic!("expected a struct instantiation")
        };

        let x = assert_ok!(
            ty_builder.create_ty_with_subst(&Type::TyParam(0), std::slice::from_ref(&arg))
        );
        let y = assert_ok!(
            ty_builder.create_ty_with_subst(&Type::TyParam(0), std::slice::from_ref(&arg))
        );
        for ty in [&x, &y] {
            let Type::StructInstantiation(node) = ty else {
                panic!("expected a struct instantiation")
            };
            assert!(TriompheArc::ptr_eq(node, arg_node));
        }

        let v = assert_ok!(ty_builder
            .create_ty_with_subst(&Type::vector(Type::TyParam(0)), std::slice::from_ref(&arg)));
        let Type::Vector(elem) = &v else {
            panic!("expected a vector")
        };
        let Type::StructInstantiation(node) = elem.ty() else {
            panic!("expected a struct instantiation")
        };
        assert!(TriompheArc::ptr_eq(node, arg_node));
    }

    fn struct_instantiation_ty_for_test(ty_args: Vec<Type>) -> Type {
        Type::struct_instantiation(
            StructNameIndex::new(0),
            ty_args,
            AbilityInfo::struct_(AbilitySet::EMPTY),
        )
    }

    fn struct_ty_for_test() -> Type {
        Type::Struct {
            idx: StructNameIndex::new(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        }
    }

    fn nested_vec_for_test(ty_depth: u64) -> (Type, SignatureToken, TypeTag) {
        use SignatureToken as S;
        use Type::*;
        use TypeTag as T;

        let mut ty = U8;
        let mut tok = S::U8;
        let mut tag = T::U8;

        let mut depth = 1;
        while depth < ty_depth {
            ty = Type::vector(ty.clone());
            tok = S::Vector(Box::new(tok.clone()));
            tag = T::Vector(Box::new(tag.clone()));
            depth += 1;
        }
        (ty, tok, tag)
    }

    #[test]
    fn test_num_nodes_in_type() {
        use Type::*;

        let cases = [
            (U8, 1),
            (Type::vector(U8), 2),
            (Type::vector(Type::vector(U8)), 3),
            (Type::reference(Bool), 2),
            (TyParam(0), 1),
            (struct_ty_for_test(), 1),
            (struct_instantiation_ty_for_test(vec![U8, U8]), 3),
            (
                struct_instantiation_ty_for_test(vec![
                    U8,
                    struct_instantiation_ty_for_test(vec![Bool, Bool, Bool]),
                    U8,
                ]),
                7,
            ),
        ];

        for (ty, expected) in cases {
            assert_eq!(ty.num_nodes(), expected);
        }
    }

    #[test]
    fn test_num_nodes_in_subst() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5, true, true, true);
        let cases: Vec<(Type, Vec<Type>, usize, usize)> = vec![
            (TyParam(0), vec![Bool], 1, 1),
            (TyParam(0), vec![Type::vector(Bool)], 2, 2),
            (Bool, vec![], 1, 1),
            (
                struct_instantiation_ty_for_test(vec![TyParam(0), TyParam(0)]),
                vec![Type::vector(Bool)],
                5,
                3,
            ),
            (
                struct_instantiation_ty_for_test(vec![TyParam(0), TyParam(1)]),
                vec![Type::vector(Bool), Type::vector(Type::vector(Bool))],
                6,
                4,
            ),
        ];

        for (ty, ty_args, expected_num_nodes, expected_depth) in cases {
            let num_nodes = assert_ok!(ty_builder.num_nodes_in_subst(&ty, &ty_args));
            assert_eq!(num_nodes, expected_num_nodes);
            let (count, depth) = ty.num_nodes_in_subst(&ty_args).unwrap();
            assert_eq!(count, expected_num_nodes);
            assert_eq!(depth, expected_depth);
        }
    }

    #[test]
    fn test_substitution_large_depth() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5, true, true, true);

        let ty = Type::vector(Type::vector(TyParam(0)));
        let ty_arg = Type::vector(Type::vector(Bool));
        assert_ok!(ty_builder.create_ty_with_subst(&ty, std::slice::from_ref(&ty_arg)));

        let ty_arg = Type::vector(ty_arg);
        let err = assert_err!(ty_builder.create_ty_with_subst(&ty, &[ty_arg]));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);
    }

    #[test]
    fn test_substitution_large_count() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(11, 5, true, true, true);

        let ty_params: Vec<Type> = (0..5).map(TyParam).collect();
        let ty = struct_instantiation_ty_for_test(ty_params);

        // Each type argument contributes 2 nodes, so in total the count is 11.
        let ty_args: Vec<Type> = (0..5).map(|_| Type::vector(Bool)).collect();
        let num_nodes = assert_ok!(ty_builder.num_nodes_in_subst(&ty, &ty_args));
        assert_eq!(num_nodes, 11);

        let ty_args: Vec<Type> = (0..5)
            .map(|i| {
                if i == 4 {
                    // 3 nodes, to increase the total count to 12.
                    struct_instantiation_ty_for_test(vec![U64, struct_ty_for_test()])
                } else {
                    Type::vector(Bool)
                }
            })
            .collect();
        let err = assert_err!(ty_builder.create_ty_with_subst(&ty, &ty_args));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_primitive_tys() {
        use Type::*;

        // Limits are irrelevant here.
        let ty_builder = TypeBuilder::with_limits(1, 1, true, true, true);

        assert_eq!(ty_builder.create_u8_ty(), U8);
        assert_eq!(ty_builder.create_u16_ty(), U16);
        assert_eq!(ty_builder.create_u32_ty(), U32);
        assert_eq!(ty_builder.create_u64_ty(), U64);
        assert_eq!(ty_builder.create_u128_ty(), U128);
        assert_eq!(ty_builder.create_u256_ty(), U256);
        assert_eq!(ty_builder.create_bool_ty(), Bool);
    }

    #[test]
    fn test_create_struct_ty() {
        let idx = StructNameIndex::new(0);
        let ability_info = AbilityInfo::struct_(AbilitySet::EMPTY);

        // Limits are not relevant here.
        let struct_ty = TypeBuilder::with_limits(1, 1, true, true, true)
            .create_struct_ty(idx, ability_info.clone());
        assert_matches!(struct_ty, Type::Struct { .. });
    }

    #[test]
    fn test_create_struct_instantiation_ty() {
        use Type::*;

        let struct_ty = StructType::for_test();
        let ty_params = [TyParam(0), Bool, TyParam(1)];

        // Should succeed, type size limit is 5, and we have 5 nodes.
        let ty_builder = TypeBuilder::with_limits(5, 100, true, true, true);
        let ty_args = [Bool, Type::vector(Bool)];
        assert_ok!(ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args));

        // Should fail, we have size of 6 now.
        let ty_args = [Type::vector(Bool), Type::vector(Bool)];
        let err = assert_err!(
            ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args)
        );
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);

        // Should succeed, type depth limit is 4, and we have 4 nodes (3 in type parameter + struct).
        let nested_vec = Type::vector(Type::vector(Bool));
        let ty_args = vec![Bool, nested_vec.clone()];
        let ty_builder = TypeBuilder::with_limits(100, 4, true, true, true);
        assert_ok!(ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args));

        // Should fail, we have depth of 5 now.
        let ty_params = vec![Bool, Type::vector(nested_vec)];
        let err = assert_err!(
            ty_builder.create_struct_instantiation_ty(&struct_ty, &ty_params, &ty_args)
        );
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);
    }

    #[test]
    fn test_create_vec_ty() {
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth, true, true, true);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < max_ty_depth {
            ty = assert_ok!(ty_builder.create_vec_ty(&ty));
            assert_matches!(ty, Type::Vector(_));
            depth += 1;
        }
        assert_eq!(depth, max_ty_depth);

        // Type creation fails on exceeding the depth.
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        // The checks are always ordered: first number of nodes, then depth. Using
        // a type builder with smaller than depth size limit must return a different
        // error code.
        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100, true, true, true);
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test]
    fn test_create_ref_ty() {
        let limit = 5;

        let ty_builder = TypeBuilder::with_limits(100, limit, true, true, true);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < limit {
            ty = assert_ok!(ty_builder.create_vec_ty(&ty));
            depth += 1;
        }

        let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, false));
        assert_matches!(ref_ty, Type::Reference(_));

        let ty_builder = TypeBuilder::with_limits(limit, 100, true, true, true);

        let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, false));
        assert_matches!(ref_ty, Type::Reference(_));
    }

    #[test]
    fn test_create_mut_ref_ty() {
        let limit = 5;

        let ty_builder = TypeBuilder::with_limits(100, limit, true, true, true);

        let mut depth = 1;
        let mut ty = Type::Bool;
        while depth < limit {
            ty = assert_ok!(ty_builder.create_vec_ty(&ty));
            depth += 1;
        }

        let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, true));
        assert_matches!(ref_ty, Type::MutableReference(_));

        let ty_builder = TypeBuilder::with_limits(limit, 100, true, true, true);

        let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, true));
        assert_matches!(ref_ty, Type::MutableReference(_));
    }

    #[test]
    fn test_create_constant_ty() {
        use SignatureToken as S;
        use Type::*;

        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(100, max_ty_depth, true, true, true);

        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U8)), U8);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U16)), U16);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U32)), U32);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U64)), U64);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U128)), U128);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::U256)), U256);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I8)), I8);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I16)), I16);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I32)), I32);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I64)), I64);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I128)), I128);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::I256)), I256);
        assert_eq!(assert_ok!(ty_builder.create_constant_ty(&S::Bool)), Bool);
        assert_eq!(
            assert_ok!(ty_builder.create_constant_ty(&S::Address)),
            Address
        );

        // Vectors are special, because we limit their depth (and size).
        // Here, we test the boundary cases.

        for depth in [max_ty_depth - 1, max_ty_depth] {
            let (expected_ty, vec_tok, _) = nested_vec_for_test(depth);
            let ty = assert_ok!(ty_builder.create_constant_ty(&vec_tok));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, vec_tok, _) = nested_vec_for_test(max_ty_depth + 1);
        let err = assert_err!(ty_builder.create_constant_ty(&vec_tok));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100, true, true, true);

        for size in [max_ty_size - 1, max_ty_size] {
            let (expected_ty, vec_tok, _) = nested_vec_for_test(size);
            let ty = assert_ok!(ty_builder.create_constant_ty(&vec_tok));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, vec_tok, _) = nested_vec_for_test(max_ty_size + 1);
        let err = assert_err!(ty_builder.create_constant_ty(&vec_tok));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);

        // The following tokens cannot be constants:

        let struct_tok = S::Struct(StructHandleIndex::new(0));
        assert_err!(ty_builder.create_constant_ty(&struct_tok));

        let struct_instantiation_tok = S::StructInstantiation(StructHandleIndex::new(0), vec![]);
        assert_err!(ty_builder.create_constant_ty(&struct_instantiation_tok));

        assert_err!(ty_builder.create_constant_ty(&S::Signer));

        let ref_tok = S::Reference(Box::new(S::U8));
        assert_err!(ty_builder.create_constant_ty(&ref_tok));

        let mut_ref_tok = S::Reference(Box::new(S::U8));
        assert_err!(ty_builder.create_constant_ty(&mut_ref_tok));

        let ty_param_tok = S::TypeParameter(0);
        assert_err!(ty_builder.create_constant_ty(&ty_param_tok));
    }

    #[test]
    fn test_create_ty() {
        use Type::*;
        use TypeTag as T;

        let max_ty_size = 11;
        let max_ty_depth = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, max_ty_depth, true, true, true);

        let no_op = |_: &StructTag| unreachable!("Should not be called");

        // Primitive types.

        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U8, no_op)), U8);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U16, no_op)), U16);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U32, no_op)), U32);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U64, no_op)), U64);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U128, no_op)), U128);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::U256, no_op)), U256);
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::Bool, no_op)), Bool);
        assert_eq!(
            assert_ok!(ty_builder.create_ty(&T::Address, no_op)),
            Address
        );
        assert_eq!(assert_ok!(ty_builder.create_ty(&T::Signer, no_op)), Signer);

        // Vectors.

        for depth in [max_ty_depth - 1, max_ty_depth] {
            let (expected_ty, _, vec_tag) = nested_vec_for_test(depth);
            let ty = assert_ok!(ty_builder.create_ty(&vec_tag, no_op));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, _, vec_tag) = nested_vec_for_test(max_ty_depth + 1);
        let err = assert_err!(ty_builder.create_ty(&vec_tag, no_op));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_TYPE_DEPTH_REACHED);

        let max_ty_size = 5;
        let ty_builder = TypeBuilder::with_limits(max_ty_size, 100, true, true, true);

        for size in [max_ty_size - 1, max_ty_size] {
            let (expected_ty, _, vec_tag) = nested_vec_for_test(size);
            let ty = assert_ok!(ty_builder.create_ty(&vec_tag, no_op));
            assert_eq!(&ty, &expected_ty);
        }

        let (_, _, vec_tag) = nested_vec_for_test(max_ty_size + 1);
        let err = assert_err!(ty_builder.create_ty(&vec_tag, no_op));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    fn struct_ty_for_test_with(abilities: AbilitySet, phantom_mask: &[bool]) -> StructType {
        let mut ty = StructType::for_test();
        ty.abilities = abilities;
        ty.phantom_ty_params_mask = phantom_mask.iter().copied().collect();
        ty.ty_params = phantom_mask
            .iter()
            .map(|is_phantom| StructTypeParameter {
                constraints: AbilitySet::EMPTY,
                is_phantom: *is_phantom,
            })
            .collect();
        ty
    }

    /// Builds a diverse set of (fully instantiated) types for property-style tests.
    ///
    /// Keep the entries with references nested inside a type: newer gas versions count reference
    /// nodes as free, so the cached node count may over-count and is only ever used to accept,
    /// never to reject. Those entries are what exercise that fallback.
    fn sample_tys() -> Vec<Type> {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(1000, 1000, true, true, true);
        let p = struct_ty_for_test_with(
            AbilitySet::singleton(Ability::Copy) | AbilitySet::singleton(Ability::Drop),
            &[true],
        );
        let s = struct_ty_for_test_with(
            AbilitySet::singleton(Ability::Copy) | AbilitySet::singleton(Ability::Drop),
            &[true; 32],
        );
        let n = struct_ty_for_test_with(AbilitySet::ALL, &[false, false]);
        let p_params = [TyParam(0)];
        let s_params: Vec<Type> = (0..32).map(TyParam).collect();
        let n_params = [TyParam(0), TyParam(1)];

        // P<P<P<u8>>>
        let mut t = U8;
        for _ in 0..3 {
            t = assert_ok!(ty_builder.create_struct_instantiation_ty(&p, &p_params, &[t]));
        }
        // S<T x 30, u8, u8>: the 123-node type from the reports.
        let mut s_args = vec![t.clone(); 30];
        s_args.push(U8);
        s_args.push(U8);
        let wide = assert_ok!(ty_builder.create_struct_instantiation_ty(&s, &s_params, &s_args));
        assert_eq!(wide.num_nodes(), 123);

        let (nested_vec, _, _) = nested_vec_for_test(6);
        let n_signer =
            assert_ok!(ty_builder.create_struct_instantiation_ty(&n, &n_params, &[Signer, U8]));
        let n_nested = assert_ok!(ty_builder
            .create_struct_instantiation_ty(&n, &n_params, &[wide.clone(), n_signer.clone()]));
        let func = Type::function(
            vec![
                Type::reference(wide.clone()),
                Type::mut_reference(U8),
                nested_vec.clone(),
            ],
            vec![n_signer.clone()],
            AbilitySet::EMPTY,
        );
        vec![
            U8,
            Signer,
            struct_ty_for_test(),
            nested_vec.clone(),
            t,
            wide.clone(),
            n_signer,
            n_nested,
            Type::vector(wide.clone()),
            Type::reference(nested_vec),
            Type::mut_reference(wide),
            func,
        ]
    }

    #[test]
    fn test_type_info_is_consistent_with_structure() {
        use Type::*;

        for ty in sample_tys() {
            assert!(ty.has_consistent_type_info(), "{ty}");
            assert!(!ty.type_info().has_ty_params(), "{ty}");
        }

        // Types with type parameters: counts are still exact, abilities are unknown.
        let ty_builder = TypeBuilder::with_limits(1000, 1000, true, true, true);
        let n = struct_ty_for_test_with(AbilitySet::ALL, &[false, true]);
        let template = Type::struct_instantiation(
            n.idx,
            vec![TyParam(0), Type::vector(TyParam(1))],
            AbilityInfo::generic_struct(n.abilities, n.phantom_ty_params_mask.clone()),
        );
        assert!(template.has_consistent_type_info());
        assert!(template.type_info().has_ty_params());
        assert_eq!(template.num_nodes_with_max_depth(), (4, 3));
        assert!(template.type_info().cached_abilities().is_none());
        assert_err!(template.abilities());

        // Substituting produces a type with known abilities.
        let instantiated = assert_ok!(ty_builder.create_ty_with_subst(&template, &[Signer, U8]));
        assert!(instantiated.has_consistent_type_info());
        assert!(!instantiated.type_info().has_ty_params());
        assert_eq!(
            instantiated.type_info().cached_abilities(),
            Some(AbilitySet::singleton(Ability::Drop))
        );
        assert_eq!(
            assert_ok!(instantiated.abilities()),
            AbilitySet::singleton(Ability::Drop)
        );
    }

    #[test]
    fn test_type_info_does_not_affect_equality_and_hashing() {
        use std::hash::{Hash, Hasher};

        // A type built via the constructor and the "same" type with (deliberately) different
        // cached info must compare and hash equal.
        let s = struct_ty_for_test_with(AbilitySet::ALL, &[false, true]);
        let a = Type::struct_instantiation(
            s.idx,
            vec![Type::Signer, Type::U8],
            AbilityInfo::generic_struct(s.abilities, s.phantom_ty_params_mask.clone()),
        );
        let b = Type::struct_instantiation(
            s.idx,
            vec![Type::Signer, Type::U8],
            AbilityInfo::struct_(AbilitySet::EMPTY),
        );
        assert_eq!(a, b);
        let hash = |ty: &Type| {
            let mut h = std::collections::hash_map::DefaultHasher::new();
            ty.hash(&mut h);
            h.finish()
        };
        assert_eq!(hash(&a), hash(&b));
    }

    #[test]
    fn test_create_ref_and_vec_ty_share_inner_ty() {
        use Type::*;

        let ty_builder = TypeBuilder::with_limits(128, 20, true, true, true);
        for ty in sample_tys() {
            let node_arc = match &ty {
                StructInstantiation(node) => node.clone(),
                _ => continue,
            };
            // Creating a reference or a vector must not rebuild the inner type: the node
            // must be the very same allocation.
            let same_args = |inner: &Type| match inner {
                StructInstantiation(node) => TriompheArc::ptr_eq(node, &node_arc),
                _ => false,
            };
            for is_mut in [false, true] {
                let ref_ty = assert_ok!(ty_builder.create_ref_ty(&ty, is_mut));
                match &ref_ty {
                    Reference(inner_ty) | MutableReference(inner_ty) => {
                        assert_eq!(inner_ty.ty(), &ty);
                        assert!(same_args(inner_ty));
                    },
                    _ => panic!("expected reference"),
                }
                assert!(ref_ty.has_consistent_type_info());
            }
            let vec_ty = assert_ok!(ty_builder.create_vec_ty(&ty));
            match &vec_ty {
                Vector(elem_ty) => assert!(same_args(elem_ty)),
                _ => panic!("expected vector"),
            }
            assert!(vec_ty.has_consistent_type_info());
        }
    }

    #[test]
    fn test_subst_limits_match_rebuild() {
        let tys = sample_tys();
        for v2 in [false, true] {
            for max_size in 1..=130u64 {
                for max_depth in 1..=12u64 {
                    let ty_builder = TypeBuilder::with_limits(max_size, max_depth, v2, true, true);
                    let check = |c: &mut u64, d: u64| ty_builder.check_before_count_increment(c, d);
                    for ty in &tys {
                        for generic in [Type::TyParam(0), Type::vector(Type::TyParam(0))] {
                            let concrete = match &generic {
                                Type::TyParam(_) => ty.clone(),
                                _ => Type::vector(ty.clone()),
                            };
                            let expected = ty_builder
                                .clone_impl(&concrete, &mut 0, 1, check)
                                .map_err(|e| e.major_status());
                            let actual = ty_builder
                                .create_ty_with_subst(&generic, std::slice::from_ref(ty))
                                .map_err(|e| e.major_status());
                            assert_eq!(
                                actual, expected,
                                "v2={v2} size={max_size} depth={max_depth} ty={concrete}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_create_ref_and_vec_ty_limits_match_rebuild() {
        // For every combination of counting mode, size limit, and depth limit, creating a
        // reference / vector over an existing type must produce exactly the same result (and the
        // same error code) as rebuilding the type node by node via `clone_impl` does.
        let tys = sample_tys();
        for v2 in [false, true] {
            for max_size in 1..=130u64 {
                for max_depth in 1..=12u64 {
                    let ty_builder = TypeBuilder::with_limits(max_size, max_depth, v2, true, true);
                    for ty in &tys {
                        for is_mut in [false, true] {
                            let mut count = 1 - ty_builder.ref_ty_count_decrement;
                            let check =
                                |c: &mut u64, d: u64| ty_builder.check_before_count_increment(c, d);
                            let expected = ty_builder
                                .clone_impl(
                                    ty,
                                    &mut count,
                                    1 + ty_builder.ref_ty_depth_increment,
                                    check,
                                )
                                .map(|inner_ty| {
                                    if is_mut {
                                        Type::mut_reference(inner_ty)
                                    } else {
                                        Type::reference(inner_ty)
                                    }
                                })
                                .map_err(|e| e.major_status());
                            let actual = ty_builder
                                .create_ref_ty(ty, is_mut)
                                .map_err(|e| e.major_status());
                            assert_eq!(
                                actual, expected,
                                "ref v2={v2} size={max_size} depth={max_depth} ty={ty}"
                            );
                        }

                        let mut count = 1;
                        let check =
                            |c: &mut u64, d: u64| ty_builder.check_before_count_increment(c, d);
                        let expected = ty_builder
                            .clone_impl(ty, &mut count, 2, check)
                            .map(Type::vector)
                            .map_err(|e| e.major_status());
                        let actual = ty_builder.create_vec_ty(ty).map_err(|e| e.major_status());
                        assert_eq!(
                            actual, expected,
                            "vec v2={v2} size={max_size} depth={max_depth} ty={ty}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_create_ref_ty_rejects_ty_params() {
        let ty_builder = TypeBuilder::with_limits(128, 20, true, true, true);
        let ty = Type::vector(Type::TyParam(0));
        let err = assert_err!(ty_builder.create_ref_ty(&ty, false));
        assert_eq!(
            err.major_status(),
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        );
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(
            err.major_status(),
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR
        );
    }

    #[test]
    fn test_cached_abilities_match_recursive_computation() {
        for ty in sample_tys() {
            assert_eq!(
                assert_ok!(ty.abilities()),
                assert_ok!(ty.abilities_uncached()),
                "{ty}"
            );
        }
    }

    #[test]
    fn test_type_info_saturates_safely() {
        // Build a type whose node count overflows u16 and make sure counts fall back to exact
        // traversal rather than reporting a wrong (saturated) value.
        let mut ty = Type::U8;
        for _ in 0..16 {
            ty = Type::struct_instantiation(
                StructNameIndex::new(0),
                vec![ty.clone(), ty],
                AbilityInfo::struct_(AbilitySet::EMPTY),
            );
        }
        assert!(ty.type_info().is_saturated());
        let (num_nodes, depth) = ty.num_nodes_with_max_depth();
        assert_eq!(num_nodes, (1 << 17) - 1);
        assert_eq!(depth, 17);
        assert_eq!(ty.num_nodes(), (1 << 17) - 1);
        assert!(ty.has_consistent_type_info());
        // Limit checks must still reject it with the right error.
        let ty_builder = TypeBuilder::with_limits(128, 20, true, true, true);
        let err = assert_err!(ty_builder.create_vec_ty(&ty));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }
}
