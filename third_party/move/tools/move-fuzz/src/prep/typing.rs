// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The type language used by the driver-script synthesizer.
//!
//! # Why not an existing Move type representation?
//!
//! The synthesizer needs a representation with four properties at the same time:
//!
//! 1. *absolute* datatype identity (address + module + name), because the whole
//!    point of the flow graph is to chain a function in one module into a
//!    function in another;
//! 2. *type parameters*, because it reasons about uninstantiated declarations
//!    and about the generics of the script it is about to emit;
//! 3. *references* (`&T` / `&mut T`), because it models borrow chains;
//! 4. *memoized abilities*, because ability checks dominate the search and are
//!    not recoverable from a type in isolation.
//!
//! None of the obvious candidates has all four:
//!
//! - `move_binary_format::file_format::SignatureToken` has 2 and 3 but not 1:
//!   `Struct(StructHandleIndex)` indexes one specific `CompiledModule`'s handle
//!   table and is meaningless outside it. For 4 it carries abilities only on
//!   `Function(.., AbilitySet)`; there is nowhere to memoize the abilities of a
//!   struct instantiation or of a type parameter.
//! - `move_core_types::language_storage::TypeTag` has 1 but neither 2 nor 3: it
//!   is fully instantiated and cannot express a reference. It *is* used in this
//!   crate - imported as `VmTypeTag` - for the type arguments that actually
//!   reach the VM.
//! - `move_model::ty::Type` has 2 and 3 but not 1 or 4: it names structs by
//!   `(ModuleId, StructId)` indices into a `GlobalEnv`, carries specification-
//!   and inference-only variants (`TypeDomain`, `ResourceDomain`, `Var`,
//!   `Error`), and is not `serde`-serializable. The synthesis pipeline here
//!   builds its types from `SignatureToken`s in compiled units and has no
//!   `GlobalEnv` in hand - one is only ever constructed on the unit-test path,
//!   in `crate::package::unit_test` - and these types must round-trip through
//!   the on-disk corpus.
//!
//! # The ladder
//!
//! Four representations here form a deliberate ladder. They are not four
//! spellings of one thing:
//!
//! - [`TypeTag`] - a type as *written in a declaration*. `Param(usize)` refers
//!   to a generic slot of the declaring function or struct. No abilities.
//! - [`TypeBase`] - the same shape after *instantiation*: abilities of each
//!   datatype and parameter are memoized, and `Param` now refers to a generic
//!   slot of the *generated script*.
//! - [`SimpleType`] / [`ComplexType`] - a partition of [`TypeBase`] by whether a
//!   value can be produced directly (`Simple`) or only by calling something that
//!   returns it (`Complex`). Splitting the enum, rather than testing a
//!   predicate, is what keeps the matches in `prep::canvas` and `prep::graph`
//!   total.
//! - `crate::prep::canvas::BasicInput` - what actually becomes a parameter of
//!   the emitted script, and the exact domain of the mutator.
//!
//! [`TypeRef`], [`TypeItem`] and `crate::prep::graph::DatatypeItem` are the
//! first three of these under a reference mode; all three are aliases of the one
//! generic [`crate::common::Refty`].
//!
//! Note that this module's [`TypeTag`] is *not*
//! `move_core_types::language_storage::TypeTag`; wherever both are in scope this
//! crate imports the latter as `VmTypeTag`.

use crate::{common::Refty, prep::ident::DatatypeIdent};
use itertools::Itertools;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

/// Intrinsic datatypes known and specially handled
pub enum IntrinsicType {
    Bitvec,
    String,
    Object,
}

impl IntrinsicType {
    pub fn try_parse_ident(ident: &DatatypeIdent) -> Option<Self> {
        if ident.address() != AccountAddress::ONE {
            return None;
        }
        let parsed = match (ident.module_name(), ident.datatype_name()) {
            ("bit_vector", "BitVector") => IntrinsicType::Bitvec,
            ("string", "String") => IntrinsicType::String,
            ("object", "Object") => IntrinsicType::Object,
            _ => return None,
        };
        Some(parsed)
    }
}

/// A specific type instance within a typing context
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum TypeTag {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    U256,
    I256,
    Bitvec,
    String,
    Address,
    Signer,
    Vector {
        element: Box<Self>,
    },
    Datatype {
        ident: DatatypeIdent,
        type_args: Vec<Self>,
    },
    Param(usize),
    ObjectKnown {
        ident: DatatypeIdent,
        type_args: Vec<Self>,
    },
    ObjectParam(usize),
    Function {
        params: Vec<TypeRef>,
        returns: Vec<TypeRef>,
        abilities: AbilitySet,
    },
}

impl Display for TypeTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool => write!(f, "bool"),
            Self::U8 => write!(f, "u8"),
            Self::I8 => write!(f, "i8"),
            Self::U16 => write!(f, "u16"),
            Self::I16 => write!(f, "i16"),
            Self::U32 => write!(f, "u32"),
            Self::I32 => write!(f, "i32"),
            Self::U64 => write!(f, "u64"),
            Self::I64 => write!(f, "i64"),
            Self::U128 => write!(f, "u128"),
            Self::I128 => write!(f, "i128"),
            Self::U256 => write!(f, "u256"),
            Self::I256 => write!(f, "i256"),
            Self::Bitvec => write!(f, "std::bit_vector::BitVector"),
            Self::String => write!(f, "std::string::String"),
            Self::Address => write!(f, "address"),
            Self::Signer => write!(f, "signer"),
            Self::Vector { element } => write!(f, "vector<{element}>"),
            Self::Datatype { ident, type_args } => {
                if type_args.is_empty() {
                    write!(f, "{ident}")
                } else {
                    let inst = type_args.iter().join(", ");
                    write!(f, "{ident}<{inst}>")
                }
            },
            Self::Param(index) => write!(f, "#{index}"),
            Self::ObjectKnown { ident, type_args } => {
                if type_args.is_empty() {
                    write!(f, "aptos_framework::object::Object<{ident}>")
                } else {
                    let inst = type_args.iter().join(", ");
                    write!(f, "aptos_framework::object::Object<{ident}<{inst}>>")
                }
            },
            Self::ObjectParam(index) => write!(f, "aptos_framework::object::Object<#{index}>"),
            Self::Function {
                params,
                returns,
                abilities: _,
            } => {
                let params_str = params.iter().join(", ");
                let returns_str = returns.iter().join(", ");
                write!(f, "|{params_str}| ({returns_str})")
            },
        }
    }
}

/// A type token that can appear in function declarations: a [`TypeTag`] under
/// one of Move's three reference modes; see [`Refty`].
pub type TypeRef = Refty<TypeTag>;

/// A type instance with concrete execution semantics
///
/// This enum is intentionally kept in-sync with `TypeTag`,
/// with the addition of `abilities` information for datatypes and generics.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum TypeBase {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    U256,
    I256,
    Bitvec,
    String,
    Address,
    Signer,
    Vector {
        element: Box<Self>,
    },
    Datatype {
        ident: DatatypeIdent,
        type_args: Vec<Self>,
        abilities: AbilitySet,
    },
    Param {
        index: usize,
        abilities: AbilitySet,
    },
    ObjectKnown {
        ident: DatatypeIdent,
        type_args: Vec<Self>,
        abilities: AbilitySet,
    },
    ObjectParam {
        index: usize,
        abilities: AbilitySet,
    },
    Function {
        params: Vec<TypeItem>,
        returns: Vec<TypeItem>,
        abilities: AbilitySet,
    },
}

impl TypeBase {
    /// Retrieve the abilities of this type base
    pub fn abilities(&self) -> AbilitySet {
        match self {
            Self::Bool
            | Self::U8
            | Self::I8
            | Self::U16
            | Self::I16
            | Self::U32
            | Self::I32
            | Self::U64
            | Self::I64
            | Self::U128
            | Self::I128
            | Self::U256
            | Self::I256
            | Self::Bitvec
            | Self::String
            | Self::Address
            | Self::ObjectKnown { .. }
            | Self::ObjectParam { .. } => AbilitySet::PRIMITIVES,
            Self::Signer => AbilitySet::SIGNER,
            Self::Vector { element } => {
                let mut actual_abilities = AbilitySet::EMPTY;
                let provided_abilities = element.abilities();
                for ability in AbilitySet::VECTOR {
                    let required = ability.requires();
                    if provided_abilities.has_ability(required) {
                        actual_abilities = actual_abilities | ability;
                    }
                }
                actual_abilities
            },
            Self::Datatype {
                ident: _,
                type_args: _,
                abilities,
            } => *abilities,
            Self::Param {
                index: _,
                abilities,
            } => *abilities,
            Self::Function { abilities, .. } => *abilities,
        }
    }

    /// Collected involved type parameters
    pub fn involved_parameters(&self, params: &mut BTreeSet<usize>) {
        match self {
            Self::Bool
            | Self::U8
            | Self::I8
            | Self::U16
            | Self::I16
            | Self::U32
            | Self::I32
            | Self::U64
            | Self::I64
            | Self::U128
            | Self::I128
            | Self::U256
            | Self::I256
            | Self::Bitvec
            | Self::String
            | Self::Address
            | Self::Signer => {},
            Self::Vector { element } => element.involved_parameters(params),
            Self::Datatype {
                ident: _,
                type_args,
                abilities: _,
            } => {
                for arg in type_args {
                    arg.involved_parameters(params);
                }
            },
            Self::Param {
                index,
                abilities: _,
            } => {
                params.insert(*index);
            },
            Self::ObjectKnown {
                ident: _,
                type_args,
                abilities: _,
            } => {
                for arg in type_args {
                    arg.involved_parameters(params);
                }
            },
            Self::ObjectParam {
                index,
                abilities: _,
            } => {
                params.insert(*index);
            },
            Self::Function {
                params: fn_params,
                returns,
                abilities: _,
            } => {
                for t in fn_params {
                    t.involved_parameters(params);
                }
                for t in returns {
                    t.involved_parameters(params);
                }
            },
        }
    }
}

/// How a type parameter is spelled when a type is rendered.
///
/// The same `TypeBase` / `TypeItem` tree is printed for two audiences and the
/// two renderings differ in exactly one place, so they share one renderer
/// instead of two copies that silently drift when a variant is added.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ParamStyle {
    /// `#0` - diagnostics, traces and canvas dedup keys.
    Diagnostic,
    /// `T0` - the declaration name used in generated Move source.
    Source,
}

impl TypeBase {
    /// Render this type, spelling type parameters according to `style`.
    ///
    /// This is the *only* renderer for `TypeBase`: `Display` is
    /// `render(ParamStyle::Diagnostic)`, and the script generator in
    /// `prep::canvas` uses `render(ParamStyle::Source)`.
    pub fn render(&self, style: ParamStyle) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::U8 => "u8".to_string(),
            Self::I8 => "i8".to_string(),
            Self::U16 => "u16".to_string(),
            Self::I16 => "i16".to_string(),
            Self::U32 => "u32".to_string(),
            Self::I32 => "i32".to_string(),
            Self::U64 => "u64".to_string(),
            Self::I64 => "i64".to_string(),
            Self::U128 => "u128".to_string(),
            Self::I128 => "i128".to_string(),
            Self::U256 => "u256".to_string(),
            Self::I256 => "i256".to_string(),
            Self::Bitvec => "std::bit_vector::BitVector".to_string(),
            Self::String => "std::string::String".to_string(),
            Self::Address => "address".to_string(),
            Self::Signer => "signer".to_string(),
            Self::Vector { element } => format!("vector<{}>", element.render(style)),
            Self::Datatype {
                ident,
                type_args,
                abilities: _,
            } => {
                if type_args.is_empty() {
                    ident.to_string()
                } else {
                    let inst = type_args.iter().map(|t| t.render(style)).join(", ");
                    format!("{ident}<{inst}>")
                }
            },
            Self::Param {
                index,
                abilities: _,
            } => match style {
                ParamStyle::Diagnostic => format!("#{index}"),
                ParamStyle::Source => format!("T{index}"),
            },
            Self::ObjectKnown {
                ident,
                type_args,
                abilities: _,
            } => {
                if type_args.is_empty() {
                    format!("aptos_framework::object::Object<{ident}>")
                } else {
                    let inst = type_args.iter().map(|t| t.render(style)).join(", ");
                    format!("aptos_framework::object::Object<{ident}<{inst}>>")
                }
            },
            Self::ObjectParam {
                index,
                abilities: _,
            } => match style {
                ParamStyle::Diagnostic => format!("aptos_framework::object::Object<#{index}>"),
                ParamStyle::Source => format!("aptos_framework::object::Object<T{index}>"),
            },
            Self::Function {
                params,
                returns,
                abilities: _,
            } => {
                let params_str = params.iter().map(|t| t.render(style)).join(", ");
                let returns_str = returns.iter().map(|t| t.render(style)).join(", ");
                format!("|{params_str}| ({returns_str})")
            },
        }
    }
}

impl Display for TypeBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render(ParamStyle::Diagnostic))
    }
}

/// A type token with concrete execution semantics: a [`TypeBase`] under one of
/// Move's three reference modes; see [`Refty`].
///
/// `Display` (inherited from `Refty`) renders type parameters as `#0`; call
/// `render` with [`ParamStyle::Source`] to emit Move source.
pub type TypeItem = Refty<TypeBase>;

impl Refty<TypeBase> {
    /// Retrieve the abilities of this type base
    pub fn abilities(&self) -> AbilitySet {
        match self {
            Self::Base(base) => base.abilities(),
            Self::ImmRef(_) | Self::MutRef(_) => AbilitySet::REFERENCES,
        }
    }

    /// Collected involved type parameters
    pub fn involved_parameters(&self, params: &mut BTreeSet<usize>) {
        self.base().involved_parameters(params)
    }

    /// Render this type, spelling type parameters according to `style`.
    pub fn render(&self, style: ParamStyle) -> String {
        match self {
            Self::Base(base) => base.render(style),
            Self::ImmRef(base) => format!("&{}", base.render(style)),
            Self::MutRef(base) => format!("&mut {}", base.render(style)),
        }
    }
}

/// Type unifier (type tag -> type base)
pub struct TypeSubstitution<'a> {
    generics: &'a [AbilitySet],
    unified: Vec<Option<TypeBase>>,
}

impl<'a> TypeSubstitution<'a> {
    /// Initialize a type unification context
    pub fn new(generics: &'a [AbilitySet]) -> Self {
        Self {
            generics,
            unified: vec![None; generics.len()],
        }
    }

    fn check_and_assign_param(&mut self, param: usize, ty: TypeBase) -> bool {
        assert!(param < self.generics.len());

        // check ability constraints
        if !self.generics[param].is_subset(ty.abilities()) {
            return false;
        }

        // check existing assignment (if any)
        let entry = self
            .unified
            .get_mut(param)
            .expect("must have a param entry");

        if let Some(existing) = entry {
            return existing == &ty;
        }

        // make the assignment
        *entry = Some(ty);
        true
    }

    /// Try to unify a type tag and a type base
    pub fn unify(&mut self, ty_tag: &TypeTag, ty_base: &TypeBase) -> bool {
        match (ty_tag, ty_base) {
            // direct unification
            (TypeTag::Bool, TypeBase::Bool)
            | (TypeTag::U8, TypeBase::U8)
            | (TypeTag::I8, TypeBase::I8)
            | (TypeTag::U16, TypeBase::U16)
            | (TypeTag::I16, TypeBase::I16)
            | (TypeTag::U32, TypeBase::U32)
            | (TypeTag::I32, TypeBase::I32)
            | (TypeTag::U64, TypeBase::U64)
            | (TypeTag::I64, TypeBase::I64)
            | (TypeTag::U128, TypeBase::U128)
            | (TypeTag::I128, TypeBase::I128)
            | (TypeTag::U256, TypeBase::U256)
            | (TypeTag::I256, TypeBase::I256)
            | (TypeTag::Bitvec, TypeBase::Bitvec)
            | (TypeTag::String, TypeBase::String)
            | (TypeTag::Address, TypeBase::Address)
            | (TypeTag::Signer, TypeBase::Signer) => true,

            // delegated unification
            (
                TypeTag::Vector {
                    element: element_tag,
                },
                TypeBase::Vector {
                    element: element_base,
                },
            ) => self.unify(element_tag, element_base),
            (
                TypeTag::Datatype {
                    ident: ident_tag,
                    type_args: type_args_tag,
                },
                TypeBase::Datatype {
                    ident: ident_base,
                    type_args: type_args_base,
                    abilities: _,
                },
            )
            | (
                TypeTag::ObjectKnown {
                    ident: ident_tag,
                    type_args: type_args_tag,
                },
                TypeBase::ObjectKnown {
                    ident: ident_base,
                    type_args: type_args_base,
                    abilities: _,
                },
            ) => ident_tag == ident_base && self.unify_all(type_args_tag, type_args_base),

            // param assignment
            (TypeTag::Param(param), ty) => self.check_and_assign_param(*param, ty.clone()),
            (TypeTag::ObjectParam(param), TypeBase::ObjectParam { index, abilities }) => {
                let ty = TypeBase::Param {
                    index: *index,
                    abilities: *abilities,
                };
                self.check_and_assign_param(*param, ty)
            },
            (
                TypeTag::ObjectParam(param),
                TypeBase::ObjectKnown {
                    ident,
                    type_args,
                    abilities,
                },
            ) => {
                let ty = TypeBase::Datatype {
                    ident: ident.clone(),
                    type_args: type_args.clone(),
                    abilities: *abilities,
                };
                self.check_and_assign_param(*param, ty)
            },

            // function type unification
            (
                TypeTag::Function {
                    params: params_tag,
                    returns: returns_tag,
                    abilities: abilities_tag,
                },
                TypeBase::Function {
                    params: params_base,
                    returns: returns_base,
                    abilities: abilities_base,
                },
            ) => {
                abilities_tag == abilities_base
                    && params_tag.len() == params_base.len()
                    && returns_tag.len() == returns_base.len()
                    && self.unify_all_refs(params_tag, params_base)
                    && self.unify_all_refs(returns_tag, returns_base)
            },

            // all other cases
            _ => false,
        }
    }

    /// Try to unify a series of (type_tag, type_base) pairs
    pub fn unify_all(&mut self, ty_tags: &[TypeTag], ty_bases: &[TypeBase]) -> bool {
        assert_eq!(ty_tags.len(), ty_bases.len());
        for (ty_tag, ty_base) in ty_tags.iter().zip(ty_bases.iter()) {
            if !self.unify(ty_tag, ty_base) {
                return false;
            }
        }
        true
    }

    /// Try to unify a series of (type_ref, type_item) pairs (for function type params/returns)
    fn unify_all_refs(&mut self, ty_refs: &[TypeRef], ty_items: &[TypeItem]) -> bool {
        assert_eq!(ty_refs.len(), ty_items.len());
        for (ty_ref, ty_item) in ty_refs.iter().zip(ty_items.iter()) {
            let unified = match (ty_ref, ty_item) {
                (TypeRef::Base(tag), TypeItem::Base(base))
                | (TypeRef::ImmRef(tag), TypeItem::ImmRef(base))
                | (TypeRef::MutRef(tag), TypeItem::MutRef(base)) => self.unify(tag, base),
                // reference kinds do not match
                _ => false,
            };
            if !unified {
                return false;
            }
        }
        true
    }

    /// Finish and return the type unification result
    pub fn finish(self) -> Vec<Option<TypeBase>> {
        self.unified
    }
}

/// An error for type inference
enum TIError {
    CyclicUnification,
}

type TIResult<T> = Result<T, TIError>;

macro_rules! ti_unwrap {
    ($item:expr) => {
        match ($item)? {
            None => return Ok(None),
            Some(__v) => __v,
        }
    };
}

/// An equivalence group in type unification
#[derive(Clone)]
struct TypeEquivGroup {
    vars: BTreeSet<usize>,
    sort: Option<TypeBase>,
    abilities: AbilitySet,
}

impl TypeEquivGroup {
    /// Represent this group with a type parameter at the minimum index
    fn base(&self) -> TypeBase {
        let index = *self.vars.first().expect("at least one type param");
        TypeBase::Param {
            index,
            abilities: self.abilities,
        }
    }

    /// Extract a type representing this group
    pub fn repr(&self) -> TypeBase {
        match self.sort.as_ref() {
            None => self.base(),
            Some(t) => t.clone(),
        }
    }
}

/// A type unification instance
#[derive(Clone)]
struct TypeUnifier {
    /// holds the set of possible candidates associated with each type parameter
    params: BTreeMap<usize, usize>,
    /// hold the equivalence groups
    groups: Vec<TypeEquivGroup>,
}

impl TypeUnifier {
    /// Create an empty type unification context
    pub fn new(params: &BTreeMap<usize, AbilitySet>) -> Self {
        let mut base = Self {
            params: BTreeMap::new(),
            groups: vec![],
        };
        for (index, abilities) in params {
            base.init_param(*index, *abilities);
        }
        base
    }

    /// Initialize a type parameter in the unification context
    fn init_param(&mut self, index: usize, abilities: AbilitySet) {
        // assign a fresh equivalence group to the type variable
        let group = TypeEquivGroup {
            vars: BTreeSet::from([index]),
            sort: None,
            abilities,
        };
        let group_index = self.groups.len();

        // register the param and the group
        self.groups.push(group);
        let existing = self.params.insert(index, group_index);
        assert!(existing.is_none());
    }

    /// Merge the type constraints
    fn merge_group(
        &mut self,
        l: usize,
        h: usize,
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeBase>> {
        let idx_l = *self.params.get(&l).unwrap();
        let idx_h = *self.params.get(&h).unwrap();

        // obtain groups
        let mut group_l = self.groups.get(idx_l).unwrap().clone();
        if !involved.is_disjoint(&group_l.vars) {
            return Err(TIError::CyclicUnification);
        }

        let group_h = self.groups.get(idx_h).unwrap().clone();
        if !involved.is_disjoint(&group_h.vars) {
            return Err(TIError::CyclicUnification);
        }

        // prevent recursive typing
        involved.extend(group_l.vars.iter().copied());
        involved.extend(group_h.vars.iter().copied());

        // nothing to do if they belong to the same group
        if idx_l == idx_h {
            return Ok(Some(group_l.repr()));
        }

        // unify the equivalence set, after a sanity check
        if !group_l.vars.is_disjoint(&group_h.vars) {
            panic!("non-disjoint equivalence set");
        }
        group_l.vars.extend(group_h.vars);

        // check whether they unity to the same type, if any
        match (group_l.sort.as_ref(), group_h.sort.as_ref()) {
            (None, None) => {
                // none of the groups have type inferred
            },
            (Some(_), None) => {
                // the lower group already has candidates
            },
            (None, Some(sort_h)) => {
                // propagate the type candidates to the lower group
                group_l.sort = Some(sort_h.clone());
            },
            (Some(sort_l), Some(sort_h)) => {
                // further unity (refine) the types, also check for mismatches
                let unified = ti_unwrap!(self.unify(sort_l, sort_h, involved));
                group_l.sort = Some(unified);
            },
        };

        // pre-calculate the inferred type
        let inferred = group_l.repr();

        // redirect the group for the type variable at a higher index
        *self.groups.get_mut(idx_l).unwrap() = group_l;
        *self.params.get_mut(&h).unwrap() = idx_l;

        // return the inferred type
        Ok(Some(inferred))
    }

    /// Assign the constraint
    fn update_group(
        &mut self,
        v: usize,
        t: &TypeBase,
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeBase>> {
        // obtain the group
        let idx = *self.params.get(&v).unwrap();
        let mut group = self.groups.get(idx).unwrap().clone();

        // decides on whether further unification is needed
        let inferred = match group.sort.as_ref() {
            None => {
                // propagate the type to the group
                t.clone()
            },
            Some(e) => {
                // further unity (refine) the types, also check for mismatches
                ti_unwrap!(self.unify(e, t, involved))
            },
        };

        // update the type in this group
        group.sort = Some(inferred.clone());

        // reset the equivalence of group for the type variable at a lower index
        *self.groups.get_mut(idx).unwrap() = group;

        // return the inferred type
        Ok(Some(inferred))
    }

    /// Unify two types
    pub fn unify(
        &mut self,
        lhs: &TypeBase,
        rhs: &TypeBase,
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<TypeBase>> {
        let inferred = match (lhs, rhs) {
            // direct unification
            (TypeBase::Bool, TypeBase::Bool) => TypeBase::Bool,
            (TypeBase::U8, TypeBase::U8) => TypeBase::U8,
            (TypeBase::I8, TypeBase::I8) => TypeBase::I8,
            (TypeBase::U16, TypeBase::U16) => TypeBase::U16,
            (TypeBase::I16, TypeBase::I16) => TypeBase::I16,
            (TypeBase::U32, TypeBase::U32) => TypeBase::U32,
            (TypeBase::I32, TypeBase::I32) => TypeBase::I32,
            (TypeBase::U64, TypeBase::U64) => TypeBase::U64,
            (TypeBase::I64, TypeBase::I64) => TypeBase::I64,
            (TypeBase::U128, TypeBase::U128) => TypeBase::U128,
            (TypeBase::I128, TypeBase::I128) => TypeBase::I128,
            (TypeBase::U256, TypeBase::U256) => TypeBase::U256,
            (TypeBase::I256, TypeBase::I256) => TypeBase::I256,
            (TypeBase::Bitvec, TypeBase::Bitvec) => TypeBase::Bitvec,
            (TypeBase::String, TypeBase::String) => TypeBase::String,
            (TypeBase::Address, TypeBase::Address) => TypeBase::Address,
            (TypeBase::Signer, TypeBase::Signer) => TypeBase::Signer,

            // delegated unification
            (
                TypeBase::Vector {
                    element: element_tag,
                },
                TypeBase::Vector {
                    element: element_base,
                },
            ) => TypeBase::Vector {
                element: Box::new(ti_unwrap!(self.unify(element_tag, element_base, involved))),
            },
            (
                TypeBase::Datatype {
                    ident: lhs_ident,
                    type_args: lhs_ty_args,
                    abilities: lhs_abilities,
                },
                TypeBase::Datatype {
                    ident: rhs_ident,
                    type_args: rhs_ty_args,
                    abilities: rhs_abilities,
                },
            ) => {
                if lhs_ident != rhs_ident || lhs_abilities != rhs_abilities {
                    return Ok(None);
                }
                TypeBase::Datatype {
                    ident: lhs_ident.clone(),
                    type_args: ti_unwrap!(self.unify_all(lhs_ty_args, rhs_ty_args, involved)),
                    abilities: *lhs_abilities,
                }
            },
            (
                TypeBase::ObjectKnown {
                    ident: lhs_ident,
                    type_args: lhs_ty_args,
                    abilities: lhs_abilities,
                },
                TypeBase::ObjectKnown {
                    ident: rhs_ident,
                    type_args: rhs_ty_args,
                    abilities: rhs_abilities,
                },
            ) => {
                if lhs_ident != rhs_ident || lhs_abilities != rhs_abilities {
                    return Ok(None);
                }
                TypeBase::ObjectKnown {
                    ident: lhs_ident.clone(),
                    type_args: ti_unwrap!(self.unify_all(lhs_ty_args, rhs_ty_args, involved)),
                    abilities: *lhs_abilities,
                }
            },

            // param-param unification
            (
                TypeBase::Param {
                    index: lhs_index,
                    abilities: lhs_abilities,
                },
                TypeBase::Param {
                    index: rhs_index,
                    abilities: rhs_abilities,
                },
            ) => {
                if lhs_abilities != rhs_abilities {
                    return Ok(None);
                }
                match Ord::cmp(lhs_index, rhs_index) {
                    Ordering::Equal => {
                        // no knowledge gain in this case
                        TypeBase::Param {
                            index: *lhs_index,
                            abilities: *lhs_abilities,
                        }
                    },
                    Ordering::Less => {
                        ti_unwrap!(self.merge_group(*lhs_index, *rhs_index, involved))
                    },
                    Ordering::Greater => {
                        ti_unwrap!(self.merge_group(*rhs_index, *lhs_index, involved))
                    },
                }
            },
            (
                TypeBase::ObjectParam {
                    index: lhs_index,
                    abilities: lhs_abilities,
                },
                TypeBase::ObjectParam {
                    index: rhs_index,
                    abilities: rhs_abilities,
                },
            ) => {
                if lhs_abilities != rhs_abilities {
                    return Ok(None);
                }
                match ti_unwrap!(self.merge_group(*lhs_index, *rhs_index, involved)) {
                    TypeBase::Param { index, abilities } => {
                        TypeBase::ObjectParam { index, abilities }
                    },
                    TypeBase::Datatype {
                        ident,
                        type_args,
                        abilities,
                    } => TypeBase::ObjectKnown {
                        ident,
                        type_args,
                        abilities,
                    },
                    _ => {
                        unreachable!(
                            "expected symmetric object param unification to yield a param or datatype"
                        )
                    },
                }
            },

            // param-type unification
            (TypeBase::Param { index, abilities }, ty)
            | (ty, TypeBase::Param { index, abilities }) => {
                if abilities != &ty.abilities() {
                    return Ok(None);
                }
                ti_unwrap!(self.update_group(*index, ty, involved))
            },
            (
                TypeBase::ObjectParam {
                    index,
                    abilities: param_abilities,
                },
                TypeBase::ObjectKnown {
                    ident,
                    type_args,
                    abilities,
                },
            )
            | (
                TypeBase::ObjectKnown {
                    ident,
                    type_args,
                    abilities,
                },
                TypeBase::ObjectParam {
                    index,
                    abilities: param_abilities,
                },
            ) => {
                if param_abilities != abilities {
                    return Ok(None);
                }
                let ty = TypeBase::Datatype {
                    ident: ident.clone(),
                    type_args: type_args.clone(),
                    abilities: *abilities,
                };
                match ti_unwrap!(self.update_group(*index, &ty, involved)) {
                    TypeBase::Datatype {
                        ident,
                        type_args,
                        abilities,
                    } => TypeBase::ObjectKnown {
                        ident,
                        type_args,
                        abilities,
                    },
                    _ => unreachable!(
                        "expected asymmetric object param unification to yield datatype only"
                    ),
                }
            },

            // function type unification
            (
                TypeBase::Function {
                    params: lhs_params,
                    returns: lhs_returns,
                    abilities: lhs_abilities,
                },
                TypeBase::Function {
                    params: rhs_params,
                    returns: rhs_returns,
                    abilities: rhs_abilities,
                },
            ) => {
                if lhs_abilities != rhs_abilities
                    || lhs_params.len() != rhs_params.len()
                    || lhs_returns.len() != rhs_returns.len()
                {
                    return Ok(None);
                }
                TypeBase::Function {
                    params: ti_unwrap!(self.unify_all_items(lhs_params, rhs_params, involved)),
                    returns: ti_unwrap!(self.unify_all_items(lhs_returns, rhs_returns, involved)),
                    abilities: *lhs_abilities,
                }
            },

            // all other cases are considered mismatch
            _ => return Ok(None),
        };

        // return the inferred type
        Ok(Some(inferred))
    }

    /// Try to unify a series of (type_base, type_base) pairs
    pub fn unify_all(
        &mut self,
        lhs: &[TypeBase],
        rhs: &[TypeBase],
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<Vec<TypeBase>>> {
        assert_eq!(lhs.len(), rhs.len());
        let mut result = vec![];
        for (l, r) in lhs.iter().zip(rhs.iter()) {
            let unified = ti_unwrap!(self.unify(l, r, involved));
            result.push(unified);
        }
        Ok(Some(result))
    }

    /// Try to unify a series of (type_item, type_item) pairs (for function type params/returns)
    fn unify_all_items(
        &mut self,
        lhs: &[TypeItem],
        rhs: &[TypeItem],
        involved: &mut BTreeSet<usize>,
    ) -> TIResult<Option<Vec<TypeItem>>> {
        assert_eq!(lhs.len(), rhs.len());
        let mut result = vec![];
        for (l, r) in lhs.iter().zip(rhs.iter()) {
            let unified = match (l, r) {
                (TypeItem::Base(l_base), TypeItem::Base(r_base)) => {
                    TypeItem::Base(ti_unwrap!(self.unify(l_base, r_base, involved)))
                },
                (TypeItem::ImmRef(l_base), TypeItem::ImmRef(r_base)) => {
                    TypeItem::ImmRef(ti_unwrap!(self.unify(l_base, r_base, involved)))
                },
                (TypeItem::MutRef(l_base), TypeItem::MutRef(r_base)) => {
                    TypeItem::MutRef(ti_unwrap!(self.unify(l_base, r_base, involved)))
                },
                _ => return Ok(None),
            };
            result.push(unified);
        }
        Ok(Some(result))
    }

    /// Retrieve the type behind the type variable
    pub fn retrieve_type(&self, index: usize) -> TypeBase {
        let idx = *self.params.get(&index).unwrap();
        self.groups.get(idx).unwrap().repr()
    }
}

/// Type unifier (type base -> type base)
pub struct TypeUnification {
    unifier: TypeUnifier,
}

impl TypeUnification {
    /// Initialize a type unification context
    pub fn new(params: &BTreeMap<usize, AbilitySet>) -> Self {
        Self {
            unifier: TypeUnifier::new(params),
        }
    }

    /// Unify two types
    pub fn unify(&mut self, lhs: &TypeBase, rhs: &TypeBase) -> Option<TypeBase> {
        let mut involved = BTreeSet::new();
        self.unifier.unify(lhs, rhs, &mut involved).unwrap_or(None)
    }

    /// Finish and return the type unification result
    pub fn finish(self) -> BTreeMap<usize, TypeBase> {
        let mut result = BTreeMap::new();
        for &index in self.unifier.params.keys() {
            let ty = self.unifier.retrieve_type(index);
            if matches!(ty, TypeBase::Param { index: param_index, .. } if param_index == index) {
                continue;
            }
            result.insert(index, ty);
        }
        result
    }
}

/// Types that can be trivially constructed and destructed
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum SimpleType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    U128,
    I128,
    U256,
    I256,
    Bitvec,
    String,
    Address,
    Signer,
    Vector {
        element: Box<Self>,
    },
    ObjectKnown {
        ident: DatatypeIdent,
        type_args: Vec<TypeBase>,
        abilities: AbilitySet,
    },
    ObjectParam {
        index: usize,
        abilities: AbilitySet,
    },
    Function {
        params: Vec<TypeItem>,
        returns: Vec<TypeItem>,
        abilities: AbilitySet,
    },
}

impl SimpleType {
    /// Revert it back to a `TypeBase`
    pub fn revert(&self) -> TypeBase {
        match self {
            Self::Bool => TypeBase::Bool,
            Self::U8 => TypeBase::U8,
            Self::I8 => TypeBase::I8,
            Self::U16 => TypeBase::U16,
            Self::I16 => TypeBase::I16,
            Self::U32 => TypeBase::U32,
            Self::I32 => TypeBase::I32,
            Self::U64 => TypeBase::U64,
            Self::I64 => TypeBase::I64,
            Self::U128 => TypeBase::U128,
            Self::I128 => TypeBase::I128,
            Self::U256 => TypeBase::U256,
            Self::I256 => TypeBase::I256,
            Self::Bitvec => TypeBase::Bitvec,
            Self::String => TypeBase::String,
            Self::Address => TypeBase::Address,
            Self::Signer => TypeBase::Signer,
            Self::Vector { element } => TypeBase::Vector {
                element: Box::new(element.revert()),
            },
            Self::ObjectKnown {
                ident,
                type_args,
                abilities,
            } => TypeBase::ObjectKnown {
                ident: ident.clone(),
                type_args: type_args.clone(),
                abilities: *abilities,
            },
            Self::ObjectParam { index, abilities } => TypeBase::ObjectParam {
                index: *index,
                abilities: *abilities,
            },
            Self::Function {
                params,
                returns,
                abilities,
            } => TypeBase::Function {
                params: params.clone(),
                returns: returns.clone(),
                abilities: *abilities,
            },
        }
    }
}

/// A type constructed based on datatypes or parameters (cannot be trivially handled)
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum ComplexType {
    Datatype {
        ident: DatatypeIdent,
        type_args: Vec<TypeBase>,
        abilities: AbilitySet,
    },
    Param {
        index: usize,
        abilities: AbilitySet,
    },
    Vector {
        element: Box<Self>,
    },
}

impl ComplexType {
    /// Revert it back to a `TypeBase`
    pub fn revert(&self) -> TypeBase {
        match self {
            Self::Datatype {
                ident,
                type_args,
                abilities,
            } => TypeBase::Datatype {
                ident: ident.clone(),
                type_args: type_args.clone(),
                abilities: *abilities,
            },
            Self::Param { index, abilities } => TypeBase::Param {
                index: *index,
                abilities: *abilities,
            },
            Self::Vector { element } => TypeBase::Vector {
                element: Box::new(element.revert()),
            },
        }
    }
}

/// Either a simple type or a complex type
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum TypeMode {
    Simple(SimpleType),
    Complex(ComplexType),
}

impl TypeMode {
    /// Convert a type base into a type mode.
    ///
    /// Returns `None` when the shape cannot be materialized by the driver generator.
    /// Currently the only such shape is a vector of function values. Callers must
    /// degrade gracefully and skip the enclosing entrypoint instead of failing.
    pub fn convert(t: &TypeBase) -> Option<Self> {
        let converted = match t {
            TypeBase::Bool => Self::Simple(SimpleType::Bool),
            TypeBase::U8 => Self::Simple(SimpleType::U8),
            TypeBase::I8 => Self::Simple(SimpleType::I8),
            TypeBase::U16 => Self::Simple(SimpleType::U16),
            TypeBase::I16 => Self::Simple(SimpleType::I16),
            TypeBase::U32 => Self::Simple(SimpleType::U32),
            TypeBase::I32 => Self::Simple(SimpleType::I32),
            TypeBase::U64 => Self::Simple(SimpleType::U64),
            TypeBase::I64 => Self::Simple(SimpleType::I64),
            TypeBase::U128 => Self::Simple(SimpleType::U128),
            TypeBase::I128 => Self::Simple(SimpleType::I128),
            TypeBase::U256 => Self::Simple(SimpleType::U256),
            TypeBase::I256 => Self::Simple(SimpleType::I256),
            TypeBase::Bitvec => Self::Simple(SimpleType::Bitvec),
            TypeBase::String => Self::Simple(SimpleType::String),
            TypeBase::Address => Self::Simple(SimpleType::Address),
            TypeBase::Signer => Self::Simple(SimpleType::Signer),
            TypeBase::Vector { element } => match Self::convert(element)? {
                // A vector of function values cannot be materialized: lambdas are bound
                // per-parameter (see `LambdaBinding`), not per-element, so there is no way
                // to synthesize the elements. Report the shape as unsupported so the
                // enclosing entrypoint is skipped rather than crashing the run.
                Self::Simple(SimpleType::Function { .. }) => return None,
                Self::Simple(elem_simple) => Self::Simple(SimpleType::Vector {
                    element: Box::new(elem_simple),
                }),
                Self::Complex(elem_complex) => Self::Complex(ComplexType::Vector {
                    element: Box::new(elem_complex),
                }),
            },
            TypeBase::Datatype {
                ident,
                type_args,
                abilities,
            } => Self::Complex(ComplexType::Datatype {
                ident: ident.clone(),
                type_args: type_args.clone(),
                abilities: *abilities,
            }),
            TypeBase::Param { index, abilities } => Self::Complex(ComplexType::Param {
                index: *index,
                abilities: *abilities,
            }),
            TypeBase::ObjectKnown {
                ident,
                type_args,
                abilities,
            } => Self::Simple(SimpleType::ObjectKnown {
                ident: ident.clone(),
                type_args: type_args.clone(),
                abilities: *abilities,
            }),
            TypeBase::ObjectParam { index, abilities } => Self::Simple(SimpleType::ObjectParam {
                index: *index,
                abilities: *abilities,
            }),
            TypeBase::Function {
                params: fn_params,
                returns,
                abilities,
            } => Self::Simple(SimpleType::Function {
                params: fn_params.clone(),
                returns: returns.clone(),
                abilities: *abilities,
            }),
        };
        Some(converted)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SimpleType, TypeBase, TypeItem, TypeMode, TypeRef, TypeSubstitution, TypeTag,
        TypeUnification,
    };
    use crate::prep::ident::DatatypeIdent;
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn datatype(name: &str) -> DatatypeIdent {
        DatatypeIdent::from_struct_tuple(
            AccountAddress::ONE,
            Identifier::new("m").unwrap(),
            Identifier::new(name).unwrap(),
        )
    }

    #[test]
    fn test_type_substitution_unifies_generic_datatype_arguments() {
        let mut subst = TypeSubstitution::new(&[AbilitySet::PRIMITIVES]);
        let tag = TypeTag::Datatype {
            ident: datatype("Box"),
            type_args: vec![TypeTag::Param(0)],
        };
        let base = TypeBase::Datatype {
            ident: datatype("Box"),
            type_args: vec![TypeBase::U64],
            abilities: AbilitySet::PRIMITIVES,
        };
        assert!(subst.unify(&tag, &base));
        assert_eq!(subst.finish(), vec![Some(TypeBase::U64)]);
    }

    #[test]
    fn test_type_substitution_rejects_unsatisfied_ability_constraints() {
        let constraints = [AbilitySet::EMPTY.add(Ability::Key)];
        let mut subst = TypeSubstitution::new(&constraints);
        assert!(!subst.unify(&TypeTag::Param(0), &TypeBase::U64));
    }

    #[test]
    fn test_type_unification_merges_params_transitively() {
        let params = BTreeMap::from([
            (0usize, AbilitySet::PRIMITIVES),
            (1usize, AbilitySet::PRIMITIVES),
        ]);
        let mut unifier = TypeUnification::new(&params);
        let lhs = TypeBase::Vector {
            element: Box::new(TypeBase::Param {
                index: 0,
                abilities: AbilitySet::PRIMITIVES,
            }),
        };
        let rhs = TypeBase::Vector {
            element: Box::new(TypeBase::Param {
                index: 1,
                abilities: AbilitySet::PRIMITIVES,
            }),
        };
        let unified = unifier.unify(&lhs, &rhs).unwrap();
        assert_eq!(unified, lhs);
        assert_eq!(
            unifier.finish(),
            BTreeMap::from([(1usize, TypeBase::Param {
                index: 0,
                abilities: AbilitySet::PRIMITIVES,
            })])
        );
    }

    #[test]
    fn test_type_unification_unifies_function_types() {
        let params = BTreeMap::from([(0usize, AbilitySet::PRIMITIVES)]);
        let mut unifier = TypeUnification::new(&params);
        let lhs = TypeBase::Function {
            params: vec![TypeItem::Base(TypeBase::Param {
                index: 0,
                abilities: AbilitySet::PRIMITIVES,
            })],
            returns: vec![TypeItem::ImmRef(TypeBase::Address)],
            abilities: AbilitySet::EMPTY,
        };
        let rhs = TypeBase::Function {
            params: vec![TypeItem::Base(TypeBase::U64)],
            returns: vec![TypeItem::ImmRef(TypeBase::Address)],
            abilities: AbilitySet::EMPTY,
        };
        assert_eq!(unifier.unify(&lhs, &rhs), Some(rhs.clone()));
        assert_eq!(unifier.finish(), BTreeMap::from([(0usize, TypeBase::U64)]));
    }

    #[test]
    fn test_type_base_involved_parameters_collects_nested_params() {
        let mut params = BTreeSet::new();
        let ty = TypeBase::Function {
            params: vec![TypeItem::Base(TypeBase::Vector {
                element: Box::new(TypeBase::Param {
                    index: 1,
                    abilities: AbilitySet::PRIMITIVES,
                }),
            })],
            returns: vec![TypeItem::MutRef(TypeBase::ObjectParam {
                index: 3,
                abilities: AbilitySet::PRIMITIVES,
            })],
            abilities: AbilitySet::EMPTY,
        };
        ty.involved_parameters(&mut params);
        assert_eq!(params, BTreeSet::from([1usize, 3usize]));
    }

    #[test]
    fn test_type_ref_display_formats_function_signatures() {
        let ty = TypeRef::Base(TypeTag::Function {
            params: vec![TypeRef::Base(TypeTag::U64)],
            returns: vec![TypeRef::MutRef(TypeTag::Address)],
            abilities: AbilitySet::EMPTY,
        });
        assert_eq!(ty.to_string(), "|u64| (&mut address)");
    }

    #[test]
    fn test_type_substitution_rejects_function_type_with_mismatched_param() {
        // `|u64|` must not unify against `|bool|`: the reference kinds match (both `Base`),
        // so only the inner `unify` can reject the pair.
        let mut subst = TypeSubstitution::new(&[]);
        let tag = TypeTag::Function {
            params: vec![TypeRef::Base(TypeTag::U64)],
            returns: vec![],
            abilities: AbilitySet::EMPTY,
        };
        let base = TypeBase::Function {
            params: vec![TypeItem::Base(TypeBase::Bool)],
            returns: vec![],
            abilities: AbilitySet::EMPTY,
        };
        assert!(!subst.unify(&tag, &base));
    }

    #[test]
    fn test_type_substitution_rejects_function_type_with_conflicting_param_assignment() {
        // `|#0, #0|` must not unify against `|u64, bool|`: the second assignment of `#0`
        // conflicts with the first, which would otherwise leave a fully resolved but
        // ill-typed substitution `[u64]`.
        let constraints = [AbilitySet::PRIMITIVES];
        let mut subst = TypeSubstitution::new(&constraints);
        let tag = TypeTag::Function {
            params: vec![
                TypeRef::Base(TypeTag::Param(0)),
                TypeRef::Base(TypeTag::Param(0)),
            ],
            returns: vec![],
            abilities: AbilitySet::EMPTY,
        };
        let base = TypeBase::Function {
            params: vec![
                TypeItem::Base(TypeBase::U64),
                TypeItem::Base(TypeBase::Bool),
            ],
            returns: vec![],
            abilities: AbilitySet::EMPTY,
        };
        assert!(!subst.unify(&tag, &base));
    }

    #[test]
    fn test_type_substitution_unifies_matching_function_type_refs() {
        let constraints = [AbilitySet::PRIMITIVES];
        let mut subst = TypeSubstitution::new(&constraints);
        let tag = TypeTag::Function {
            params: vec![
                TypeRef::Base(TypeTag::Param(0)),
                TypeRef::ImmRef(TypeTag::Address),
            ],
            returns: vec![TypeRef::MutRef(TypeTag::Bool)],
            abilities: AbilitySet::EMPTY,
        };
        let base = TypeBase::Function {
            params: vec![
                TypeItem::Base(TypeBase::U64),
                TypeItem::ImmRef(TypeBase::Address),
            ],
            returns: vec![TypeItem::MutRef(TypeBase::Bool)],
            abilities: AbilitySet::EMPTY,
        };
        assert!(subst.unify(&tag, &base));
        assert_eq!(subst.finish(), vec![Some(TypeBase::U64)]);
    }

    #[test]
    fn test_type_mode_convert_rejects_vector_of_function_values() {
        let fn_ty = TypeBase::Function {
            params: vec![TypeItem::Base(TypeBase::U64)],
            returns: vec![TypeItem::Base(TypeBase::U64)],
            abilities: AbilitySet::PRIMITIVES,
        };

        // a bare function value is still supported (bound via a lambda binding)
        assert!(matches!(
            TypeMode::convert(&fn_ty),
            Some(TypeMode::Simple(SimpleType::Function { .. }))
        ));

        // ... but a vector of them is unsupported and must be skipped, not panic
        assert!(TypeMode::convert(&TypeBase::Vector {
            element: Box::new(fn_ty.clone()),
        })
        .is_none());

        // rejection propagates through nesting
        assert!(TypeMode::convert(&TypeBase::Vector {
            element: Box::new(TypeBase::Vector {
                element: Box::new(fn_ty),
            }),
        })
        .is_none());

        // ordinary vectors are unaffected
        assert!(matches!(
            TypeMode::convert(&TypeBase::Vector {
                element: Box::new(TypeBase::U64),
            }),
            Some(TypeMode::Simple(SimpleType::Vector { .. }))
        ));
    }
}
