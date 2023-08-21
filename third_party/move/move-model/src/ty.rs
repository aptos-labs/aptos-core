// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains types and related functions.

use crate::{
    ast::QualifiedSymbol,
    model::{GlobalEnv, Loc, ModuleId, QualifiedInstId, StructEnv, StructId},
    symbol::Symbol,
};
use itertools::Itertools;
use move_binary_format::{file_format::TypeParameterIndex, normalized::Type as MType};
use move_core_types::{
    language_storage::{StructTag, TypeTag},
    u256::U256,
};
use num::BigInt;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt,
    fmt::{Debug, Formatter},
};

/// Represents a type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Tuple(Vec<Type>),
    Vector(Box<Type>),
    Struct(ModuleId, StructId, Vec<Type>),
    TypeParameter(u16),

    // Types only appearing in programs.
    Reference(ReferenceKind, Box<Type>),

    // Types only appearing in specifications
    Fun(Box<Type>, Box<Type>),
    TypeDomain(Box<Type>),
    ResourceDomain(ModuleId, StructId, Option<Vec<Type>>),

    // Temporary types used during type checking
    Error,
    Var(u32),
}

/// Represents a reference kind.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum ReferenceKind {
    Immutable,
    Mutable,
}

impl ReferenceKind {
    pub fn from_is_mut(is_mut: bool) -> ReferenceKind {
        if is_mut {
            ReferenceKind::Mutable
        } else {
            ReferenceKind::Immutable
        }
    }
}

pub const BOOL_TYPE: Type = Type::Primitive(PrimitiveType::Bool);
pub const NUM_TYPE: Type = Type::Primitive(PrimitiveType::Num);

/// Represents a primitive (builtin) type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    // Types only appearing in specifications
    Num,
    Range,
    EventStore,
}

/// A type substitution.
#[derive(Debug, Clone)]
pub struct Substitution {
    /// Assignment of types to variables.
    subs: BTreeMap<u32, Type>,
    /// Constraints on (unassigned) variables.
    constraints: BTreeMap<u32, Vec<(Loc, WideningOrder, Constraint)>>,
}

/// A constraint on a type variable, maintained during unification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Constraint {
    /// The type variable must be instantiated with one of the given numbber types. This is used
    /// for representing integer constants.
    SomeNumber(BTreeSet<PrimitiveType>),
    /// The type variable must be instantiated with a reference of given type.
    SomeReference(Type),
    /// The type variable defaults to the given type if no other binding is found. This is
    /// a pseudo constraint which never fails, but used to generate a default for
    /// inference.
    WithDefault(Type),
}

impl Constraint {
    /// Returns the default type of a constraint. A the end of type unification, variables
    /// with constraints that have defaults will be substituted by those defaults.
    fn default_type(&self) -> Option<Type> {
        match self {
            Constraint::SomeNumber(options) if options.contains(&PrimitiveType::U64) => {
                Some(Type::new_prim(PrimitiveType::U64))
            },
            Constraint::SomeReference(ty) => Some(Type::Reference(
                ReferenceKind::Immutable,
                Box::new(ty.clone()),
            )),
            Constraint::WithDefault(ty) => Some(ty.clone()),
            _ => None,
        }
    }
}

/// Represents an error resulting from type unification.
#[derive(Debug)]
pub enum TypeUnificationError {
    TypeMismatch(Type, Type),
    ArityMismatch(String, usize, usize),
    CyclicSubstitution(Type, Type),
    MutabilityMismatch(ReferenceKind, ReferenceKind),
    ConstraintUnsatisfied(Loc, Type, WideningOrder, Constraint),
    RedirectedError(Loc, Box<TypeUnificationError>),
}

impl PrimitiveType {
    /// Returns true if this type is a specification language only type
    pub fn is_spec(&self) -> bool {
        use PrimitiveType::*;
        match self {
            Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address | Signer => false,
            Num | Range | EventStore => true,
        }
    }

    /// Attempt to convert this type into a normalized::Type
    pub fn into_normalized_type(self) -> Option<MType> {
        use PrimitiveType::*;
        Some(match self {
            Bool => MType::Bool,
            U8 => MType::U8,
            U16 => MType::U16,
            U32 => MType::U32,
            U64 => MType::U64,
            U128 => MType::U128,
            U256 => MType::U256,
            Address => MType::Address,
            Signer => MType::Signer,
            Num | Range | EventStore => return None,
        })
    }

    /// Infer a type from a value. Returns the smallest type from u64 onwards which can fit the
    /// value.
    pub fn possible_int_types(value: BigInt) -> Vec<PrimitiveType> {
        Self::all_int_types()
            .into_iter()
            .filter(|t| value <= Self::get_max_value(t).expect("type has max"))
            .collect()
    }

    pub fn all_int_types() -> Vec<PrimitiveType> {
        vec![
            PrimitiveType::U8,
            PrimitiveType::U16,
            PrimitiveType::U32,
            PrimitiveType::U64,
            PrimitiveType::U128,
            PrimitiveType::U256,
        ]
    }

    /// Gets the maximal value allowed for a numeric type, or none if it is unbounded.
    pub fn get_max_value(self: &PrimitiveType) -> Option<BigInt> {
        match self {
            PrimitiveType::U8 => Some(BigInt::from(u8::MAX)),
            PrimitiveType::U16 => Some(BigInt::from(u16::MAX)),
            PrimitiveType::U32 => Some(BigInt::from(u32::MAX)),
            PrimitiveType::U64 => Some(BigInt::from(u64::MAX)),
            PrimitiveType::U128 => Some(BigInt::from(u128::MAX)),
            PrimitiveType::U256 => Some(BigInt::from(&U256::max_value())),
            PrimitiveType::Num => None,
            _ => unreachable!("no num type"),
        }
    }
}

impl Type {
    /// Create a new primitive type
    pub fn new_prim(p: PrimitiveType) -> Type {
        Type::Primitive(p)
    }

    /// Create a new type parameter
    pub fn new_param(pos: usize) -> Type {
        Type::TypeParameter(pos as u16)
    }

    /// Creates a unit type
    pub fn unit() -> Type {
        Type::Tuple(vec![])
    }

    /// Determines whether this is a type parameter.
    pub fn is_type_parameter(&self) -> bool {
        matches!(self, Type::TypeParameter(..))
    }

    /// Determines whether this is a reference.
    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Reference(_, _))
    }

    /// Determines whether this is a mutable reference.
    pub fn is_mutable_reference(&self) -> bool {
        matches!(self, Type::Reference(ReferenceKind::Mutable, _))
    }

    /// Determines whether this is an immutable reference.
    pub fn is_immutable_reference(&self) -> bool {
        matches!(self, Type::Reference(ReferenceKind::Immutable, _))
    }

    /// Determines whether this type is a struct.
    pub fn is_struct(&self) -> bool {
        matches!(self, Type::Struct(..))
    }

    /// Determines whether this is the error type.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Determines whether this type is a vector
    pub fn is_vector(&self) -> bool {
        matches!(self, Type::Vector(..))
    }

    /// Determines whether this is a struct, or a vector of structs, or a reference to any of
    /// those.
    pub fn is_struct_or_vector_of_struct(&self) -> bool {
        match self.skip_reference() {
            Type::Struct(..) => true,
            Type::Vector(ety) => ety.is_struct_or_vector_of_struct(),
            _ => false,
        }
    }

    /// Whether the type is allowed for a Move constant.
    pub fn is_valid_for_constant(&self) -> bool {
        use PrimitiveType::*;
        use Type::*;
        match self {
            Primitive(p) => matches!(p, U8 | U16 | U32 | U64 | U128 | U256 | Bool | Address),
            Vector(ety) => ety.is_valid_for_constant(),
            _ => false,
        }
    }

    /// Returns true if this type is a specification language only type or contains specification
    /// language only types
    pub fn is_spec(&self) -> bool {
        use Type::*;
        match self {
            Primitive(p) => p.is_spec(),
            Fun(..) | TypeDomain(..) | ResourceDomain(..) | Error => true,
            Var(..) | TypeParameter(..) => false,
            Tuple(ts) => ts.iter().any(|t| t.is_spec()),
            Struct(_, _, ts) => ts.iter().any(|t| t.is_spec()),
            Vector(et) => et.is_spec(),
            Reference(_, bt) => bt.is_spec(),
        }
    }

    /// Returns true if this is a bool.
    pub fn is_bool(&self) -> bool {
        if let Type::Primitive(PrimitiveType::Bool) = self {
            return true;
        }
        false
    }

    /// Returns true of this is a type variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Type::Var(_))
    }

    /// Returns true if this is any number type.
    pub fn is_number(&self) -> bool {
        if let Type::Primitive(p) = self {
            if let PrimitiveType::U8
            | PrimitiveType::U16
            | PrimitiveType::U32
            | PrimitiveType::U64
            | PrimitiveType::U128
            | PrimitiveType::U256
            | PrimitiveType::Num = p
            {
                return true;
            }
        }
        false
    }

    /// Returns true if this is an address or signer type.
    pub fn is_signer_or_address(&self) -> bool {
        matches!(
            self,
            Type::Primitive(PrimitiveType::Signer) | Type::Primitive(PrimitiveType::Address)
        )
    }

    /// Return true if this is an account address
    pub fn is_address(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Address))
    }

    /// Return true if this is an account address
    pub fn is_signer(&self) -> bool {
        matches!(self, Type::Primitive(PrimitiveType::Signer))
    }

    /// Test whether this type can be used to substitute a type parameter
    pub fn can_be_type_argument(&self) -> bool {
        match self {
            Type::Primitive(p) => !p.is_spec(),
            Type::Tuple(..) => false,
            Type::Vector(e) => e.can_be_type_argument(),
            Type::Struct(_, _, insts) => insts.iter().all(|e| e.can_be_type_argument()),
            Type::TypeParameter(..) => true,
            // references cannot be a type argument
            Type::Reference(..) => false,
            // spec types cannot be a type argument
            Type::Fun(..)
            | Type::TypeDomain(..)
            | Type::ResourceDomain(..)
            | Type::Var(..)
            | Type::Error => false,
        }
    }

    /// Skip reference type.
    pub fn skip_reference(&self) -> &Type {
        if let Type::Reference(_, bt) = self {
            bt
        } else {
            self
        }
    }

    /// If this is a struct type, replace the type instantiation.
    pub fn replace_struct_instantiation(&self, inst: &[Type]) -> Type {
        match self {
            Type::Struct(mid, sid, _) => Type::Struct(*mid, *sid, inst.to_vec()),
            _ => self.clone(),
        }
    }

    /// If this is a struct type, return the associated struct env and type parameters.
    pub fn get_struct<'env>(
        &'env self,
        env: &'env GlobalEnv,
    ) -> Option<(StructEnv<'env>, &'env [Type])> {
        if let Type::Struct(module_idx, struct_idx, params) = self {
            Some((env.get_module(*module_idx).into_struct(*struct_idx), params))
        } else {
            None
        }
    }

    /// If this is a struct type, return the associated QualifiedInstId.
    pub fn get_struct_id(&self, env: &GlobalEnv) -> Option<QualifiedInstId<StructId>> {
        self.get_struct(env).map(|(se, inst)| {
            se.module_env
                .get_id()
                .qualified(se.get_id())
                .instantiate(inst.to_vec())
        })
    }

    /// Require this to be a struct, if so extracts its content.
    pub fn require_struct(&self) -> (ModuleId, StructId, &[Type]) {
        if let Type::Struct(mid, sid, targs) = self {
            (*mid, *sid, targs.as_slice())
        } else {
            panic!("expected `Type::Struct`, found: `{:?}`", self)
        }
    }

    /// Instantiates type parameters in this type.
    pub fn instantiate(&self, params: &[Type]) -> Type {
        if params.is_empty() {
            self.clone()
        } else {
            self.replace(Some(params), None, false)
        }
    }

    /// Instantiate type parameters in the vector of types.
    pub fn instantiate_vec(vec: Vec<Type>, params: &[Type]) -> Vec<Type> {
        if params.is_empty() {
            vec
        } else {
            vec.into_iter().map(|ty| ty.instantiate(params)).collect()
        }
    }

    /// Instantiate type parameters in the slice of types.
    pub fn instantiate_slice(slice: &[Type], params: &[Type]) -> Vec<Type> {
        if params.is_empty() {
            slice.to_owned()
        } else {
            slice.iter().map(|ty| ty.instantiate(params)).collect()
        }
    }

    /// Convert a partial assignment for type parameters into an instantiation.
    pub fn type_param_map_to_inst(arity: usize, map: BTreeMap<u16, Type>) -> Vec<Type> {
        let mut inst: Vec<_> = (0..arity).map(Type::new_param).collect();
        for (idx, ty) in map {
            inst[idx as usize] = ty;
        }
        inst
    }

    /// A helper function to do replacement of type parameters.
    fn replace(
        &self,
        params: Option<&[Type]>,
        subs: Option<&Substitution>,
        use_constr: bool,
    ) -> Type {
        let replace_vec = |types: &[Type]| -> Vec<Type> {
            types
                .iter()
                .map(|t| t.replace(params, subs, use_constr))
                .collect()
        };
        match self {
            Type::TypeParameter(i) => {
                if let Some(ps) = params {
                    ps[*i as usize].clone()
                } else {
                    self.clone()
                }
            },
            Type::Var(i) => {
                if let Some(s) = subs {
                    if let Some(t) = s.subs.get(i) {
                        // Recursively call replacement again here, in case the substitution s
                        // refers to type variables.
                        // TODO: a more efficient approach is to maintain that type assignments
                        // are always fully specialized w.r.t. to the substitution.
                        t.replace(params, subs, use_constr)
                    } else if use_constr {
                        if let Some(default_ty) = s.constraints.get(i).and_then(|constrs| {
                            constrs.iter().find_map(|(_, _, c)| c.default_type())
                        }) {
                            default_ty
                        } else {
                            self.clone()
                        }
                    } else {
                        self.clone()
                    }
                } else {
                    self.clone()
                }
            },
            Type::Reference(kind, bt) => {
                Type::Reference(*kind, Box::new(bt.replace(params, subs, use_constr)))
            },
            Type::Struct(mid, sid, args) => Type::Struct(*mid, *sid, replace_vec(args)),
            Type::Fun(arg, result) => Type::Fun(
                Box::new(arg.replace(params, subs, use_constr)),
                Box::new(result.replace(params, subs, use_constr)),
            ),
            Type::Tuple(args) => Type::Tuple(replace_vec(args)),
            Type::Vector(et) => Type::Vector(Box::new(et.replace(params, subs, use_constr))),
            Type::TypeDomain(et) => {
                Type::TypeDomain(Box::new(et.replace(params, subs, use_constr)))
            },
            Type::ResourceDomain(mid, sid, args_opt) => {
                Type::ResourceDomain(*mid, *sid, args_opt.as_ref().map(|args| replace_vec(args)))
            },
            Type::Primitive(..) | Type::Error => self.clone(),
        }
    }

    /// Checks whether this type contains a type for which the predicate is true.
    pub fn contains<P>(&self, p: &P) -> bool
    where
        P: Fn(&Type) -> bool,
    {
        if p(self) {
            true
        } else {
            let contains_vec = |ts: &[Type]| ts.iter().any(p);
            match self {
                Type::Reference(_, bt) => bt.contains(p),
                Type::Struct(_, _, args) => contains_vec(args),
                Type::Fun(arg, result) => arg.contains(p) || result.contains(p),
                Type::Tuple(args) => contains_vec(args),
                Type::Vector(et) => et.contains(p),
                _ => false,
            }
        }
    }

    /// Returns true if this type is incomplete, i.e. contains any type variables.
    pub fn is_incomplete(&self) -> bool {
        use Type::*;
        match self {
            Var(_) => true,
            Tuple(ts) => ts.iter().any(|t| t.is_incomplete()),
            Fun(a, r) => a.is_incomplete() || r.is_incomplete(),
            Struct(_, _, ts) => ts.iter().any(|t| t.is_incomplete()),
            Vector(et) => et.is_incomplete(),
            Reference(_, bt) => bt.is_incomplete(),
            TypeDomain(bt) => bt.is_incomplete(),
            Error | Primitive(..) | TypeParameter(_) | ResourceDomain(..) => false,
        }
    }

    /// Return true if this type contains generic types (i.e., types that can be instantiated).
    pub fn is_open(&self) -> bool {
        let mut has_var = false;
        self.visit(&mut |t| has_var = has_var || matches!(t, Type::TypeParameter(_)));
        has_var
    }

    /// Compute used modules in this type, adding them to the passed set.
    pub fn module_usage(&self, usage: &mut BTreeSet<ModuleId>) {
        use Type::*;
        match self {
            Tuple(ts) => ts.iter().for_each(|t| t.module_usage(usage)),
            Fun(a, r) => {
                a.module_usage(usage);
                r.module_usage(usage);
            },
            Struct(mid, _, ts) => {
                usage.insert(*mid);
                ts.iter().for_each(|t| t.module_usage(usage));
            },
            Vector(et) => et.module_usage(usage),
            Reference(_, bt) => bt.module_usage(usage),
            TypeDomain(bt) => bt.module_usage(usage),
            _ => {},
        }
    }

    /// Attempt to convert this type into a normalized::Type
    pub fn into_struct_type(self, env: &GlobalEnv) -> Option<MType> {
        use Type::*;
        match self {
            Struct(mid, sid, ts) => env.get_struct_type(mid, sid, &ts),
            _ => None,
        }
    }

    /// Attempt to convert this type into a normalized::Type
    pub fn into_normalized_type(self, env: &GlobalEnv) -> Option<MType> {
        use Type::*;
        match self {
            Primitive(p) => Some(p.into_normalized_type().expect("Invariant violation: unexpected spec primitive")),
            Struct(mid, sid, ts) =>
                env.get_struct_type(mid, sid, &ts),
            Vector(et) => Some(MType::Vector(
                Box::new(et.into_normalized_type(env)
                    .expect("Invariant violation: vector type argument contains incomplete, tuple, or spec type"))
            )),
            Reference(r, t) =>
            match r {
                ReferenceKind::Mutable => {
                    Some(MType::MutableReference(Box::new(t.into_normalized_type(env).expect("Invariant violation: reference type contains incomplete, tuple, or spec type"))))
                },
                ReferenceKind::Immutable => {
                    Some(MType::Reference(Box::new(t.into_normalized_type(env).expect("Invariant violation: reference type contains incomplete, tuple, or spec type"))))
                },
            },
            TypeParameter(idx) => Some(MType::TypeParameter(idx)),
            Tuple(..) | Error | Fun(..) | TypeDomain(..) | ResourceDomain(..) | Var(..) =>
                None
        }
    }

    /// Attempt to convert this type into a language_storage::StructTag
    pub fn into_struct_tag(self, env: &GlobalEnv) -> Option<StructTag> {
        self.into_struct_type(env)?.into_struct_tag()
    }

    /// Attempt to convert this type into a language_storage::TypeTag
    pub fn into_type_tag(self, env: &GlobalEnv) -> Option<TypeTag> {
        self.into_normalized_type(env)?.into_type_tag()
    }

    /// Create a `Type` from `t`
    pub fn from_type_tag(t: &TypeTag, env: &GlobalEnv) -> Self {
        use Type::*;
        match t {
            TypeTag::Bool => Primitive(PrimitiveType::Bool),
            TypeTag::U8 => Primitive(PrimitiveType::U8),
            TypeTag::U16 => Primitive(PrimitiveType::U8),
            TypeTag::U32 => Primitive(PrimitiveType::U8),
            TypeTag::U64 => Primitive(PrimitiveType::U64),
            TypeTag::U128 => Primitive(PrimitiveType::U128),
            TypeTag::U256 => Primitive(PrimitiveType::U8),
            TypeTag::Address => Primitive(PrimitiveType::Address),
            TypeTag::Signer => Primitive(PrimitiveType::Signer),
            TypeTag::Struct(s) => {
                let qid = env.find_struct_by_tag(s).unwrap_or_else(|| {
                    panic!("Invariant violation: couldn't resolve struct {:?}", s)
                });
                let type_args = s
                    .type_params
                    .iter()
                    .map(|arg| Self::from_type_tag(arg, env))
                    .collect();
                Struct(qid.module_id, qid.id, type_args)
            },
            TypeTag::Vector(type_param) => Vector(Box::new(Self::from_type_tag(type_param, env))),
        }
    }

    /// Get the unbound type variables in the type.
    pub fn get_vars(&self) -> BTreeSet<u32> {
        let mut vars = BTreeSet::new();
        self.internal_get_vars(&mut vars);
        vars
    }

    fn internal_get_vars(&self, vars: &mut BTreeSet<u32>) {
        use Type::*;
        match self {
            Var(id) => {
                vars.insert(*id);
            },
            Tuple(ts) => ts.iter().for_each(|t| t.internal_get_vars(vars)),
            Fun(a, r) => {
                a.internal_get_vars(vars);
                r.internal_get_vars(vars);
            },
            Struct(_, _, ts) => ts.iter().for_each(|t| t.internal_get_vars(vars)),
            Vector(et) => et.internal_get_vars(vars),
            Reference(_, bt) => bt.internal_get_vars(vars),
            TypeDomain(bt) => bt.internal_get_vars(vars),
            Error | Primitive(..) | TypeParameter(..) | ResourceDomain(..) => {},
        }
    }

    pub fn visit<F: FnMut(&Type)>(&self, visitor: &mut F) {
        let visit_slice = |s: &[Type], visitor: &mut F| {
            for ty in s {
                ty.visit(visitor);
            }
        };
        match self {
            Type::Tuple(tys) => visit_slice(tys, visitor),
            Type::Vector(bt) => bt.visit(visitor),
            Type::Struct(_, _, tys) => visit_slice(tys, visitor),
            Type::Reference(_, ty) => ty.visit(visitor),
            Type::Fun(a, ty) => {
                a.visit(visitor);
                ty.visit(visitor);
            },
            Type::TypeDomain(bt) => bt.visit(visitor),
            _ => {},
        }
        visitor(self)
    }

    /// If this is a tuple, return its elements, otherwise a vector with the given type.
    pub fn flatten(self) -> Vec<Type> {
        if let Type::Tuple(tys) = self {
            tys
        } else {
            vec![self]
        }
    }

    /// If this is a tuple and it has zero elements (the 'unit' type), return true.
    pub fn is_unit(&self) -> bool {
        matches!(self, Type::Tuple(ts) if ts.is_empty())
    }

    /// If this is a vector of more than one type, make a tuple out of it, otherwise return the
    /// type.
    pub fn tuple(mut tys: Vec<Type>) -> Type {
        if tys.is_empty() || tys.len() > 1 {
            Type::Tuple(tys)
        } else {
            tys.pop().unwrap()
        }
    }
}

/// A parameter for type unification that specifies the type compatibility rules to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variance {
    /// All integer types are compatible, and reference types are eliminated.
    SpecVariance,
    /// Same like `SpecVariance` but only for outermost types. This is useful for preventing
    /// variance for type parameters: e.g. we want `num` and `u64` be substitutable, but
    /// not `vector<num>` and `vector<u64>`.
    ShallowSpecVariance,
    /// Variance used in the impl language fragment. This is currently for adapting mutable to
    /// immutable references.
    ShallowImplVariance,
    /// No variance.
    NoVariance,
}

/// Determines an ordering for unification. Combined with `Variance`, determines in which
/// direction automatic type conversion rules are to be applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WideningOrder {
    /// The left type can be widened to the right one.
    LeftToRight,
    /// The right type can be widened to the left one.
    RightToLeft,
    /// The smallest common type into which both left and right type can be widened.
    Join,
}

impl Variance {
    /// Checks whether specification language variance rules are selected.
    pub fn is_spec_variance(self) -> bool {
        matches!(self, Variance::SpecVariance | Variance::ShallowSpecVariance)
    }

    pub fn is_impl_variance(self) -> bool {
        matches!(self, Variance::ShallowImplVariance)
    }

    /// Constructs the variance to be used for subterms of the current type.
    pub fn sub_variance(self) -> Variance {
        match self {
            Variance::ShallowSpecVariance => Variance::NoVariance,
            Variance::SpecVariance => Variance::SpecVariance,
            Variance::ShallowImplVariance => Variance::NoVariance,
            Variance::NoVariance => Variance::NoVariance,
        }
    }

    /// Makes a selected variance shallow, if possible.
    pub fn shallow(self) -> Self {
        match self {
            Variance::ShallowSpecVariance => Variance::ShallowSpecVariance,
            Variance::SpecVariance => Variance::ShallowSpecVariance,
            Variance::ShallowImplVariance => Variance::ShallowImplVariance,
            Variance::NoVariance => Variance::NoVariance,
        }
    }
}

impl WideningOrder {
    /// Swaps the order, if there is any.
    pub fn swap(self) -> Self {
        match self {
            WideningOrder::LeftToRight => WideningOrder::RightToLeft,
            WideningOrder::RightToLeft => WideningOrder::LeftToRight,
            WideningOrder::Join => WideningOrder::Join,
        }
    }
}

impl Substitution {
    /// Creates a new substitution.
    pub fn new() -> Self {
        Self {
            subs: BTreeMap::new(),
            constraints: BTreeMap::new(),
        }
    }

    /// Add a constraint to the variable.
    pub fn add_constraint(&mut self, var: u32, loc: Loc, order: WideningOrder, c: Constraint) {
        self.constraints
            .entry(var)
            .or_default()
            .push((loc, order, c));
    }

    /// Returns true if this is a free variable without constraints.
    pub fn is_free_var_without_constraints(&self, ty: &Type) -> bool {
        if let Type::Var(idx) = ty {
            self.is_free_var(&Type::Var(*idx))
                && self
                    .constraints
                    .get(idx)
                    .map(|cs| cs.is_empty())
                    .unwrap_or(true)
        } else {
            false
        }
    }

    /// Returns true if the type is a free variable.
    pub fn is_free_var(&self, ty: &Type) -> bool {
        if let Type::Var(idx) = ty {
            !self.subs.contains_key(idx)
        } else {
            false
        }
    }

    /// Binds the type variable. If there are constraints associated with the
    /// variable, those are evaluated, possibly leading into unification
    /// errors.
    pub fn bind(&mut self, var: u32, ty: Type) -> Result<(), TypeUnificationError> {
        // Specialize the type before binding, to maximize groundness of type terms.
        let ty = self.specialize(&ty);
        if let Some(constrs) = self.constraints.remove(&var) {
            for (loc, order, c) in constrs {
                self.eval_constraint(loc, &ty, order, c)?
            }
        }
        self.subs.insert(var, ty);
        Ok(())
    }

    /// Evaluates whether the given type satisfies the constraint, discharging the constraint.
    /// Notice that discharging is possible since we expect the type to be fully specialized.
    /// For variables, we just transfer the constraint. For other types, since constraints
    /// are over shallow structure of types, they can be decided.
    pub fn eval_constraint(
        &mut self,
        loc: Loc,
        ty: &Type,
        order: WideningOrder,
        c: Constraint,
    ) -> Result<(), TypeUnificationError> {
        if matches!(ty, Type::Error) {
            Ok(())
        } else if let Type::Var(other_var) = ty {
            // Transfer constraint on to other variable, which we assert to be free
            assert!(!self.subs.contains_key(other_var));
            self.add_constraint(*other_var, loc, order, c);
            Ok(())
        } else {
            match &c {
                Constraint::SomeNumber(options) => match ty {
                    Type::Primitive(prim) if options.contains(prim) => Ok(()),
                    _ => Err(TypeUnificationError::ConstraintUnsatisfied(
                        loc,
                        ty.clone(),
                        order,
                        c,
                    )),
                },
                Constraint::SomeReference(inner_type) => match ty {
                    Type::Reference(_, target_type) => {
                        match self.unify(Variance::NoVariance, order, target_type, inner_type) {
                            Ok(_) => Ok(()),
                            Err(e) => Err(TypeUnificationError::RedirectedError(loc, Box::new(e))),
                        }
                    },
                    _ => Err(TypeUnificationError::ConstraintUnsatisfied(
                        loc,
                        ty.clone(),
                        order,
                        c,
                    )),
                },
                Constraint::WithDefault(_) => Ok(()),
            }
        }
    }

    /// Specializes the type, substituting all variables bound in this substitution.
    pub fn specialize(&self, t: &Type) -> Type {
        t.replace(None, Some(self), false)
    }

    /// Similar like `specialize`, but if a variable is not resolvable but has constraints,
    /// attempts to derive a default from the constraints. For instance, a `SomeNumber(..u64..)`
    /// constraint can default to `u64`.
    pub fn specialize_with_defaults(&self, t: &Type) -> Type {
        t.replace(None, Some(self), true)
    }

    /// Checks whether the type is a number, considering constraints.
    pub fn is_some_number(&self, t: &Type) -> bool {
        if t.is_number() {
            return true;
        }
        if let Type::Var(idx) = t {
            if let Some(constrs) = self.constraints.get(idx) {
                if constrs
                    .iter()
                    .any(|(_, _, c)| matches!(c, Constraint::SomeNumber(_)))
                {
                    return true;
                }
            }
        }
        false
    }

    /// Return either a shallow or deep substitution of the type variable.
    ///
    /// If deep substitution is requested, follow down the substitution chain until either
    /// - `Some(ty)` when the final type is not a type variable or
    /// - `None` when the final type variable does not have a substitution
    pub fn get_substitution(&self, var: u32, shallow: bool) -> Option<Type> {
        match self.subs.get(&var) {
            None => None,
            Some(Type::Var(next_var)) => {
                if shallow {
                    Some(Type::Var(*next_var))
                } else {
                    self.get_substitution(*next_var, false)
                }
            },
            Some(subst_ty) => Some(subst_ty.clone()),
        }
    }

    /// Unify two types, returning the unified type.
    ///
    /// This currently implements the following notion of type compatibility:
    ///
    /// - 1) References are dropped (i.e. &T and T are compatible)
    /// - 2) All integer types are compatible if spec-variance is allowed.
    /// - 3) With the joint effect of 1) and 2), if (P, Q) is compatible under spec-variance,
    ///      (&P, Q), (P, &Q), and (&P, &Q) are all compatible under co-variance.
    /// - 4) If in two tuples (P1, P2, ..., Pn) and (Q1, Q2, ..., Qn), all (Pi, Qi) pairs are
    ///      compatible under spec-variance, then the two tuples are compatible under
    ///      spec-variance.
    ///
    /// The substitution will be refined by variable assignments as needed to perform
    /// unification. If unification fails, the substitution will be in some intermediate state;
    /// to implement transactional unification, the substitution must be cloned before calling
    /// this.
    pub fn unify(
        &mut self,
        variance: Variance,
        order: WideningOrder,
        t1: &Type,
        t2: &Type,
    ) -> Result<Type, TypeUnificationError> {
        // Derive the variance level for recursion
        let sub_variance = variance.sub_variance();
        // If variance is for specs and any of the arguments is a reference, drop it for
        // unification, but ensure it is put back since we need to maintain this information
        // for later phases.
        if variance.is_spec_variance() {
            if let Type::Reference(kind, bt1) = t1 {
                // Avoid creating nested references.
                let t2 = if let Type::Reference(_, bt2) = t2 {
                    bt2.as_ref()
                } else {
                    t2
                };
                return Ok(Type::Reference(
                    *kind,
                    Box::new(self.unify(sub_variance, order, bt1.as_ref(), t2)?),
                ));
            }
            if let Type::Reference(kind, bt2) = t2 {
                return Ok(Type::Reference(
                    *kind,
                    Box::new(self.unify(sub_variance, order, t1, bt2.as_ref())?),
                ));
            }
        }

        // Substitute or assign variables.
        if let Some(rt) = self.try_substitute_or_assign(variance, order, t1, t2)? {
            return Ok(rt);
        }
        if let Some(rt) = self.try_substitute_or_assign(variance, order.swap(), t2, t1)? {
            return Ok(rt);
        }

        // Accept any error type.
        if matches!(t1, Type::Error) {
            return Ok(t2.clone());
        }
        if matches!(t2, Type::Error) {
            return Ok(t1.clone());
        }

        // Unify matching structured types.
        match (t1, t2) {
            (Type::Primitive(p1), Type::Primitive(p2)) => {
                if p1 == p2 {
                    return Ok(t1.clone());
                }
                // All integer types are compatible if spec-variance is allowed.
                if variance.is_spec_variance() && t1.is_number() && t2.is_number() {
                    return Ok(Type::Primitive(PrimitiveType::Num));
                }
            },
            (Type::TypeParameter(idx1), Type::TypeParameter(idx2)) => {
                if idx1 == idx2 {
                    return Ok(t1.clone());
                }
            },
            (Type::Reference(k1, ty1), Type::Reference(k2, ty2)) => {
                let ty = self.unify(sub_variance, order, ty1, ty2)?;
                let k = if variance.is_impl_variance() {
                    use ReferenceKind::*;
                    use WideningOrder::*;
                    match (k1, k2, order) {
                        (Immutable, Immutable, _) | (Mutable, Mutable, _) => k1,
                        (Immutable, Mutable, RightToLeft | Join) => k1,
                        (Mutable, Immutable, LeftToRight | Join) => k2,
                        _ => {
                            let (kl, kr) = if order == RightToLeft {
                                (k1, k2)
                            } else {
                                (k2, k1)
                            };
                            return Err(TypeUnificationError::MutabilityMismatch(*kl, *kr));
                        },
                    }
                } else if *k1 != *k2 {
                    return Err(TypeUnificationError::MutabilityMismatch(*k1, *k2));
                } else {
                    k1
                };
                return Ok(Type::Reference(*k, Box::new(ty)));
            },
            (Type::Tuple(ts1), Type::Tuple(ts2)) => {
                return Ok(Type::Tuple(self.unify_vec(
                    // Note for tuples, we pass on `variance` not `sub_variance`. A shallow
                    // variance type will be effective for the elements of tuples,
                    // which are treated similar as expression lists in function calls.
                    variance, order, ts1, ts2, "tuples",
                )?));
            },
            (Type::Fun(a1, r1), Type::Fun(a2, r2)) => {
                return Ok(Type::Fun(
                    Box::new(self.unify(sub_variance, order, a1, a2)?),
                    Box::new(self.unify(sub_variance, order, r1, r2)?),
                ));
            },
            (Type::Struct(m1, s1, ts1), Type::Struct(m2, s2, ts2)) => {
                if m1 == m2 && s1 == s2 {
                    return Ok(Type::Struct(
                        *m1,
                        *s1,
                        self.unify_vec(sub_variance, order, ts1, ts2, "structs")?,
                    ));
                }
            },
            (Type::Vector(e1), Type::Vector(e2)) => {
                return Ok(Type::Vector(Box::new(self.unify(
                    sub_variance,
                    order,
                    e1,
                    e2,
                )?)));
            },
            (Type::TypeDomain(e1), Type::TypeDomain(e2)) => {
                return Ok(Type::TypeDomain(Box::new(self.unify(
                    sub_variance,
                    order,
                    e1,
                    e2,
                )?)));
            },
            _ => {},
        }
        match order {
            WideningOrder::LeftToRight | WideningOrder::Join => Err(
                TypeUnificationError::TypeMismatch(self.specialize(t1), self.specialize(t2)),
            ),
            WideningOrder::RightToLeft => Err(TypeUnificationError::TypeMismatch(
                self.specialize(t2),
                self.specialize(t1),
            )),
        }
    }

    /// Helper to unify two type vectors.
    fn unify_vec(
        &mut self,
        variance: Variance,
        order: WideningOrder,
        ts1: &[Type],
        ts2: &[Type],
        item_name: &str,
    ) -> Result<Vec<Type>, TypeUnificationError> {
        if ts1.len() != ts2.len() {
            return Err(TypeUnificationError::ArityMismatch(
                item_name.to_owned(),
                ts1.len(),
                ts2.len(),
            ));
        }
        let mut rs = vec![];
        for i in 0..ts1.len() {
            rs.push(self.unify(variance, order, &ts1[i], &ts2[i])?);
        }
        Ok(rs)
    }

    /// Tries to substitute or assign a variable. Returned option is Some if unification
    /// was performed, None if not.
    fn try_substitute_or_assign(
        &mut self,
        variance: Variance,
        order: WideningOrder,
        t1: &Type,
        t2: &Type,
    ) -> Result<Option<Type>, TypeUnificationError> {
        if let Type::Var(v1) = t1 {
            if let Some(s1) = self.subs.get(v1).cloned() {
                return Ok(Some(self.unify(variance, order, &s1, t2)?));
            }
            // Be sure to skip any top-level var assignments for t2, for
            // cycle check.
            let mut t2 = t2.clone();
            while let Type::Var(v2) = &t2 {
                if let Some(s2) = self.subs.get(v2) {
                    t2 = s2.clone()
                } else {
                    break;
                }
            }
            // Skip the cycle check if we are unifying the same two variables.
            if t1 == &t2 {
                return Ok(Some(t1.clone()));
            }
            // Cycle check.
            if !self.occurs_check(&t2, *v1) {
                self.bind(*v1, t2.clone())?;
                Ok(Some(t2))
            } else {
                Err(TypeUnificationError::CyclicSubstitution(
                    self.specialize(t1),
                    self.specialize(&t2),
                ))
            }
        } else {
            Ok(None)
        }
    }

    /// Check whether the variables occurs in the type, or in any assignment to variables in the
    /// type.
    fn occurs_check(&self, ty: &Type, var: u32) -> bool {
        ty.get_vars().iter().any(|v| {
            if v == &var {
                return true;
            }
            if let Some(sty) = self.subs.get(v) {
                self.occurs_check(sty, var)
            } else {
                false
            }
        })
    }
}

impl Default for Substitution {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to unify types which stem from different generic contexts.
///
/// Both comparison side may have type parameters (equally named as #0, #1, ...).
/// The helper converts the type parameter from or both sides into variables
/// and then performs unification of the terms. The resulting substitution
/// is converted back to parameter instantiations.
///
/// Example: consider a function `f<X>` which uses memory `M<X, u64>`, and invariant
/// `invariant<X>` which uses memory `M<bool, X>`. Using this helper to unify both
/// memories will result in instantiations which when applied create `f<bool>`
/// and `invariant<u64>` respectively.
pub struct TypeUnificationAdapter {
    type_vars_map: BTreeMap<u32, (bool, TypeParameterIndex)>,
    types_adapted_lhs: Vec<Type>,
    types_adapted_rhs: Vec<Type>,
}

impl TypeUnificationAdapter {
    /// Initialize the context for the type unifier.
    ///
    /// If `treat_lhs_type_param_as_var_after_index` is set to P,
    /// - any type parameter on the LHS with index < P will be treated as concrete types and
    /// - only type parameters on the LHS with index >= P are treated as variables and thus,
    ///   participate in the type unification process.
    /// The same rule applies to the RHS parameters via `treat_rhs_type_param_as_var_after_index`.
    fn new<'a, I>(
        lhs_types: I,
        rhs_types: I,
        treat_lhs_type_param_as_var_after_index: Option<TypeParameterIndex>,
        treat_rhs_type_param_as_var_after_index: Option<TypeParameterIndex>,
    ) -> Self
    where
        I: Iterator<Item = &'a Type> + Clone,
    {
        debug_assert!(
            treat_lhs_type_param_as_var_after_index.is_some()
                || treat_rhs_type_param_as_var_after_index.is_some(),
            "At least one side of the unification must be treated as variable"
        );

        // Check the input types do not contain type variables.
        debug_assert!(
            lhs_types.clone().chain(rhs_types.clone()).all(|ty| {
                let mut b = true;
                ty.visit(&mut |t| b = b && !matches!(t, Type::Var(_)));
                b
            }),
            "unexpected type variable"
        );

        // Compute the number of type parameters for each side.
        let mut lhs_type_param_count = 0;
        let mut rhs_type_param_count = 0;
        let count_type_param = |t: &Type, current: &mut u16| {
            if let Type::TypeParameter(idx) = t {
                *current = (*current).max(*idx + 1);
            }
        };
        for ty in lhs_types.clone() {
            ty.visit(&mut |t| count_type_param(t, &mut lhs_type_param_count));
        }
        for ty in rhs_types.clone() {
            ty.visit(&mut |t| count_type_param(t, &mut rhs_type_param_count));
        }

        // Create a type variable instantiation for each side.
        let mut var_count = 0;
        let mut type_vars_map = BTreeMap::new();
        let lhs_inst = match treat_lhs_type_param_as_var_after_index {
            None => vec![],
            Some(boundary) => (0..boundary)
                .map(Type::TypeParameter)
                .chain((boundary..lhs_type_param_count).map(|i| {
                    let idx = var_count;
                    var_count += 1;
                    type_vars_map.insert(idx, (true, i));
                    Type::Var(idx)
                }))
                .collect(),
        };
        let rhs_inst = match treat_rhs_type_param_as_var_after_index {
            None => vec![],
            Some(boundary) => (0..boundary)
                .map(Type::TypeParameter)
                .chain((boundary..rhs_type_param_count).map(|i| {
                    let idx = var_count;
                    var_count += 1;
                    type_vars_map.insert(idx, (false, i));
                    Type::Var(idx)
                }))
                .collect(),
        };

        // Do the adaptation.
        let types_adapted_lhs = lhs_types.map(|t| t.instantiate(&lhs_inst)).collect();
        let types_adapted_rhs = rhs_types.map(|t| t.instantiate(&rhs_inst)).collect();

        Self {
            type_vars_map,
            types_adapted_lhs,
            types_adapted_rhs,
        }
    }

    /// Create a TypeUnificationAdapter with the goal of unifying a pair of types.
    ///
    /// If `treat_lhs_type_param_as_var` is True, treat all type parameters on the LHS as variables.
    /// If `treat_rhs_type_param_as_var` is True, treat all type parameters on the RHS as variables.
    pub fn new_pair(
        lhs_type: &Type,
        rhs_type: &Type,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
    ) -> Self {
        Self::new(
            std::iter::once(lhs_type),
            std::iter::once(rhs_type),
            treat_lhs_type_param_as_var.then_some(0),
            treat_rhs_type_param_as_var.then_some(0),
        )
    }

    /// Create a TypeUnificationAdapter with the goal of unifying a pair of type tuples.
    ///
    /// If `treat_lhs_type_param_as_var` is True, treat all type parameters on the LHS as variables.
    /// If `treat_rhs_type_param_as_var` is True, treat all type parameters on the RHS as variables.
    pub fn new_vec(
        lhs_types: &[Type],
        rhs_types: &[Type],
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
    ) -> Self {
        Self::new(
            lhs_types.iter(),
            rhs_types.iter(),
            treat_lhs_type_param_as_var.then_some(0),
            treat_rhs_type_param_as_var.then_some(0),
        )
    }

    /// Consume the TypeUnificationAdapter and produce the unification result. If type unification
    /// is successful, return a pair of instantiations for type parameters on each side which
    /// unify the LHS and RHS respectively. If the LHS and RHS cannot unify, None is returned.
    pub fn unify(
        self,
        variance: Variance,
        shallow_subst: bool,
    ) -> Option<(BTreeMap<u16, Type>, BTreeMap<u16, Type>)> {
        let mut subst = Substitution::new();
        match subst.unify_vec(
            variance,
            WideningOrder::LeftToRight,
            &self.types_adapted_lhs,
            &self.types_adapted_rhs,
            "",
        ) {
            Ok(_) => {
                let mut inst_lhs = BTreeMap::new();
                let mut inst_rhs = BTreeMap::new();
                for (var_idx, (is_lhs, param_idx)) in &self.type_vars_map {
                    let subst_ty = match subst.get_substitution(*var_idx, shallow_subst) {
                        None => continue,
                        Some(Type::Var(subst_var_idx)) => {
                            match self.type_vars_map.get(&subst_var_idx) {
                                None => {
                                    // If the original types do not contain free type
                                    // variables, this should not happen.
                                    panic!("unexpected type variable");
                                },
                                Some((_, subs_param_idx)) => {
                                    // There can be either lhs or rhs type parameters left, but
                                    // not both sides, so it is unambiguous to just return it here.
                                    Type::TypeParameter(*subs_param_idx)
                                },
                            }
                        },
                        Some(subst_ty) => subst_ty.clone(),
                    };
                    let inst = if *is_lhs {
                        &mut inst_lhs
                    } else {
                        &mut inst_rhs
                    };
                    inst.insert(*param_idx, subst_ty);
                }

                Some((inst_lhs, inst_rhs))
            },
            Err(_) => None,
        }
    }
}

impl TypeUnificationError {
    /// If this error is associated with a specific location, return this.
    pub fn specific_loc(&self) -> Option<Loc> {
        match self {
            TypeUnificationError::ConstraintUnsatisfied(loc, ..)
            | TypeUnificationError::RedirectedError(loc, ..) => Some(loc.clone()),
            _ => None,
        }
    }

    /// Return the message for this error.
    pub fn message(&self, display_context: &TypeDisplayContext) -> String {
        match self {
            TypeUnificationError::TypeMismatch(t1, t2) => {
                format!(
                    "expected `{}` but found `{}`",
                    t2.display(display_context),
                    t1.display(display_context),
                )
            },
            TypeUnificationError::ArityMismatch(item, a1, a2) => {
                format!("{} have different arity ({} != {})", item, a1, a2)
            },
            TypeUnificationError::CyclicSubstitution(t1, t2) => {
                format!(
                    "type unification cycle check failed (`{} =?= {}`, try to annotate type)",
                    t1.display(display_context),
                    t2.display(display_context),
                )
            },
            TypeUnificationError::MutabilityMismatch(k1, k2) => {
                let pr = |k: ReferenceKind| match k {
                    ReferenceKind::Immutable => "&",
                    ReferenceKind::Mutable => "&mut",
                };
                format!("mutability mismatch ({} != {})", pr(*k1), pr(*k2))
            },
            TypeUnificationError::ConstraintUnsatisfied(_, ty, order, constr) => match constr {
                Constraint::SomeNumber(options) => {
                    let all_ints = PrimitiveType::all_int_types()
                        .into_iter()
                        .collect::<BTreeSet<_>>();
                    let options_str = if options == &all_ints {
                        "integer".to_owned()
                    } else {
                        options
                            .iter()
                            .map(|p| Type::new_prim(*p).display(display_context).to_string())
                            .join("|")
                    };
                    let type_str = ty.display(display_context).to_string();
                    let (expected, actual) = match order {
                        WideningOrder::Join | WideningOrder::LeftToRight => (type_str, options_str),
                        WideningOrder::RightToLeft => (options_str, type_str),
                    };
                    format!("expected `{}` but found `{}`", expected, actual)
                },
                Constraint::SomeReference(_) => {
                    format!(
                        "expected `{}` to be a reference",
                        ty.display(display_context)
                    )
                },
                Constraint::WithDefault(_) => unreachable!("default constraint in error message"),
            },
            TypeUnificationError::RedirectedError(_, err) => err.message(display_context),
        }
    }
}

/// A helper to derive the set of instantiations for type parameters
pub struct TypeInstantiationDerivation {}

impl TypeInstantiationDerivation {
    /// Find what the instantiations should we have for the type parameter at `target_param_index`.
    ///
    /// The invariant is, forall type parameters whose index < target_param_index, it should either
    /// - be assigned with a concrete type already and hence, ceases to be a type parameter, or
    /// - does not have any matching instantiation and hence, either remains a type parameter or is
    ///   represented as a type error.
    /// But in anyway, these type parameters no longer participate in type unification anymore.
    ///
    /// If `target_lhs` is True, derive instantiations for the type parameter with
    /// `target_param_index` on the `lhs_types`. Otherwise, target the `rhs_types`.
    fn derive_instantiations_for_target_parameter(
        lhs_types: &BTreeSet<Type>,
        rhs_types: &BTreeSet<Type>,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
        target_param_index: TypeParameterIndex,
        target_lhs: bool,
    ) -> BTreeSet<Type> {
        // progressively increase the boundary
        let treat_lhs_type_param_as_var_after_index =
            treat_lhs_type_param_as_var.then_some(if target_lhs { target_param_index } else { 0 });
        let treat_rhs_type_param_as_var_after_index =
            treat_rhs_type_param_as_var.then_some(if target_lhs { 0 } else { target_param_index });

        let mut target_param_insts = BTreeSet::new();
        for t_lhs in lhs_types {
            for t_rhs in rhs_types {
                // Try to unify the instantiations
                let adapter = TypeUnificationAdapter::new(
                    std::iter::once(t_lhs),
                    std::iter::once(t_rhs),
                    treat_lhs_type_param_as_var_after_index,
                    treat_rhs_type_param_as_var_after_index,
                );
                let rel = adapter.unify(Variance::SpecVariance, false);
                if let Some((subst_lhs, subst_rhs)) = rel {
                    let subst = if target_lhs { subst_lhs } else { subst_rhs };
                    for (param_idx, inst_ty) in subst.into_iter() {
                        if param_idx != target_param_index {
                            // this parameter will be unified at a later stage.
                            //
                            // NOTE: this code is inefficient when we have multiple type parameters,
                            // but a vast majority of Move code we see so far have at most one type
                            // parameter, so we trade-off efficiency with simplicity in code.
                            assert!(param_idx > target_param_index);
                            continue;
                        }
                        target_param_insts.insert(inst_ty);
                    }
                }
            }
        }
        target_param_insts
    }

    /// Find the set of valid instantiation combinations for all the type parameters.
    ///
    /// The algorithm is progressive. For a list of parameters with arity `params_arity = N`, it
    /// - first finds all possible instantiation for parameter at index 0 (`inst_param_0`) and,'
    /// - for each instantiation in `inst_param_0`,
    ///   - refines LHS or RHS types and
    ///   - finds all possible instantiations for parameter at index 1 (`inst_param_1`)
    ///   - for each instantiation in `inst_param_1`,
    ///     - refines LHS or RHS types and
    ///     - finds all possible instantiations for parameter at index 2 (`inst_param_2`)
    ///     - for each instantiation in `inst_param_2`,
    ///       - ......
    /// The process continues until all type parameters are analyzed (i.e., reaching the type
    /// parameter at index `N`).
    ///
    /// If `refine_lhs` is True, refine the `lhs_types` after each round; same for `refine_rhs`.
    ///
    /// If `target_lhs` is True, find instantiations for the type parameters in the `lhs_types`,
    /// otherwise, target the `rhs_types`.
    ///
    /// If `mark_irrelevant_param_as_error` is True, type parameters that do not have any valid
    /// instantiation will be marked as `Type::Error`. Otherwise, leave the type parameter as it is.
    pub fn progressive_instantiation<'a, I>(
        lhs_types: I,
        rhs_types: I,
        treat_lhs_type_param_as_var: bool,
        treat_rhs_type_param_as_var: bool,
        refine_lhs: bool,
        refine_rhs: bool,
        params_arity: usize,
        target_lhs: bool,
        mark_irrelevant_param_as_error: bool,
    ) -> BTreeSet<Vec<Type>>
    where
        I: Iterator<Item = &'a Type> + Clone,
    {
        let initial_param_insts: Vec<_> = (0..params_arity).map(Type::new_param).collect();

        let mut work_queue = VecDeque::new();
        work_queue.push_back(initial_param_insts);
        for target_param_index in 0..params_arity {
            let mut for_next_round = vec![];
            while let Some(param_insts) = work_queue.pop_front() {
                // refine the memory usage sets with current param instantiations
                let refined_lhs = lhs_types
                    .clone()
                    .map(|t| {
                        if refine_lhs {
                            t.instantiate(&param_insts)
                        } else {
                            t.clone()
                        }
                    })
                    .collect();
                let refined_rhs = rhs_types
                    .clone()
                    .map(|t| {
                        if refine_rhs {
                            t.instantiate(&param_insts)
                        } else {
                            t.clone()
                        }
                    })
                    .collect();

                // find type instantiations for the target parameter index
                let mut target_param_insts = Self::derive_instantiations_for_target_parameter(
                    &refined_lhs,
                    &refined_rhs,
                    treat_lhs_type_param_as_var,
                    treat_rhs_type_param_as_var,
                    target_param_index as TypeParameterIndex,
                    target_lhs,
                );

                // decide what to do with an irrelevant type parameter
                if target_param_insts.is_empty() {
                    let irrelevant_type = if mark_irrelevant_param_as_error {
                        Type::Error
                    } else {
                        Type::new_param(target_param_index)
                    };
                    target_param_insts.insert(irrelevant_type);
                }

                // instantiate the target type parameter in every possible way
                for inst in target_param_insts {
                    let mut next_insts = param_insts.clone();
                    next_insts[target_param_index] = inst;
                    for_next_round.push(next_insts);
                }
            }
            work_queue.extend(for_next_round);
        }

        // the final workqueue contains possible instantiations for all type parameters
        work_queue.into_iter().collect()
    }
}

/// Data providing context for displaying types.
#[derive(Clone)]
pub struct TypeDisplayContext<'a> {
    pub env: &'a GlobalEnv,
    pub type_param_names: Option<Vec<Symbol>>,
    /// An optional substitution used for display
    pub subs_opt: Option<&'a Substitution>,
    /// During type checking, the env might not contain the types yet of the currently checked
    /// module. This field allows to access symbolic information in this case.
    pub builder_struct_table: Option<&'a BTreeMap<(ModuleId, StructId), QualifiedSymbol>>,
}

impl<'a> TypeDisplayContext<'a> {
    pub fn new(env: &'a GlobalEnv) -> TypeDisplayContext<'a> {
        Self {
            env,
            type_param_names: None,
            subs_opt: None,
            builder_struct_table: None,
        }
    }

    pub fn new_with_params(
        env: &'a GlobalEnv,
        type_param_names: Vec<Symbol>,
    ) -> TypeDisplayContext<'a> {
        Self {
            env,
            subs_opt: None,
            type_param_names: Some(type_param_names),
            builder_struct_table: None,
        }
    }

    pub fn add_subs(self, subs: &'a Substitution) -> Self {
        Self {
            subs_opt: Some(subs),
            ..self
        }
    }
}

/// Helper for type displays.
pub struct TypeDisplay<'a> {
    type_: &'a Type,
    context: &'a TypeDisplayContext<'a>,
}

impl Type {
    pub fn display<'a>(&'a self, context: &'a TypeDisplayContext<'a>) -> TypeDisplay<'a> {
        TypeDisplay {
            type_: self,
            context,
        }
    }
}

impl<'a> fmt::Display for TypeDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Type::*;
        let comma_list = |f: &mut Formatter<'_>, ts: &[Type]| -> fmt::Result {
            let mut first = true;
            for t in ts {
                if first {
                    first = false
                } else {
                    f.write_str(", ")?;
                }
                write!(f, "{}", t.display(self.context))?;
            }
            Ok(())
        };
        match self.type_ {
            Primitive(p) => write!(f, "{}", p),
            Tuple(ts) => {
                f.write_str("(")?;
                comma_list(f, ts)?;
                f.write_str(")")
            },
            Vector(t) => write!(f, "vector<{}>", t.display(self.context)),
            TypeDomain(t) => write!(f, "domain<{}>", t.display(self.context)),
            ResourceDomain(mid, sid, inst_opt) => {
                write!(f, "resources<{}", self.struct_str(*mid, *sid))?;
                if let Some(inst) = inst_opt {
                    f.write_str("<")?;
                    comma_list(f, inst)?;
                    f.write_str(">")?;
                }
                f.write_str(">")
            },
            Fun(a, t) => {
                f.write_str("|")?;
                write!(f, "{}", a.display(self.context))?;
                f.write_str("|")?;
                write!(f, "{}", t.display(self.context))
            },
            Struct(mid, sid, ts) => {
                write!(f, "{}", self.struct_str(*mid, *sid))?;
                if !ts.is_empty() {
                    f.write_str("<")?;
                    comma_list(f, ts)?;
                    f.write_str(">")?;
                }
                Ok(())
            },
            Reference(kind, t) => {
                f.write_str("&")?;
                let modifier = match kind {
                    ReferenceKind::Immutable => "",
                    ReferenceKind::Mutable => "mut ",
                };
                f.write_str(modifier)?;
                write!(f, "{}", t.display(self.context))
            },
            TypeParameter(idx) => {
                if let Some(names) = &self.context.type_param_names {
                    let idx = *idx as usize;
                    if idx < names.len() {
                        write!(f, "{}", names[idx].display(self.context.env.symbol_pool()))
                    } else {
                        write!(f, "#{}", idx)
                    }
                } else {
                    write!(f, "#{}", idx)
                }
            },
            Var(idx) => {
                if let Some(ty) = self.context.subs_opt.and_then(|s| s.subs.get(idx)) {
                    ty.fmt(f)
                } else if let Some(ctrs) =
                    self.context.subs_opt.and_then(|s| s.constraints.get(idx))
                {
                    let mut out = "".to_owned();
                    for (_, _, c) in ctrs {
                        // We asssume no inconsistent constraints, so break on the first one
                        match c {
                            Constraint::SomeNumber(_) => {
                                out = "integer".to_owned();
                                break;
                            },
                            Constraint::SomeReference(ty) => {
                                out = format!("&_{}", ty.display(self.context));
                                break;
                            },
                            Constraint::WithDefault(ty) => {
                                out = format!("{}/*default*/", ty.display(self.context))
                            },
                        }
                    }
                    if out.is_empty() {
                        out = format!("?{}", idx)
                    }
                    f.write_str(&out)
                } else {
                    write!(f, "?{}", idx)
                }
            },
            Error => f.write_str("*error*"),
        }
    }
}

impl<'a> TypeDisplay<'a> {
    fn struct_str(&self, mid: ModuleId, sid: StructId) -> String {
        let env = self.context.env;
        if let Some(builder_table) = self.context.builder_struct_table {
            let qsym = builder_table.get(&(mid, sid)).expect("type known");
            qsym.display(self.context.env).to_string()
        } else {
            let struct_env = env.get_module(mid).into_struct(sid);
            format!(
                "{}::{}",
                struct_env.module_env.get_name().display(env),
                struct_env.get_name().display(env.symbol_pool())
            )
        }
    }
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use PrimitiveType::*;
        match self {
            Bool => f.write_str("bool"),
            U8 => f.write_str("u8"),
            U16 => f.write_str("u16"),
            U32 => f.write_str("u32"),
            U64 => f.write_str("u64"),
            U128 => f.write_str("u128"),
            U256 => f.write_str("u256"),
            Address => f.write_str("address"),
            Signer => f.write_str("signer"),
            Range => f.write_str("range"),
            Num => f.write_str("num"),
            EventStore => f.write_str("estore"),
        }
    }
}
