// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains types and related functions.

use crate::{
    ast::{ModuleName, QualifiedSymbol},
    builder::{ith_str, pluralize},
    model::{
        FunId, GlobalEnv, Loc, ModuleId, QualifiedId, QualifiedInstId, StructEnv, StructId,
        TypeParameter, TypeParameterKind,
    },
    symbol::Symbol,
};
use itertools::Itertools;
#[allow(deprecated)]
use move_binary_format::normalized::Type as MType;
use move_binary_format::{
    access::ModuleAccess, file_format::SignatureToken, views::StructHandleView, CompiledModule,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    language_storage::{FunctionTag, StructTag, TypeTag},
    u256::U256,
};
use num::BigInt;
use num_traits::identities::Zero;
use std::{
    cmp::Ordering,
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    fmt,
    fmt::{Debug, Formatter},
    iter,
};

/// Represents a type.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Tuple(Vec<Type>),
    Vector(Box<Type>),
    Struct(ModuleId, StructId, /*type-params*/ Vec<Type>),
    TypeParameter(u16),
    Fun(
        /*args*/ Box<Type>,
        /*result*/ Box<Type>,
        AbilitySet,
    ),

    // Types only appearing in programs.
    Reference(ReferenceKind, Box<Type>),

    // Types only appearing in specifications.
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

impl fmt::Display for ReferenceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceKind::Immutable => f.write_str("`&`"),
            ReferenceKind::Mutable => f.write_str("`&mut`"),
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
#[derive(Debug, Clone, Default)]
pub struct Substitution {
    /// Assignment of types to variables.
    subs: BTreeMap<u32, Type>,
    /// Constraints on (unassigned) variables.
    constraints: BTreeMap<u32, Vec<(Loc, WideningOrder, Constraint)>>,
    /// Contexts for the constraints, used in error reporting.
    constraint_contexts: BTreeMap<u32, ConstraintContext>,
    /// Constraints which have been reported to be unsatisfied, by type. By
    /// collecting those, we avoid followup errors in constraint
    /// evaluation.
    reported: BTreeMap<Type, BTreeSet<Constraint>>,
}

/// A constraint on a type variable, maintained during unification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Constraint {
    /// The type variable must be instantiated with one of the given number types. This is used
    /// for representing integer constants.
    SomeNumber(BTreeSet<PrimitiveType>),
    /// The type variable must be instantiated with a reference of given type.
    SomeReference(Type),
    /// The type variable must be instantiated with a struct which has the given fields with
    /// types.
    SomeStruct(BTreeMap<Symbol, Type>),
    /// The type variable must be instantiated with a type for which a receiver function with the given
    /// signature exists: the name, the optional type arguments, the argument types, and the
    /// result type.
    SomeReceiverFunction(
        Symbol,
        /// The optional type arguments, with locations
        Option<(Vec<Loc>, Vec<Type>)>,
        /// The locations of the arguments
        Vec<Loc>,
        /// The argument type
        Vec<Type>,
        /// The result type
        Type,
    ),
    /// The type variable must be instantiated with a function value with the given argument
    /// and result type. This is used to represent function types for which the ability set
    /// is unknown.
    SomeFunctionValue(
        /// The argument type. This is contra-variant.
        Type,
        /// The result type. This is co-variant.
        Type,
    ),
    /// The type must not be reference because it is used as the type of some field or
    /// as a type argument.
    NoReference,
    /// The type must not be tuple because it is used as the type of some field or
    /// as a type argument.
    NoTuple,
    /// The type must not be a phantom type. A phantom type is only allowed
    /// as a type argument for a phantom type parameter.
    NoPhantom,
    /// The type must have the given set of abilities.
    HasAbilities(AbilitySet, AbilityCheckingScope),
    /// The type variable defaults to the given type if no other binding is found. This is
    /// a pseudo constraint which never fails, but used to generate a default for
    /// inference.
    WithDefault(Type),
}

/// Scope of ability checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbilityCheckingScope {
    /// Type parameters are excluded from ability checking. This is in usages the case
    /// where we check abilities for field types, for example, since those constraints
    /// are modulo an actual type instantiation.
    ExcludeTypeParams,
    /// Type parameters are included in ability checking. This is the case if
    /// we check ability constraints for a type instantiation, as in `S<T>`,
    /// and we have `struct S<X:A>`.
    IncludeTypeParams,
}

/// A type to describe the context from where a constraint stems. Used for
/// error reporting.
#[derive(Debug, Clone, Default)]
pub struct ConstraintContext {
    // The constraint was added to a type which was inferred.
    inferred: bool,
    // The origin of the constraint.
    origin: ConstraintOrigin,
}

/// A type to describe the origin of a constraint. Used for error reporting.
#[derive(Debug, Clone, Default)]
pub enum ConstraintOrigin {
    /// Origin is not further specified.
    #[default]
    Unspecified,
    /// The origin is a local of the given name.
    Local(Symbol),
    /// The origin is a field of the given name.
    Field(Symbol),
    /// The origin is a type parameter instantiation. In
    /// `TypeParameter(parent, is_struct, name, param)`, `parent` is an optional parent
    /// from which this origin is derived, `is_struct` indicates whether the parameter is from
    /// a struct or function, `name` is the name of that struct or function, and
    /// `param` the parameter declaration.
    TypeParameter(
        Option<Box<ConstraintOrigin>>,
        /*is_struct*/ bool,
        Symbol,
        TypeParameter,
    ),
    /// The origin is a vector type parameter instantiation, with an optional parent
    /// from which this origin is derived.
    VectorTypeParameter(Option<Box<ConstraintOrigin>>),
    /// For `TupleElement(parent, i)`, with an optional parent
    /// from which this origin is derived.
    TupleElement(Box<ConstraintOrigin>, usize),
}

impl ConstraintContext {
    /// Creates a context with the property that the related type was inferred.
    pub fn inferred() -> Self {
        Self {
            inferred: true,
            origin: ConstraintOrigin::Unspecified,
        }
    }

    /// Marks a context to be rooted in the given type parameter.
    pub fn for_type_param(self, is_struct: bool, item: Symbol, type_param: TypeParameter) -> Self {
        Self {
            origin: ConstraintOrigin::TypeParameter(None, is_struct, item, type_param),
            ..self
        }
    }

    /// Marks a context to be rooted in a vector type parameter.
    pub fn for_vector_type_param(self) -> Self {
        Self {
            origin: ConstraintOrigin::VectorTypeParameter(None),
            ..self
        }
    }

    /// Marks a context to be rooted in a local.
    pub fn for_local(self, name: Symbol) -> Self {
        Self {
            origin: ConstraintOrigin::Local(name),
            ..self
        }
    }

    /// Marks a context to be rooted in a field.
    pub fn for_field(self, name: Symbol) -> Self {
        Self {
            origin: ConstraintOrigin::Field(name),
            ..self
        }
    }

    /// Makes a derived context for a tuple element.
    pub fn derive_tuple_element(self, idx: usize) -> Self {
        Self {
            origin: ConstraintOrigin::TupleElement(Box::new(self.origin.clone()), idx),
            ..self
        }
    }

    /// Makes a derived context for a vector type parameter
    pub fn derive_vector_type_param(self) -> Self {
        Self {
            origin: ConstraintOrigin::VectorTypeParameter(Some(Box::new(self.origin.clone()))),
            ..self
        }
    }

    /// Makes a derived context for a struct type parameter.
    pub fn derive_struct_parameter(self, name: Symbol, param: TypeParameter) -> Self {
        Self {
            origin: ConstraintOrigin::TypeParameter(
                Some(Box::new(self.origin.clone())),
                true,
                name,
                param,
            ),
            ..self
        }
    }

    /// Creates a description from the context: a note to add to the general error
    /// message, and hints and labels with additional information.
    pub fn describe(
        &self,
        context: &TypeDisplayContext,
    ) -> (String, Vec<String>, Vec<(Loc, String)>) {
        let ConstraintContext { inferred, origin } = self;
        let mut labels = vec![];
        let mut hints = vec![];
        origin.describe(context, &mut hints, &mut labels);
        (
            if *inferred { "type was inferred" } else { "" }.to_string(),
            hints,
            labels,
        )
    }
}

impl ConstraintOrigin {
    /// Creates a description for the context origin, for error messages, in form of
    /// hints and labels for the error diagnosis system.
    fn describe(
        &self,
        context: &TypeDisplayContext,
        hints: &mut Vec<String>,
        labels: &mut Vec<(Loc, String)>,
    ) {
        use self::TypeParameter as TP;
        use ConstraintOrigin::*;
        match self {
            Unspecified => {
                // Do nothing
            },
            Local(name) => hints.push(format!(
                "required by declaration of local `{}`",
                name.display(context.env.symbol_pool())
            )),
            Field(name) => hints.push(format!(
                "required by declaration of field `{}`",
                name.display(context.env.symbol_pool()),
            )),
            TypeParameter(parent, is_struct, item, TP(name, kind, loc)) => {
                let name = name.display(context.env.symbol_pool());
                let phantom_str = if kind.is_phantom { "phantom " } else { "" };
                let abilities_str = if kind.abilities.is_empty() {
                    "".to_string()
                } else {
                    format!(":{}", kind.abilities)
                };

                hints.push(format!(
                    "required by instantiating type parameter `{}{}{}` of {} `{}`",
                    phantom_str,
                    name,
                    abilities_str,
                    if *is_struct { "struct" } else { "function" },
                    item.display(context.env.symbol_pool())
                ));
                if let Some(parent) = parent {
                    parent.describe(context, hints, labels)
                } else {
                    // For the root context, add a label for the type parameter which
                    // defines the constraints, but only if the location is not from
                    // a builtin function. Note it doesn't make sense to add labels
                    // for non-root type parameters because they do not contribute to ability
                    // inference (only type arguments do, not the formal parameters).
                    if loc != &context.env.internal_loc() {
                        labels.push((
                            loc.clone(),
                            format!("declaration of type parameter `{}`", name),
                        ));
                    }
                }
            },
            TupleElement(parent, idx) => {
                hints.push(format!("required by {} tuple element", ith_str(*idx)));
                parent.describe(context, hints, labels)
            },
            VectorTypeParameter(parent) => {
                hints.push("required by instantiating vector type parameter".to_string());
                if let Some(parent) = parent {
                    parent.describe(context, hints, labels)
                }
            },
        }
    }
}

impl Constraint {
    /// Returns the default type of constraint. At the end of type unification, variables
    /// with constraints that have defaults will be substituted by those defaults.
    pub fn default_type(&self) -> Option<Type> {
        match self {
            Constraint::SomeNumber(options) if options.contains(&PrimitiveType::U64) => {
                Some(Type::new_prim(PrimitiveType::U64))
            },
            Constraint::SomeReference(ty) => Some(Type::Reference(
                ReferenceKind::Immutable,
                Box::new(ty.clone()),
            )),
            Constraint::WithDefault(ty) => Some(ty.clone()),
            Constraint::SomeFunctionValue(arg_type, result_type) => {
                // For functions, if there is no requirement from the type context, default
                // to the minimal empty ability set
                Some(Type::function(
                    arg_type.clone(),
                    result_type.clone(),
                    AbilitySet::EMPTY,
                ))
            },
            _ => None,
        }
    }

    /// Determines the default type for a list of constraints. This finds a constraint
    /// which can generate a type, and then checks whether it is compatible with any
    /// other provided constraints.
    pub fn default_type_for<'a>(constrs: impl Iterator<Item = &'a Constraint>) -> Option<Type> {
        let mut result = None;
        let mut abilities = None;
        for ctr in constrs {
            if let Some(ty) = ctr.default_type() {
                result = Some(ty)
            } else {
                match ctr {
                    Constraint::HasAbilities(abs, _) => abilities = Some(*abs),
                    Constraint::NoTuple | Constraint::NoPhantom | Constraint::NoReference => {
                        // Skip, is trivially satisfied for a concrete type
                    },
                    Constraint::SomeNumber(_)
                    | Constraint::SomeReference(_)
                    | Constraint::SomeStruct(_)
                    | Constraint::SomeReceiverFunction(..)
                    | Constraint::SomeFunctionValue(..)
                    | Constraint::WithDefault(_) => {
                        // Incompatible
                        return None;
                    },
                }
            }
        }
        match (abilities, result) {
            (Some(abs), Some(Type::Fun(arg, res, _))) => Some(Type::Fun(arg, res, abs)),
            (Some(abs), Some(Type::Primitive(PrimitiveType::U64)))
                if !abs.has_ability(Ability::Key) =>
            {
                Some(Type::Primitive(PrimitiveType::U64))
            },
            (None, result) => result,
            _ => None,
        }
    }

    /// Returns true if the constraint should be propagated over references, such that if we
    /// have `&t`, the constraint should be forwarded to `t`.
    pub fn propagate_over_reference(&self) -> bool {
        matches!(
            self,
            Constraint::SomeStruct(..) | Constraint::SomeReceiverFunction(..)
        )
    }

    /// Returns true if the constraint should be hidden in displays to user. This is
    /// for internal constraints which would be mostly confusing to users.
    pub fn hidden(&self) -> bool {
        use Constraint::*;
        matches!(self, NoPhantom | NoReference | NoTuple | WithDefault(..))
    }

    /// Returns true if this context is accumulating. When adding a new constraint
    /// to the type substitution, an accumulating constraint can be added to the
    /// existing constraints without creating a conflict. In contrast, a
    /// non-accumulating constraint conflicts with all other constraints besides
    /// itself and accumulating constraints. For example, `HasAbilities` can
    /// co-exist with other constraints, but `SomeNumber` only with
    /// another `SomeNumber` constraint plus accumulating constraints.
    pub fn accumulating(&self) -> bool {
        matches!(
            self,
            Constraint::HasAbilities(..)
                | Constraint::WithDefault(_)
                | Constraint::NoPhantom
                | Constraint::NoTuple
                | Constraint::NoReference
        )
    }

    /// Defines an ordering on constraints to determine which one to
    /// report first on violation. Accumulating constraints are later
    /// in the order as they represent secondary errors.
    pub fn compare(&self, other: &Constraint) -> Ordering {
        if !self.accumulating() && other.accumulating() {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }

    /// Some constraint errors lead to unnecessary noise if reported more than once for
    /// the same type.
    pub fn report_only_once(&self) -> bool {
        use Constraint::*;
        matches!(self, HasAbilities(..) | NoReference | NoPhantom | NoTuple)
    }

    /// Joins the two constraints. If they are incompatible, produces a type unification error.
    /// Otherwise, returns true if `self` absorbs the `other` constraint (and waives the `other`).
    /// ctx_opt is for additional error info
    pub fn join(
        &mut self,
        context: &mut impl UnificationContext,
        subs: &mut Substitution,
        loc: &Loc,
        other: &Constraint,
        ctx_opt: Option<ConstraintContext>,
    ) -> Result<bool, TypeUnificationError> {
        match (&mut *self, other) {
            (Constraint::SomeNumber(opts1), Constraint::SomeNumber(opts2)) => {
                let joined: BTreeSet<PrimitiveType> = opts1.intersection(opts2).cloned().collect();
                if joined.is_empty() {
                    Err(TypeUnificationError::ConstraintsIncompatible(
                        loc.clone(),
                        self.clone(),
                        other.clone(),
                    ))
                } else {
                    *opts1 = joined;
                    Ok(true)
                }
            },
            (Constraint::SomeReference(ty1), Constraint::SomeReference(ty2)) => {
                *ty1 = subs.unify(context, Variance::NoVariance, WideningOrder::Join, ty1, ty2)?;
                Ok(true)
            },
            (Constraint::SomeStruct(fields1), Constraint::SomeStruct(fields2)) => {
                // Join the fields together, unifying their types if there are overlaps.
                for (name, ty) in fields2 {
                    if let Some(old_type) = fields1.insert(*name, ty.clone()) {
                        subs.unify(
                            context,
                            Variance::NoVariance,
                            WideningOrder::Join,
                            &old_type,
                            ty,
                        )?;
                    }
                }
                Ok(true)
            },
            (
                Constraint::SomeReceiverFunction(name1, generics1, _, args1, result1),
                Constraint::SomeReceiverFunction(name2, generics2, _, args2, result2),
            ) => {
                if name1 == name2 {
                    if let (Some(gens1), Some(gens2)) = (generics1, generics2) {
                        subs.unify_vec_maybe_type_args(
                            context,
                            true,
                            Variance::NoVariance,
                            WideningOrder::Join,
                            None,
                            &gens1.1,
                            &gens2.1,
                        )?;
                    }
                    subs.unify_vec(
                        context,
                        Variance::NoVariance,
                        WideningOrder::Join,
                        None,
                        args1,
                        args2,
                    )?;
                    subs.unify(
                        context,
                        Variance::NoVariance,
                        WideningOrder::Join,
                        result1,
                        result2,
                    )?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            (
                Constraint::SomeFunctionValue(arg1, result1),
                Constraint::SomeFunctionValue(arg2, result2),
            ) => {
                subs.unify(
                    context,
                    Variance::NoVariance,
                    WideningOrder::Join,
                    arg1,
                    arg2,
                )?;
                subs.unify(
                    context,
                    Variance::NoVariance,
                    WideningOrder::Join,
                    result1,
                    result2,
                )?;
                Ok(true)
            },
            (Constraint::NoReference, Constraint::NoReference) => Ok(true),
            (Constraint::NoTuple, Constraint::NoTuple) => Ok(true),
            (Constraint::NoPhantom, Constraint::NoPhantom) => Ok(true),
            (Constraint::HasAbilities(a1, scope1), Constraint::HasAbilities(a2, scope2))
                if scope1 == scope2 =>
            {
                *a1 = a1.union(*a2);
                Ok(true)
            },
            // After the above checks on same type of constraint
            // Check compatibility between ability and number
            // This check is needed because sometime the concrete integer type is not available
            // TODO: check other combination of constraints may be necessary as well.
            (Constraint::HasAbilities(a1, _), Constraint::SomeNumber(_)) => {
                let unsupported_abilities = a1.setminus(AbilitySet::PRIMITIVES);
                if !unsupported_abilities.is_empty() {
                    return Err(TypeUnificationError::MissingAbilitiesForConstraints(
                        loc.clone(),
                        other.clone(),
                        unsupported_abilities,
                        ctx_opt,
                    ));
                }
                Ok(false)
            },
            (Constraint::SomeNumber(_), Constraint::HasAbilities(a1, _)) => {
                let unsupported_abilities = a1.setminus(AbilitySet::PRIMITIVES);
                if !unsupported_abilities.is_empty() {
                    return Err(TypeUnificationError::MissingAbilitiesForConstraints(
                        loc.clone(),
                        self.clone(),
                        unsupported_abilities,
                        ctx_opt,
                    ));
                }
                Ok(false)
            },
            // After the above checks, if one of the constraints is
            // accumulating, indicate its compatible but cannot be joined.
            (c1, c2) if c1.accumulating() || c2.accumulating() => Ok(false),
            // Otherwise the constraints are incompatible.
            _ => Err(TypeUnificationError::ConstraintsIncompatible(
                loc.clone(),
                self.clone(),
                other.clone(),
            )),
        }
    }

    /// Returns the constraints which need to be satisfied to instantiate the given type
    /// parameter. This creates NoReference, NoTuple, NoPhantom unless the type
    /// parameter is phantom, and HasAbilities if any abilities need to be met.
    pub fn for_type_parameter(param: &TypeParameter) -> Vec<Constraint> {
        let mut result = vec![Constraint::NoReference, Constraint::NoTuple];
        let TypeParameter(
            _,
            TypeParameterKind {
                abilities,
                is_phantom,
            },
            _,
        ) = param;
        if !*is_phantom {
            result.push(Constraint::NoPhantom)
        }
        if !abilities.is_empty() {
            result.push(Constraint::HasAbilities(
                *abilities,
                AbilityCheckingScope::IncludeTypeParams,
            ));
        }
        result
    }

    /// Returns the constraints which need to be satisfied for a vector type parameter.
    pub fn for_vector() -> Vec<Constraint> {
        vec![
            Constraint::NoPhantom,
            Constraint::NoReference,
            Constraint::NoTuple,
        ]
    }

    /// Returns the constraints which need to be satisfied for a field type,
    /// given a struct with declared abilities.
    pub fn for_field(struct_abilities: AbilitySet, _field_ty: &Type) -> Vec<Constraint> {
        let mut result = vec![
            Constraint::NoPhantom,
            Constraint::NoTuple,
            Constraint::NoReference,
        ];
        let abilities = if struct_abilities.has_ability(Ability::Key) {
            struct_abilities.remove(Ability::Key).add(Ability::Store)
        } else {
            struct_abilities
        };
        result.push(Constraint::HasAbilities(
            abilities,
            AbilityCheckingScope::ExcludeTypeParams,
        ));
        result
    }

    /// Returns the constraints which need to be satisfied for function parameters.
    pub fn for_fun_parameter() -> Vec<Constraint> {
        vec![Constraint::NoPhantom, Constraint::NoTuple]
    }

    /// Returns the constraints which need to be satisfied for a local or
    /// parameter type.
    pub fn for_local() -> Vec<Constraint> {
        vec![Constraint::NoPhantom, Constraint::NoTuple]
    }

    /// Displays a constraint.
    pub fn display(&self, display_context: &TypeDisplayContext) -> String {
        fn fmt_types<'a>(ctx: &TypeDisplayContext, tys: impl Iterator<Item = &'a Type>) -> String {
            tys.map(|ty| ty.display(ctx)).join(",")
        }
        let pool = display_context.env.symbol_pool();
        match self {
            Constraint::SomeNumber(options) => {
                let all_ints = PrimitiveType::all_int_types()
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                let all_ints_including_num = PrimitiveType::all_int_types()
                    .into_iter()
                    .chain(iter::once(PrimitiveType::Num))
                    .collect::<BTreeSet<_>>();
                if options == &all_ints || options == &all_ints_including_num {
                    "integer".to_owned()
                } else {
                    options
                        .iter()
                        .map(|p| Type::new_prim(*p).display(display_context).to_string())
                        .join("|")
                }
            },
            Constraint::SomeReference(ty) => {
                format!("&{}", ty.display(display_context))
            },
            Constraint::SomeStruct(field_map) => {
                format!(
                    "struct{{{}}}",
                    field_map
                        .keys()
                        .map(|s| s.display(pool).to_string())
                        .join(",")
                )
            },
            Constraint::SomeReceiverFunction(name, inst, _, args, result) => {
                format!(
                    "fun self.{}{}({}):{}",
                    name.display(pool),
                    if let Some(inst) = inst {
                        format!("<{}>", fmt_types(display_context, inst.1.iter()))
                    } else {
                        "".to_owned()
                    },
                    fmt_types(display_context, args.iter()),
                    result.display(display_context)
                )
            },
            Constraint::SomeFunctionValue(arg, result) => {
                // Use display for function types with empty abilities, so
                // we can attach an 'open' ability set, as the abilities are
                // unknown.
                format!(
                    "{} has ..",
                    Type::function(arg.clone(), result.clone(), AbilitySet::EMPTY)
                        .display(display_context)
                )
            },
            Constraint::NoReference => "no-ref".to_string(),
            Constraint::NoTuple => "no-tuple".to_string(),
            Constraint::NoPhantom => "no-phantom".to_string(),
            Constraint::HasAbilities(required_abilities, _) => {
                format!("{}", required_abilities)
            },
            Constraint::WithDefault(_ty) => "".to_owned(),
        }
    }
}

/// Represents an error resulting from type unification.
#[derive(Debug)]
pub enum TypeUnificationError {
    /// The two types mismatch: `TypeMismatch(actual, expected)`
    TypeMismatch(Type, Type),
    /// Same as `TypeMismatch`, but in the context of function arguments
    FunArgTypeMismatch(Type, Type),
    /// Same as `TypeMismatch`, but in the context of function result types
    FunResultTypeMismatch(Type, Type),
    /// The arity  of some construct mismatches: `ArityMismatch(for_type_args, actual, expected)`
    ArityMismatch(/*for_type_args*/ bool, usize, usize),
    /// Two types have different mutability: `MutabilityMismatch(actual, expected)`.
    MutabilityMismatch(Type, Type),
    /// A generic representation of the error that a constraint wasn't satisfied, with
    /// an optional constraint context.
    ConstraintUnsatisfied(
        Loc,
        Type,
        WideningOrder,
        Constraint,
        Option<ConstraintContext>,
    ),
    /// The `HasAbilities` constraint failed: `MissingAbilities(loc, ty, missing, ctx)`.
    MissingAbilities(Loc, Type, AbilitySet, Option<ConstraintContext>),
    /// The `HasAbilities` constraint failed: `MissingAbilitiesForConstraints(loc, ctr, missing, ctx)`.
    MissingAbilitiesForConstraints(Loc, Constraint, AbilitySet, Option<ConstraintContext>),
    /// The two constraints are incompatible and cannot be joined.
    ConstraintsIncompatible(Loc, Constraint, Constraint),
    /// A cyclic substitution when trying to unify the given types.
    CyclicSubstitution(Type, Type),
    /// Redirect the error message for the error to the given location.
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
    #[allow(deprecated)]
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

    /// Infer a type from a value. Returns the set of int types which can fit the
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

    /// Gets the manimal value allowed for a numeric type, or none if it is unbounded.
    pub fn get_min_value(self: &PrimitiveType) -> Option<BigInt> {
        match self {
            PrimitiveType::U8 => Some(BigInt::zero()),
            PrimitiveType::U16 => Some(BigInt::zero()),
            PrimitiveType::U32 => Some(BigInt::zero()),
            PrimitiveType::U64 => Some(BigInt::zero()),
            PrimitiveType::U128 => Some(BigInt::zero()),
            PrimitiveType::U256 => Some(BigInt::zero()),
            PrimitiveType::Num => None,
            _ => unreachable!("no num type"),
        }
    }

    /// Gets the number of bits in the type, or None if unbounded..
    pub fn get_num_bits(self: &PrimitiveType) -> Option<usize> {
        match self {
            PrimitiveType::U8 => Some(8),
            PrimitiveType::U16 => Some(16),
            PrimitiveType::U32 => Some(32),
            PrimitiveType::U64 => Some(64),
            PrimitiveType::U128 => Some(128),
            PrimitiveType::U256 => Some(256),
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

    /// Creates a function type
    pub fn function(arg_ty: Type, res_ty: Type, abilities: AbilitySet) -> Self {
        Type::Fun(Box::new(arg_ty), Box::new(res_ty), abilities)
    }

    /// Determines whether this is a type parameter.
    pub fn is_type_parameter(&self) -> bool {
        matches!(self, Type::TypeParameter(..))
    }

    /// Determines whether this is a primitive.
    pub fn is_primitive(&self) -> bool {
        matches!(self, Type::Primitive(_))
    }

    /// Determines whether this is a function.
    pub fn is_function(&self) -> bool {
        matches!(self, Type::Fun(..))
    }

    /// Determines whether this is a function or a tuple with a function;
    /// this is useful to test a function parameter/return type for function values.
    pub fn has_function(&self) -> bool {
        match self {
            Type::Tuple(tys) => tys.iter().any(|ty| ty.is_function()),
            Type::Fun(..) => true,
            _ => false,
        }
    }

    /// Determines whether this is a reference.
    pub fn is_reference(&self) -> bool {
        matches!(self, Type::Reference(_, _))
    }

    /// If this is a reference, return the kind of the reference, otherwise None.
    pub fn ref_kind(&self) -> Option<ReferenceKind> {
        if let Type::Reference(kind, _) = self {
            Some(*kind)
        } else {
            None
        }
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

    /// Determines whether this is a variant struct
    pub fn is_variant_struct(&self, env: &GlobalEnv) -> bool {
        self.is_struct() && self.get_struct(env).expect("struct").0.has_variants()
    }

    /// Determines whether this is the error type.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Determines whether this type is a vector
    pub fn is_vector(&self) -> bool {
        matches!(self, Type::Vector(..))
    }

    /// Get the element type of a vector
    pub fn get_vector_element_type(&self) -> Option<Type> {
        if let Type::Vector(e) = self {
            Some(e.as_ref().clone())
        } else {
            None
        }
    }

    /// If this is a function wrapper, return the inner function type
    pub fn get_function_wrapper_ty(&self, env: &GlobalEnv) -> Option<Type> {
        if let Some((struct_env, inst)) = self.get_struct(env) {
            let fields = struct_env.get_fields().collect_vec();
            if fields.len() == 1 && fields[0].is_positional() {
                let ty = fields[0].get_type();
                if ty.is_function() {
                    return Some(ty.instantiate(inst));
                }
            }
        }
        None
    }

    /// Get the target type of a reference
    pub fn get_target_type(&self) -> Option<&Type> {
        if let Type::Reference(_, t) = self {
            Some(t.as_ref())
        } else {
            None
        }
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

    pub fn describe_valid_for_constant() -> &'static str {
        "Expected one of `u8`, `u16, `u32`, `u64`, `u128`, `u256`, `bool`, `address`, \
         or `vector<_>` with valid element type."
    }

    /// Returns true if this type is a specification language only type or contains specification
    /// language only types
    pub fn is_spec(&self) -> bool {
        use Type::*;
        match self {
            Primitive(p) => p.is_spec(),
            Fun(args, result, _) => args.is_spec() || result.is_spec(),
            TypeDomain(..) | ResourceDomain(..) | Error => true,
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

    /// Drop reference, consuming the type.
    pub fn drop_reference(self) -> Type {
        if let Type::Reference(_, bt) = self {
            *bt
        } else {
            self
        }
    }

    /// If this is a reference, return its kind.
    pub fn try_reference_kind(&self) -> Option<ReferenceKind> {
        if let Type::Reference(k, _) = self {
            Some(*k)
        } else {
            None
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
                        if let Some(default_ty) = s.constraints.get(i).and_then(|ctrs| {
                            Constraint::default_type_for(ctrs.iter().map(|(_, _, c)| c))
                        }) {
                            default_ty.replace(params, subs, use_constr)
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
            Type::Fun(arg, result, abilities) => Type::Fun(
                Box::new(arg.replace(params, subs, use_constr)),
                Box::new(result.replace(params, subs, use_constr)),
                *abilities,
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
                Type::Fun(arg, result, _) => arg.contains(p) || result.contains(p),
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
            Fun(a, r, _) => a.is_incomplete() || r.is_incomplete(),
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
            Fun(a, r, _) => {
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
    #[allow(deprecated)]
    pub fn into_struct_type(self, env: &GlobalEnv) -> Option<MType> {
        use Type::*;
        match self {
            Struct(mid, sid, ts) => env.get_struct_type(mid, sid, &ts),
            _ => None,
        }
    }

    /// Attempt to convert this type into a normalized::Type
    #[allow(deprecated)]
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
                    }
                    ReferenceKind::Immutable => {
                        Some(MType::Reference(Box::new(t.into_normalized_type(env).expect("Invariant violation: reference type contains incomplete, tuple, or spec type"))))
                    }
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
                    .type_args
                    .iter()
                    .map(|arg| Self::from_type_tag(arg, env))
                    .collect();
                Struct(qid.module_id, qid.id, type_args)
            },
            TypeTag::Vector(type_param) => Vector(Box::new(Self::from_type_tag(type_param, env))),
            TypeTag::Function(fun) => {
                let FunctionTag {
                    args,
                    results,
                    abilities,
                } = fun.as_ref();
                let from_vec = |ts: &[TypeTag]| {
                    Type::tuple(ts.iter().map(|t| Type::from_type_tag(t, env)).collect_vec())
                };
                Fun(
                    Box::new(from_vec(args)),
                    Box::new(from_vec(results)),
                    *abilities,
                )
            },
        }
    }

    /// Generates a type from a signature token in the context of the given binary module.
    /// The `env` is only passed for general purposes, type name resolution is done
    /// via a special resolver function, allowing to work with partially populated
    /// environments.
    pub fn from_signature_token(
        env: &GlobalEnv,
        module: &CompiledModule,
        struct_resolver: &impl Fn(ModuleName, Symbol) -> QualifiedId<StructId>,
        sig: &SignatureToken,
    ) -> Self {
        let from_slice = |ts: &[SignatureToken]| {
            ts.iter()
                .map(|t| Self::from_signature_token(env, module, struct_resolver, t))
                .collect::<Vec<_>>()
        };
        match sig {
            SignatureToken::Bool => Type::Primitive(PrimitiveType::Bool),
            SignatureToken::U8 => Type::Primitive(PrimitiveType::U8),
            SignatureToken::U16 => Type::Primitive(PrimitiveType::U16),
            SignatureToken::U32 => Type::Primitive(PrimitiveType::U32),
            SignatureToken::U64 => Type::Primitive(PrimitiveType::U64),
            SignatureToken::U128 => Type::Primitive(PrimitiveType::U128),
            SignatureToken::U256 => Type::Primitive(PrimitiveType::U256),
            SignatureToken::Address => Type::Primitive(PrimitiveType::Address),
            SignatureToken::Signer => Type::Primitive(PrimitiveType::Signer),
            SignatureToken::Reference(t) => Type::Reference(
                ReferenceKind::Immutable,
                Box::new(Self::from_signature_token(env, module, struct_resolver, t)),
            ),
            SignatureToken::MutableReference(t) => Type::Reference(
                ReferenceKind::Mutable,
                Box::new(Self::from_signature_token(env, module, struct_resolver, t)),
            ),
            SignatureToken::TypeParameter(index) => Type::TypeParameter(*index),
            SignatureToken::Vector(bt) => Type::Vector(Box::new(Self::from_signature_token(
                env,
                module,
                struct_resolver,
                bt,
            ))),
            SignatureToken::Struct(handle_idx) => {
                let struct_view =
                    StructHandleView::new(module, module.struct_handle_at(*handle_idx));
                let struct_id = struct_resolver(
                    env.to_module_name(&struct_view.module_id()),
                    env.symbol_pool.make(struct_view.name().as_str()),
                );
                Type::Struct(struct_id.module_id, struct_id.id, vec![])
            },
            SignatureToken::StructInstantiation(handle_idx, args) => {
                let struct_view =
                    StructHandleView::new(module, module.struct_handle_at(*handle_idx));
                let struct_id = struct_resolver(
                    env.to_module_name(&struct_view.module_id()),
                    env.symbol_pool.make(struct_view.name().as_str()),
                );
                Type::Struct(struct_id.module_id, struct_id.id, from_slice(args))
            },
            SignatureToken::Function(args, result, abilities) => Type::Fun(
                Box::new(Type::tuple(from_slice(args))),
                Box::new(Type::Tuple(from_slice(result))),
                *abilities,
            ),
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
            Fun(a, r, _) => {
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
            Type::Fun(a, ty, _) => {
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

    /// If this is a tuple and it is not a unit type, return true.
    pub fn is_non_unit_tuple(&self) -> bool {
        matches!(self, Type::Tuple(ts) if !ts.is_empty())
    }

    /// If this is a tuple, return true.
    pub fn is_tuple(&self) -> bool {
        matches!(self, Type::Tuple(_))
    }
}

/// A parameter for type unification that specifies the type compatibility rules to follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Variance {
    /// All integer types are compatible, and reference types are eliminated.
    SpecVariance,
    /// Same like `SpecVariance` but only for outermost types. This is useful for preventing
    /// variance for type parameters: e.g. we want `num` and `u64` be substitutable, but
    /// not `vector<num>` and `vector<u64>`.
    ShallowSpecVariance,
    /// Variance used in the impl language fragment. This is currently for adapting mutable to
    /// immutable references, and function types
    ShallowImplVariance,
    /// Variance used in the impl language fragment for inline functions. Historically,
    /// inline functions allow variance in function value types, and this variance is
    /// used to capture this.
    ShallowImplInlineVariance,
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
        matches!(
            self,
            Variance::ShallowImplVariance | Variance::ShallowImplInlineVariance
        )
    }

    pub fn is_impl_inline_variance(self) -> bool {
        matches!(self, Variance::ShallowImplInlineVariance)
    }

    /// Constructs the variance to be used for subterms of the current type.
    pub fn sub_variance(self) -> Variance {
        match self {
            Variance::ShallowSpecVariance => Variance::NoVariance,
            Variance::SpecVariance => Variance::SpecVariance,
            Variance::ShallowImplVariance => Variance::NoVariance,
            Variance::ShallowImplInlineVariance => Variance::NoVariance,
            Variance::NoVariance => Variance::NoVariance,
        }
    }

    /// Constructs the variance to be used for argument/result of a function
    /// type. The behavior here differs for inline function parameters: those
    /// are allowed to have variance whereas for function values, this is not
    /// allowed. Inline functions had historically this behavior which can't be
    /// broken, whereas for function values, the required type checks at runtime
    /// are too expensive and hence not supported.
    fn fun_argument_variance(self) -> Variance {
        match self {
            Variance::ShallowImplInlineVariance => self,
            _ => self.sub_variance(),
        }
    }

    /// Makes a selected variance shallow, if possible.
    pub fn shallow(self) -> Self {
        match self {
            Variance::ShallowSpecVariance => Variance::ShallowSpecVariance,
            Variance::SpecVariance => Variance::ShallowSpecVariance,
            Variance::ShallowImplVariance => Variance::ShallowImplVariance,
            Variance::ShallowImplInlineVariance => Variance::ShallowImplInlineVariance,
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

    /// Combine two orders. If they are the same or Join, self is returned, otherwise swapped
    /// order.
    pub fn combine(self, other: Self) -> Self {
        if self == other || self == WideningOrder::Join {
            self
        } else {
            self.swap()
        }
    }
}

/// A trait to provide context information about abilities.
pub trait AbilityContext {
    /// Delivers the kind of the type parameter, as it is declared: `T: key+drop`.
    fn type_param(&self, idx: u16) -> TypeParameter;

    /// Delivers the signature of a struct, that is the kinds of its type parameters,
    /// and the offered abilities of the struct.
    fn struct_signature(
        &self,
        qid: QualifiedId<StructId>,
    ) -> (Symbol, Vec<TypeParameter>, AbilitySet);
}

/// A trait to provide context information for unification.
pub trait UnificationContext: AbilityContext {
    /// Get information about the given struct field. Returns a list
    /// of optional variant and type for the field in that variant,
    /// or, if the struct is not a variant, None and type.
    /// If the field is not defined returns an empty list.
    /// The 2nd return value indicates whether the type is a variant struct.
    fn get_struct_field_decls(
        &self,
        id: &QualifiedInstId<StructId>,
        field_name: Symbol,
    ) -> (Vec<(Option<Symbol>, Type)>, bool);

    /// If this is a function type wrapper (`struct W(|T|R)`), get the underlying
    /// function type.
    fn get_function_wrapper_type(&self, id: &QualifiedInstId<StructId>) -> Option<Type>;

    /// For a given type, return a receiver style function of the given name, if available.
    /// If the function is generic it will be instantiated with fresh type variables.
    fn get_receiver_function(
        &mut self,
        ty: &Type,
        name: Symbol,
    ) -> Option<ReceiverFunctionInstance>;

    /// Returns a type display context.
    fn type_display_context(&self) -> TypeDisplayContext;
}

/// Information returned about an instantiated function
#[derive(Debug, Clone)]
pub struct ReceiverFunctionInstance {
    /// Qualified id
    pub id: QualifiedId<FunId>,
    /// Function name
    pub fun_name: Symbol,
    /// Type parameters
    pub type_params: Vec<TypeParameter>,
    /// Type instantiation of the function
    pub type_inst: Vec<Type>,
    /// Types of the arguments, instantiated
    pub arg_types: Vec<Type>,
    /// Result type, instantiated
    pub result_type: Type,
    /// Whether this is an inline function.
    pub is_inline: bool,
}

impl ReceiverFunctionInstance {
    /// Given the actual argument type, determine whether it needs to be borrowed to be passed
    /// to this function. Returns the reference kind if so.
    pub fn receiver_needs_borrow(&self, actual_arg_type: &Type) -> Option<ReferenceKind> {
        match &self.arg_types[0] {
            Type::Reference(kind, _) if !actual_arg_type.is_reference() => Some(*kind),
            _ => None,
        }
    }
}

/// A struct representing an empty unification context.
pub struct NoUnificationContext;

impl UnificationContext for NoUnificationContext {
    fn get_struct_field_decls(
        &self,
        _id: &QualifiedInstId<StructId>,
        _field_name: Symbol,
    ) -> (Vec<(Option<Symbol>, Type)>, bool) {
        (vec![], false)
    }

    fn get_function_wrapper_type(&self, _id: &QualifiedInstId<StructId>) -> Option<Type> {
        None
    }

    fn get_receiver_function(
        &mut self,
        _ty: &Type,
        _name: Symbol,
    ) -> Option<ReceiverFunctionInstance> {
        None
    }

    fn type_display_context(&self) -> TypeDisplayContext {
        unimplemented!("NoUnificationContext does not support type display")
    }
}

impl AbilityContext for NoUnificationContext {
    fn type_param(&self, _idx: u16) -> TypeParameter {
        unimplemented!("NoUnificationContext does not support abilities")
    }

    fn struct_signature(
        &self,
        _qid: QualifiedId<StructId>,
    ) -> (Symbol, Vec<TypeParameter>, AbilitySet) {
        unimplemented!("NoUnificationContext does not support abilities")
    }
}

impl Substitution {
    /// Creates a new substitution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a constraint to the variable. This tries to first join the constraint with existing
    /// ones. For instance `SomeNumber({u8, u16})` and `SomeNumber({u16,u32})` join as
    /// `SomeNumber({u16})`. A TypeUnificationError is returned if the constraints are incompatible.
    pub fn add_constraint(
        &mut self,
        context: &mut impl UnificationContext,
        var: u32,
        loc: Loc,
        order: WideningOrder,
        ctr: Constraint,
        ctx_opt: Option<ConstraintContext>,
    ) -> Result<(), TypeUnificationError> {
        // Move current constraint list out of self to avoid sharing conflicts while it
        // is being transformed.
        let mut current = self.constraints.remove(&var).unwrap_or_default();
        let mut absorbed = false;
        for (_, _, c) in current.iter_mut() {
            // Join constraints. If join returns true the constraint is absorbed.
            absorbed = c.join(context, self, &loc, &ctr, ctx_opt.clone())?;
            if absorbed {
                break;
            }
        }
        if !absorbed {
            current.push((loc, order, ctr))
        }
        self.constraints.insert(var, current);
        if let Some(ctx) = ctx_opt {
            match self.constraint_contexts.entry(var) {
                Entry::Vacant(e) => {
                    e.insert(ctx);
                },
                Entry::Occupied(e) => {
                    let curr = e.into_mut();
                    curr.inferred |= ctx.inferred;
                    if matches!(ctx.origin, ConstraintOrigin::TypeParameter(..)) {
                        // Prefer type parameter origin as it leads to
                        // more precise error messages.
                        curr.origin = ctx.origin;
                    }
                },
            }
        }
        Ok(())
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
    pub fn bind(
        &mut self,
        context: &mut impl UnificationContext,
        var: u32,
        variance: Variance,
        order: WideningOrder,
        ty: Type,
    ) -> Result<(), TypeUnificationError> {
        // Specialize the type before binding, to maximize groundness of type terms.
        let mut ty = self.specialize(&ty);
        if let Some(mut constrs) = self.constraints.remove(&var) {
            // Sort constraints to report primary errors first
            constrs.sort_by(|(_, _, c1), (_, _, c2)| c1.compare(c2).reverse());
            while let Some((loc, o, c)) = constrs.pop() {
                // The effective order is the one combining the constraint order with the
                // context order. The result needs to be swapped because the constraint
                // of the variable is evaluated against the given type.
                match self.eval_constraint(
                    context,
                    &loc,
                    &ty,
                    variance,
                    o.combine(order).swap(),
                    c.clone(),
                    self.constraint_contexts.get(&var).cloned(),
                ) {
                    Ok(_) => {
                        // Constraint discharged
                    },
                    Err(e) => {
                        // Put the constraint back, we may need it for error messages
                        constrs.push((loc, o, c));
                        self.constraints.insert(var, constrs);
                        return Err(e);
                    },
                }
            }
            // New bindings could have been created, so specialize again.
            ty = self.specialize(&ty)
        }

        // Occurs check
        if ty.get_vars().contains(&var) {
            Err(TypeUnificationError::CyclicSubstitution(Type::Var(var), ty))
        } else {
            self.subs.insert(var, ty);
            Ok(())
        }
    }

    /// Evaluates whether the given type satisfies the constraint, discharging the constraint.
    /// Notice that discharging is possible since (a) for variables, we just transfer the
    /// constraint. (b) For other types, since constraints are over shallow structure of types,
    /// they can be decided based on the top-level type term.
    pub fn eval_constraint(
        &mut self,
        context: &mut impl UnificationContext,
        loc: &Loc,
        ty: &Type,
        variance: Variance,
        order: WideningOrder,
        c: Constraint,
        ctx_opt: Option<ConstraintContext>,
    ) -> Result<(), TypeUnificationError> {
        if c.report_only_once()
            && !self
                .reported
                .entry(ty.clone())
                .or_default()
                .insert(c.clone())
        {
            // Already reported constraint mismatch for this type
            return Ok(());
        }
        if matches!(ty, Type::Error) {
            Ok(())
        } else if let Type::Var(other_var) = ty {
            // Transfer constraint on to other variable which we assert to be free
            debug_assert!(!self.subs.contains_key(other_var));
            self.add_constraint(context, *other_var, loc.clone(), order, c, ctx_opt)
        } else if c.propagate_over_reference() && ty.is_reference() {
            // Propagate constraint to referred type
            self.eval_constraint(
                context,
                loc,
                ty.skip_reference(),
                variance,
                order,
                c,
                ctx_opt,
            )
        } else {
            let constraint_unsatisfied_error = || {
                Err(TypeUnificationError::ConstraintUnsatisfied(
                    loc.clone(),
                    ty.clone(),
                    order,
                    c.clone(),
                    ctx_opt.clone(),
                ))
            };
            match (&c, ty) {
                (Constraint::SomeNumber(options), Type::Primitive(prim))
                    if options.contains(prim) =>
                {
                    Ok(())
                },
                (Constraint::SomeReference(inner_type), Type::Reference(_, target_type)) => self
                    .unify(context, variance, order, target_type, inner_type)
                    .map(|_| ())
                    .map_err(|e| e.redirect(loc.clone())),
                (Constraint::SomeStruct(constr_field_map), Type::Struct(mid, sid, inst)) => {
                    let sid = &mid.qualified_inst(*sid, inst.clone());
                    for (field_name, expected_type) in constr_field_map {
                        let (mut field_decls, _) = context.get_struct_field_decls(sid, *field_name);
                        if field_decls.is_empty() {
                            return constraint_unsatisfied_error();
                        }
                        // All available definitions must have the same type, before instantiation.
                        let (_, decl_type) = field_decls.pop().unwrap();
                        if field_decls
                            .into_iter()
                            .any(|(_, other_ty)| decl_type != other_ty)
                        {
                            return constraint_unsatisfied_error();
                        }
                        // The given declared type must unify with the expected type
                        self.unify(
                            context,
                            variance,
                            WideningOrder::RightToLeft,
                            expected_type,
                            &decl_type,
                        )
                        .map(|_| ())
                        .map_err(|e| e.redirect(loc.clone()))?
                    }
                    Ok(())
                },
                (
                    Constraint::SomeReceiverFunction(name, ty_args_opt, args_loc, args, result),
                    ty,
                ) => {
                    if let Some(receiver) = context.get_receiver_function(ty, *name) {
                        let variance = if variance.is_impl_variance() && receiver.is_inline {
                            // Switch to inline function variance now that we know this is an
                            // inline function. At the moment the constraint is constructed,
                            // this is not known.
                            Variance::ShallowImplInlineVariance
                        } else {
                            variance
                        };
                        self.eval_receiver_function_constraint(
                            context,
                            loc,
                            variance,
                            ty_args_opt,
                            args_loc,
                            args,
                            result,
                            receiver,
                        )
                    } else {
                        constraint_unsatisfied_error()
                    }
                },
                (
                    Constraint::SomeFunctionValue(ctr_arg_ty, ctr_result_ty),
                    Type::Fun(arg_ty, result_ty, _),
                ) => {
                    self.unify(
                        context,
                        variance.fun_argument_variance(),
                        order.swap(),
                        arg_ty,
                        ctr_arg_ty,
                    )
                    .map_err(TypeUnificationError::map_to_fun_arg_mismatch)?;
                    self.unify(
                        context,
                        variance.fun_argument_variance(),
                        order,
                        result_ty,
                        ctr_result_ty,
                    )
                    .map_err(TypeUnificationError::map_to_fun_result_mismatch)?;
                    Ok(())
                },
                (
                    Constraint::SomeFunctionValue(ctr_arg_ty, ctr_result_ty),
                    Type::Struct(mid, sid, inst),
                ) => {
                    let sid = &mid.qualified_inst(*sid, inst.clone());
                    if let Some(Type::Fun(arg_ty, result_ty, _)) =
                        context.get_function_wrapper_type(sid)
                    {
                        self.unify(
                            context,
                            variance.fun_argument_variance(),
                            order.swap(),
                            &arg_ty,
                            ctr_arg_ty,
                        )
                        .map_err(TypeUnificationError::map_to_fun_arg_mismatch)?;
                        self.unify(
                            context,
                            variance.fun_argument_variance(),
                            order,
                            &result_ty,
                            ctr_result_ty,
                        )
                        .map_err(TypeUnificationError::map_to_fun_result_mismatch)?;
                        Ok(())
                    } else {
                        constraint_unsatisfied_error()
                    }
                },
                (Constraint::HasAbilities(required_abilities, scope), ty) => self
                    .eval_ability_constraint(
                        context,
                        loc,
                        *required_abilities,
                        *scope,
                        ty,
                        ctx_opt,
                    ),
                (Constraint::NoReference, ty) => {
                    if ty.is_reference() {
                        constraint_unsatisfied_error()
                    } else {
                        Ok(())
                    }
                },
                (Constraint::NoTuple, ty) => {
                    if ty.is_tuple() {
                        constraint_unsatisfied_error()
                    } else {
                        Ok(())
                    }
                },
                (Constraint::NoPhantom, ty) => match ty {
                    Type::TypeParameter(idx) if context.type_param(*idx).1.is_phantom => {
                        constraint_unsatisfied_error()
                    },
                    _ => Ok(()),
                },
                (Constraint::WithDefault(_), _) => Ok(()),
                _ => constraint_unsatisfied_error(),
            }
        }
    }

    fn eval_ability_constraint(
        &mut self,
        context: &mut impl UnificationContext,
        loc: &Loc,
        required_abilities: AbilitySet,
        required_abilities_scope: AbilityCheckingScope,
        ty: &Type,
        ctx_opt: Option<ConstraintContext>,
    ) -> Result<(), TypeUnificationError> {
        use Type::*;
        let check = |abilities: AbilitySet| {
            let missing = required_abilities.setminus(abilities);
            if !missing.is_empty() {
                Err(TypeUnificationError::MissingAbilities(
                    loc.clone(),
                    ty.clone(),
                    missing,
                    ctx_opt.clone(),
                ))
            } else {
                Ok(())
            }
        };
        match ty {
            Primitive(PrimitiveType::Signer) => check(AbilitySet::SIGNER),
            Primitive(_) => check(AbilitySet::PRIMITIVES),
            Tuple(ts) => {
                check(AbilitySet::PRIMITIVES)?;
                for (i, t) in ts.iter().enumerate() {
                    self.eval_ability_constraint(
                        context,
                        loc,
                        required_abilities,
                        required_abilities_scope,
                        t,
                        ctx_opt.clone().map(|ctx| ctx.derive_tuple_element(i)),
                    )?;
                }
                Ok(())
            },
            Vector(t) => {
                check(AbilitySet::VECTOR)?;
                self.eval_ability_constraint(
                    context,
                    loc,
                    required_abilities,
                    required_abilities_scope,
                    t,
                    ctx_opt.map(|ctx| ctx.derive_vector_type_param()),
                )
            },
            Struct(m, s, ts) => {
                let (name, type_params, struct_abilities) =
                    context.struct_signature(m.qualified(*s));

                check(struct_abilities)?;
                let required = if required_abilities.has_ability(Ability::Key) {
                    required_abilities.remove(Ability::Key).add(Ability::Store)
                } else {
                    required_abilities
                };
                for (i, t) in ts.iter().enumerate() {
                    let type_param = &type_params[i];
                    // Pass the requirements on to the type instantiation, except
                    // phantoms which are excluded from ability requirements
                    if !type_param.1.is_phantom {
                        self.eval_ability_constraint(
                            context,
                            loc,
                            required,
                            required_abilities_scope,
                            t,
                            ctx_opt
                                .clone()
                                .map(|ctx| ctx.derive_struct_parameter(name, type_param.clone())),
                        )?;
                    }
                    // Add constraints derived from the parameter itself.
                    for ctr in Constraint::for_type_parameter(type_param) {
                        self.eval_constraint(
                            context,
                            loc,
                            t,
                            Variance::NoVariance,
                            WideningOrder::LeftToRight,
                            ctr,
                            Some(ConstraintContext::default().for_type_param(
                                true,
                                name,
                                type_param.clone(),
                            )),
                        )?
                    }
                }
                Ok(())
            },
            TypeParameter(idx) => {
                if required_abilities_scope == AbilityCheckingScope::IncludeTypeParams {
                    let tparam = context.type_param(*idx);
                    check(tparam.1.abilities)
                } else {
                    Ok(())
                }
            },
            Fun(_, _, abilities) => check(*abilities),
            Reference(_, _) => check(AbilitySet::REFERENCES),
            TypeDomain(_) | ResourceDomain(_, _, _) => check(AbilitySet::EMPTY),
            Error => Ok(()),
            Var(idx) => {
                // Discharge the constraint by adding it to the substitution for
                // later evaluation.
                self.add_constraint(
                    context,
                    *idx,
                    loc.clone(),
                    WideningOrder::LeftToRight,
                    Constraint::HasAbilities(required_abilities, required_abilities_scope),
                    ctx_opt,
                )
            },
        }
    }

    fn eval_receiver_function_constraint(
        &mut self,
        context: &mut impl UnificationContext,
        loc: &Loc,
        variance: Variance,
        ty_args_opt: &Option<(Vec<Loc>, Vec<Type>)>,
        args_loc: &[Loc],
        args: &[Type],
        result: &Type,
        receiver: ReceiverFunctionInstance,
    ) -> Result<(), TypeUnificationError> {
        let mut args = args.to_vec();
        let borrow_kind = receiver.receiver_needs_borrow(&args[0]);
        if let Some(ref_kind) = borrow_kind {
            // Wrap a reference around the arg type to reflect it will be automatically borrowed
            let arg_type = args.remove(0);
            args.insert(0, Type::Reference(ref_kind, Box::new(arg_type)));
        }
        if let Some(ty_args) = ty_args_opt {
            // The call has explicit type parameters (`x.f<T>()`), check them.
            self.unify_vec_maybe_type_args(
                context,
                true,
                variance,
                WideningOrder::Join,
                // Pass in locations of type args for better error messages
                Some(&ty_args.0),
                &ty_args.1,
                &receiver.type_inst,
            )?;
        }
        // Need to add any constraints for type parameters.
        for (tparam, targ) in receiver.type_params.iter().zip(&receiver.type_inst) {
            for ctr in Constraint::for_type_parameter(tparam) {
                self.eval_constraint(
                    context,
                    loc,
                    &self.specialize(targ),
                    Variance::NoVariance,
                    WideningOrder::LeftToRight,
                    ctr,
                    Some(ConstraintContext::default().for_type_param(
                        false,
                        receiver.fun_name,
                        tparam.clone(),
                    )),
                )?
            }
        }
        self.unify_vec(
            context,
            variance,
            // Arguments are contra-variant, hence LeftToRight
            WideningOrder::LeftToRight,
            // Pass in locations of arguments for better error messages
            Some(args_loc),
            &args,
            &receiver.arg_types,
        )?;
        self.unify(
            context,
            variance,
            WideningOrder::RightToLeft,
            result,
            &receiver.result_type,
        )?;
        Ok(())
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
    /// This currently implements the following notion of type compatibility, depending
    /// on mode:
    ///
    /// In specification mode:
    /// - 1) References are dropped (i.e. &T and T are compatible)
    /// - 2) All integer types are compatible if spec-variance is allowed.
    /// - 3) If in two tuples (P1, P2, ..., Pn) and (Q1, Q2, ..., Qn), all (Pi, Qi) pairs are
    ///      compatible under spec-variance, then the two tuples are compatible under
    ///      spec-variance.
    ///
    /// In implementation mode:
    /// - 1) The only known variance at this point is from `&mut T` to `&T`.
    /// - 2) The same way as (3) above, implementation variance propagates over tuples.
    ///
    /// The substitution will be refined by variable assignments as needed to perform
    /// unification. If unification fails, the substitution will be in some intermediate state;
    /// to implement transactional unification, the substitution must be cloned before calling
    /// this.
    pub fn unify(
        &mut self,
        context: &mut impl UnificationContext,
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
                    Box::new(self.unify(context, variance, order, bt1.as_ref(), t2)?),
                ));
            }
            if let Type::Reference(kind, bt2) = t2 {
                return Ok(Type::Reference(
                    *kind,
                    Box::new(self.unify(context, variance, order, t1, bt2.as_ref())?),
                ));
            }
        }

        // Substitute or assign variables.
        if let Some(rt) = self.try_substitute_or_assign(context, variance, order, t1, t2)? {
            return Ok(rt);
        }
        if let Some(rt) = self.try_substitute_or_assign(context, variance, order.swap(), t2, t1)? {
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
                // For references, allow variance to be passed down, and not use sub-variance
                let ty = self
                    .unify(context, variance, order, ty1, ty2)
                    .map_err(TypeUnificationError::lift(order, t1, t2))?;
                let k = if variance.is_impl_variance() {
                    use ReferenceKind::*;
                    use WideningOrder::*;
                    match (k1, k2, order) {
                        (Immutable, Immutable, _) | (Mutable, Mutable, _) => k1,
                        (Immutable, Mutable, RightToLeft | Join) => k1,
                        (Mutable, Immutable, LeftToRight | Join) => k2,
                        _ => {
                            let (t1, t2) = if matches!(order, LeftToRight) {
                                (t1, t2)
                            } else {
                                (t2, t1)
                            };
                            return Err(TypeUnificationError::MutabilityMismatch(t1.clone(), t2.clone()));
                        },
                    }
                } else if *k1 != *k2 {
                    return Err(TypeUnificationError::MutabilityMismatch(t1.clone(), t2.clone()));
                } else {
                    k1
                };
                return Ok(Type::Reference(*k, Box::new(ty)));
            },
            (Type::Tuple(ts1), Type::Tuple(ts2)) => {
                return Ok(Type::Tuple(
                    self.unify_vec(
                        // Note for tuples, we pass on `variance` not `sub_variance`. A shallow
                        // variance type will be effective for the elements of tuples,
                        // which are treated similar as expression lists in function calls, and allow
                        // e.g. reference type conversions.
                        context, variance, order, None, ts1, ts2,
                    )
                    .map_err(TypeUnificationError::lift(order, t1, t2))?,
                ));
            },
            (Type::Fun(a1, r1, abilities1), Type::Fun(a2, r2, abilities2))
                // Abilities must be same if NoVariance requested
                if variance != Variance::NoVariance || abilities1 == abilities2 =>
            {
                // If variance is given, arguments can be converted, with contra-variance
                // in the argument type. Formally:
                //   |T1|R1 <= |T2|R2  <==>  T1 >= T2 && R1 <= R2
                // Intuitively, function f1 can safely _substitute_ function f2 if any argument
                // of type T2 can be passed as a T1 -- which is the case since T1 >= T2.
                let arg_ty = self
                    .unify(context, variance.fun_argument_variance(), order.swap(), a1, a2)
                    .map_err(TypeUnificationError::lift(order, t1, t2))?;
                let res_ty = self
                    .unify(context, variance.fun_argument_variance(), order, r1, r2)
                    .map_err(TypeUnificationError::lift(order, t1, t2))?;
                let abilities = {
                    // Widening/conversion can remove abilities, not add them.  So check that
                    // the target has no more abilities than the source.
                    let (missing_abilities, bad_ty) = match order {
                        WideningOrder::LeftToRight => (abilities2.setminus(*abilities1), t1),
                        WideningOrder::RightToLeft => (abilities1.setminus(*abilities2), t2),
                        WideningOrder::Join => (AbilitySet::EMPTY, t1),
                    };
                    if missing_abilities.is_empty() {
                        abilities1.intersect(*abilities2)
                    } else {
                        return Err(TypeUnificationError::MissingAbilities(
                            Loc::default(),
                            bad_ty.clone(),
                            missing_abilities,
                            None,
                        ));
                    }
                };
                return Ok(Type::Fun(Box::new(arg_ty), Box::new(res_ty), abilities));
            },
            (Type::Struct(m1, s1, ts1), Type::Struct(m2, s2, ts2)) => {
                if m1 == m2 && s1 == s2 {
                    // For structs, also pass on `variance`, not `sub_variance`, to inherit
                    // shallow processing to fields.
                    return Ok(Type::Struct(
                        *m1,
                        *s1,
                        self.unify_vec(context, variance, order, None, ts1, ts2)
                            .map_err(TypeUnificationError::lift(order, t1, t2))?,
                    ));
                }
            },
            (Type::Vector(e1), Type::Vector(e2)) => {
                return Ok(Type::Vector(Box::new(
                    self.unify(context, sub_variance, order, e1, e2)
                        .map_err(TypeUnificationError::lift(order, t1, t2))?,
                )));
            },
            (Type::TypeDomain(e1), Type::TypeDomain(e2)) => {
                return Ok(Type::TypeDomain(Box::new(self.unify(
                    context,
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
    pub fn unify_vec(
        &mut self,
        context: &mut impl UnificationContext,
        variance: Variance,
        order: WideningOrder,
        locs: Option<&[Loc]>,
        ts1: &[Type],
        ts2: &[Type],
    ) -> Result<Vec<Type>, TypeUnificationError> {
        self.unify_vec_maybe_type_args(context, false, variance, order, locs, ts1, ts2)
    }

    /// Helper to unify two type vectors, maybe mark as type arguments.
    pub fn unify_vec_maybe_type_args(
        &mut self,
        context: &mut impl UnificationContext,
        for_type_args: bool,
        variance: Variance,
        order: WideningOrder,
        locs: Option<&[Loc]>,
        ts1: &[Type],
        ts2: &[Type],
    ) -> Result<Vec<Type>, TypeUnificationError> {
        let ts1n = ts1.len();
        let ts2n = ts2.len();
        if ts1n != ts2n {
            let (given, expected) =
                if matches!(order, WideningOrder::LeftToRight | WideningOrder::Join) {
                    (ts1n, ts2n)
                } else {
                    (ts2n, ts1n)
                };
            return Err(TypeUnificationError::ArityMismatch(
                for_type_args,
                given,
                expected,
            ));
        }
        let mut rs = vec![];
        for i in 0..ts1.len() {
            let mut res = self.unify(context, variance, order, &ts1[i], &ts2[i]);
            if let Some(locs) = locs {
                res = res.map_err(|e| e.redirect(locs[i].clone()))
            }
            rs.push(res?);
        }
        Ok(rs)
    }

    /// Tries to substitute or assign a variable. Returned option is Some if unification
    /// was performed, None if not.
    fn try_substitute_or_assign(
        &mut self,
        context: &mut impl UnificationContext,
        variance: Variance,
        order: WideningOrder,
        t1: &Type,
        t2: &Type,
    ) -> Result<Option<Type>, TypeUnificationError> {
        if let Type::Var(v1) = t1 {
            if let Some(s1) = self.subs.get(v1).cloned() {
                return Ok(Some(self.unify(context, variance, order, &s1, t2)?));
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
            // Skip binding if we are unifying the same two variables.
            if t1 == &t2 {
                Ok(Some(t1.clone()))
            } else {
                self.bind(context, *v1, variance, order, t2.clone())?;
                Ok(Some(t2))
            }
        } else {
            Ok(None)
        }
    }
}

/// A context which determines how type unification errors are presented
/// to the user.
///
/// For each of the categories of errors (type mismatch, arity mismatch, etc.)
/// a specific error rendering function is defined below to display the error.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub enum ErrorMessageContext {
    /// The error appears in a binding, where the rhs is not assignable to the lhs.
    Binding,
    /// The error appears in an assignment, where the rhs is not assignable to the lhs.
    Assignment,
    /// The error appears in the argument list of a function.
    Argument,
    /// The error appears in the argument list of a positional constructor.
    PositionalUnpackArgument,
    /// The error appears in a type argument.
    TypeArgument,
    /// The error appears in the argument of a receiver style function.
    ReceiverArgument,
    /// The error appears in the argument of an operator.
    OperatorArgument,
    /// The error appears in a type annotation.
    TypeAnnotation,
    /// The error appears in the return expression of a function.
    Return,
    /// The error appears in the context of including a schema and
    /// binding the given name.
    SchemaInclusion(Symbol),
    /// The error appears in a general generic context.
    General,
}

/// Note: we currently do not have context specific messages for constraint mismatches.
/// They are handled generically in `TypeUnificationError::message`.
impl ErrorMessageContext {
    pub fn type_mismatch(
        self,
        display_context: &TypeDisplayContext,
        actual: &Type,
        expected: &Type,
    ) -> String {
        self.type_mismatch_str(
            display_context,
            actual.display(display_context).to_string(),
            expected.display(display_context).to_string(),
        )
    }

    pub fn type_mismatch_str(
        self,
        display_context: &TypeDisplayContext,
        actual: String,
        expected: String,
    ) -> String {
        use ErrorMessageContext::*;
        match self {
            Binding => format!(
                "cannot bind `{}` to left-hand side of type `{}`",
                actual, expected
            ),
            Assignment => format!(
                "cannot assign `{}` to left-hand side of type `{}`",
                actual, expected
            ),
            Argument | ReceiverArgument => format!(
                "cannot pass `{}` to a function which expects argument of type `{}`",
                actual, expected
            ),
            PositionalUnpackArgument => format!(
                "cannot match {} to a struct field of type {}",
                actual, expected
            ),
            OperatorArgument => format!(
                "cannot use `{}` with an operator which expects a value of type `{}`",
                actual, expected
            ),
            TypeArgument => format!(
                "cannot use `{}` as a type argument which is expected to be of type `{}`",
                actual, expected
            ),
            TypeAnnotation => format!("cannot adapt `{}` to annotated type `{}`", actual, expected),
            Return => {
                let result_str = if expected == "()" {
                    "which returns nothing".to_string()
                } else {
                    format!("with result type `{}`", expected)
                };
                let actual_str = if actual == "()" {
                    "nothing".to_string()
                } else {
                    format!("`{}`", actual)
                };
                format!(
                    "cannot return {} from a function {}",
                    actual_str, result_str
                )
            },
            SchemaInclusion(name) => {
                format!(
                    "variable `{}` bound by schema \
                inclusion expected to have type `{}` but provided was `{}`",
                    name.display(display_context.env.symbol_pool()),
                    expected,
                    actual
                )
            },
            General => {
                if expected == "()" {
                    format!("expected expression with no value but found `{}`", actual)
                } else {
                    format!(
                        "expected `{}` but found a value of type `{}`",
                        expected, actual
                    )
                }
            },
        }
    }

    pub fn arity_mismatch(self, for_type_args: bool, actual: usize, expected: usize) -> String {
        use ErrorMessageContext::*;
        match self {
            Binding | Assignment => format!(
                "the left-hand side has {} {} but the right-hand side provided {}",
                expected,
                pluralize("item", expected),
                actual,
            ),
            Argument => format!(
                "the function takes {} {} but {} were provided",
                expected,
                if for_type_args {
                    pluralize("type argument", expected)
                } else {
                    pluralize("argument", expected)
                },
                actual
            ),
            PositionalUnpackArgument => format!(
                "the struct/variant has {} {} but {} were provided",
                expected,
                if for_type_args {
                    pluralize("type argument", expected)
                } else {
                    pluralize("field", expected)
                },
                actual
            ),
            ReceiverArgument => {
                if for_type_args {
                    format!(
                        "the receiver function takes {} type {} but {} were provided",
                        expected,
                        pluralize("argument", expected),
                        actual
                    )
                } else {
                    format!(
                        "the receiver function takes {} {} but {} were provided",
                        expected - 1,
                        pluralize("argument", expected - 1),
                        actual - 1
                    )
                }
            },
            OperatorArgument => format!(
                "the operator takes {} {} but {} were provided",
                expected,
                pluralize("argument", expected),
                actual
            ),
            TypeArgument => {
                format!(
                    "expected {} type {} but {} were provided",
                    expected,
                    pluralize("argument", expected),
                    actual
                )
            },
            Return => format!(
                "the function returns {} {} but {} were provided",
                expected,
                pluralize("argument", expected),
                actual
            ),
            SchemaInclusion(_) | General | TypeAnnotation => {
                format!("expected {} items but found {}", expected, actual)
            },
        }
    }

    pub fn mutability_mismatch(
        self,
        display_context: &TypeDisplayContext,
        actual: &Type,
        expected: &Type,
    ) -> String {
        let msg = self.type_mismatch(display_context, actual, expected);
        format!("{} (mutability mismatch)", msg)
    }

    pub fn expected_reference(self, display_context: &TypeDisplayContext, actual: &Type) -> String {
        use ErrorMessageContext::*;
        let actual = actual.display(display_context);
        match self {
            Argument | ReceiverArgument => format!(
                "the function takes a reference but `{}` was provided",
                actual
            ),
            PositionalUnpackArgument => format!(
                "the struct/variant has a reference field but `{}` was provided",
                actual
            ),
            OperatorArgument => {
                format!(
                    "the operator takes a reference but `{}` was provided",
                    actual
                )
            },
            SchemaInclusion(_) | Binding | Assignment | Return | TypeAnnotation | General
            | TypeArgument => {
                format!("a reference is expected but `{}` was provided", actual)
            },
        }
    }
}

impl TypeUnificationError {
    /// Redirect the error to be reported at given location instead of default location.
    pub fn redirect(self, loc: Loc) -> Self {
        Self::RedirectedError(loc, Box::new(self))
    }

    /// Lifts a type unification error from the critical pair to the given context type.
    /// A critical pair in type unification is the sub-term in which two type terms disagree,
    /// e.g for `S<t> != S<t'>`, `(t, t')` is the critical pair.
    /// NOTE: we may consider to store both critical pair and context type in the unification error
    /// for better messages. However, the majority of type expressions is not very large in Move
    /// so this may make create more noise than benefit.
    pub fn lift<'a>(
        order: WideningOrder,
        cty1: &'a Type,
        cty2: &'a Type,
    ) -> impl Fn(TypeUnificationError) -> TypeUnificationError + 'a {
        move |this| {
            match this {
                // A SomeNumber constraint error is conceptually the same as a TypeMismatch,
                // so lift that one as well
                TypeUnificationError::TypeMismatch(_, _)
                | TypeUnificationError::ConstraintUnsatisfied(
                    _,
                    _,
                    _,
                    Constraint::SomeNumber(..),
                    _,
                ) => {
                    if matches!(order, WideningOrder::LeftToRight | WideningOrder::Join) {
                        TypeUnificationError::TypeMismatch(cty1.clone(), cty2.clone())
                    } else {
                        TypeUnificationError::TypeMismatch(cty2.clone(), cty1.clone())
                    }
                },
                TypeUnificationError::MutabilityMismatch(_, _) => {
                    if matches!(order, WideningOrder::LeftToRight | WideningOrder::Join) {
                        TypeUnificationError::MutabilityMismatch(cty1.clone(), cty2.clone())
                    } else {
                        TypeUnificationError::MutabilityMismatch(cty2.clone(), cty1.clone())
                    }
                },
                _ => this,
            }
        }
    }

    pub fn map_to_fun_arg_mismatch(self) -> Self {
        match self {
            TypeUnificationError::TypeMismatch(t1, t2)
            | TypeUnificationError::MutabilityMismatch(t1, t2) => {
                TypeUnificationError::FunArgTypeMismatch(t1, t2)
            },
            _ => self,
        }
    }

    pub fn map_to_fun_result_mismatch(self) -> Self {
        match self {
            TypeUnificationError::TypeMismatch(t1, t2)
            | TypeUnificationError::MutabilityMismatch(t1, t2) => {
                TypeUnificationError::FunResultTypeMismatch(t1, t2)
            },
            _ => self,
        }
    }

    /// If this error is associated with a specific location and the error
    /// is better reported at that location, return it.
    pub fn specific_loc(&self) -> Option<Loc> {
        match self {
            TypeUnificationError::ConstraintUnsatisfied(_, _, _, c, _) if !c.accumulating() => {
                // Non-accumulating constraints like `SomeNumber` or more similar than
                // regular type errors and are better reported at the expression leading
                // to the error instead of the location where the constraint stems from
                None
            },
            TypeUnificationError::RedirectedError(loc, e) => {
                Some(e.specific_loc().unwrap_or_else(|| loc.clone()))
            },
            TypeUnificationError::ConstraintsIncompatible(loc, ..)
            | TypeUnificationError::ConstraintUnsatisfied(loc, ..)
            | TypeUnificationError::MissingAbilities(loc, ..) => Some(loc.clone()),
            _ => None,
        }
        .and_then(|loc| if loc.is_default() { None } else { Some(loc) })
    }

    /// Return the message for this error.
    pub fn message(
        &self,
        unification_context: &impl UnificationContext,
        error_context: &ErrorMessageContext,
    ) -> String {
        self.message_with_hints_and_labels(unification_context, error_context)
            .0
    }

    /// Return the message for this error.
    pub fn message_with_hints_and_labels(
        &self,
        unification_context: &impl UnificationContext,
        error_context: &ErrorMessageContext,
    ) -> (String, Vec<String>, Vec<(Loc, String)>) {
        let display_context = &unification_context.type_display_context();
        match self {
            TypeUnificationError::TypeMismatch(actual, expected) => (
                error_context.type_mismatch(display_context, actual, expected),
                vec![],
                vec![],
            ),
            TypeUnificationError::FunArgTypeMismatch(expected, actual) => (
                // Because of contra-variance, switches actual/expected order
                format!(
                    "expected function type has argument of type `{}` but `{}` was provided",
                    expected.display(display_context),
                    actual.display(display_context),
                ),
                vec![],
                vec![],
            ),
            TypeUnificationError::FunResultTypeMismatch(actual, expected) => (
                format!(
                    "expected function type returns value of type `{}` but `{}` was provided",
                    expected.display(display_context),
                    actual.display(display_context),
                ),
                vec![],
                vec![],
            ),
            TypeUnificationError::ArityMismatch(for_type_args, actual, expected) => (
                error_context.arity_mismatch(*for_type_args, *actual, *expected),
                vec![],
                vec![],
            ),
            TypeUnificationError::CyclicSubstitution(_actual, _expected) => {
                // We could print the types but users may find this more confusing than
                // helpful.
                (
                    "unable to infer type due to cyclic \
                    type constraints (try annotating the type)"
                        .to_string(),
                    vec![],
                    vec![],
                )
            },
            TypeUnificationError::MutabilityMismatch(actual, expected) => (
                error_context.mutability_mismatch(display_context, actual, expected),
                vec![],
                vec![],
            ),
            TypeUnificationError::MissingAbilities(_, ty, missing, ctx_opt) => {
                let (note, hints, labels) = ctx_opt
                    .as_ref()
                    .map(|ctx| ctx.describe(display_context))
                    .unwrap_or_default();
                (
                    format!(
                        "type `{}` is missing required {} `{}`{}",
                        ty.display(display_context),
                        pluralize("ability", missing.iter().count()),
                        missing,
                        if !note.is_empty() {
                            format!(" ({})", note)
                        } else {
                            "".to_string()
                        }
                    ),
                    hints,
                    labels,
                )
            },
            TypeUnificationError::MissingAbilitiesForConstraints(_, ctr, missing, ctx_opt) => {
                let (note, hints, labels) = ctx_opt
                    .as_ref()
                    .map(|ctx| ctx.describe(display_context))
                    .unwrap_or_default();
                (
                    format!(
                        "constraint `{}` does not have required {} `{}`{}",
                        ctr.display(display_context),
                        pluralize("ability", missing.iter().count()),
                        missing,
                        if !note.is_empty() {
                            format!(" ({})", note)
                        } else {
                            "".to_string()
                        }
                    ),
                    hints,
                    labels,
                )
            },
            TypeUnificationError::ConstraintUnsatisfied(_, ty, order, constr, ctx_opt) => {
                let item_name = || match ctx_opt {
                    Some(ConstraintContext {
                        origin: ConstraintOrigin::Field(_),
                        ..
                    }) => "as a field type",
                    Some(ConstraintContext {
                        origin: ConstraintOrigin::Local(_),
                        ..
                    }) => "as a local variable type",
                    Some(ConstraintContext {
                        origin: ConstraintOrigin::Unspecified,
                        ..
                    }) => "",
                    Some(ConstraintContext {
                        origin: ConstraintOrigin::TupleElement(_, _),
                        ..
                    }) => "as a tuple element",
                    _ => "as a type argument",
                };
                let (mut note, mut hints, mut labels) = ctx_opt
                    .as_ref()
                    .map(|ctx| ctx.describe(display_context))
                    .unwrap_or_default();
                let main_msg = match constr {
                    Constraint::SomeNumber(_) => {
                        let options_str = constr.display(display_context);
                        let type_str = ty.display(display_context).to_string();
                        let (expected, actual) = match order {
                            WideningOrder::Join | WideningOrder::LeftToRight => {
                                (options_str, type_str)
                            },
                            WideningOrder::RightToLeft => (type_str, options_str),
                        };
                        // Providing instantiation context for number constraints is
                        // confusing for users. Those constraints are used in
                        // operators like `*` and so on but not visible to the user.
                        // Clear the according context information.
                        note = String::new();
                        hints = vec![];
                        labels = vec![];
                        error_context.type_mismatch_str(display_context, actual, expected)
                    },
                    Constraint::SomeReference(ty) => {
                        error_context.expected_reference(display_context, ty)
                    },
                    Constraint::SomeStruct(field_map) => {
                        let (main_msg, mut special_hints) = Self::message_for_struct(
                            unification_context,
                            display_context,
                            field_map,
                            ty,
                        );
                        hints.append(&mut special_hints);
                        main_msg
                    },
                    Constraint::SomeReceiverFunction(name, ..) => {
                        format!(
                            "undeclared receiver function `{}` for type `{}`",
                            name.display(display_context.env.symbol_pool()),
                            ty.display(display_context)
                        )
                    },
                    Constraint::SomeFunctionValue(arg_ty, result_ty) => {
                        format!(
                            "expected function of type `{}` but found `{}`",
                            Type::function(arg_ty.clone(), result_ty.clone(), AbilitySet::EMPTY)
                                .display(display_context),
                            ty.display(display_context),
                        )
                    },
                    Constraint::NoTuple => {
                        format!(
                            "tuple type `{}` is not allowed {}",
                            ty.display(display_context),
                            item_name()
                        )
                    },
                    Constraint::NoReference => {
                        format!(
                            "reference type `{}` is not allowed {}",
                            ty.display(display_context),
                            item_name()
                        )
                    },
                    Constraint::NoPhantom => {
                        format!(
                            "phantom type `{}` can only be used as an argument for another phantom type parameter",
                            ty.display(display_context)
                        )
                    },
                    Constraint::HasAbilities(..) | Constraint::WithDefault(_) => {
                        unreachable!("unexpected constraint in error message")
                    },
                };
                if !note.is_empty() {
                    (format!("{} ({})", main_msg, note), hints, labels)
                } else {
                    (main_msg, hints, labels)
                }
            },
            TypeUnificationError::ConstraintsIncompatible(_, c1, c2) => {
                use Constraint::*;
                // Abstract details of gross incompatibilities
                match (c1, c2) {
                    (SomeStruct(..), SomeNumber(..)) | (SomeNumber(..), SomeStruct(..)) => (
                        "struct incompatible with integer".to_owned(),
                        vec![],
                        vec![],
                    ),
                    (SomeReference(..), SomeNumber(..)) | (SomeNumber(..), SomeReference(..)) => (
                        "reference incompatible with integer".to_owned(),
                        vec![],
                        vec![],
                    ),
                    _ => (
                        format!(
                            "constraint `{}` incompatible with `{}`",
                            c1.display(display_context),
                            c2.display(display_context)
                        ),
                        vec![],
                        vec![],
                    ),
                }
            },
            TypeUnificationError::RedirectedError(_, err) => {
                err.message_with_hints_and_labels(unification_context, error_context)
            },
        }
    }

    fn message_for_struct(
        unification_context: &impl UnificationContext,
        display_context: &TypeDisplayContext,
        field_map: &BTreeMap<Symbol, Type>,
        ty: &Type,
    ) -> (String, Vec<String>) {
        let mut hints = vec![];
        // Determine why this constraint did not match for better error message
        let msg = if let Type::Struct(mid, sid, inst) = ty {
            let mut errors = vec![];
            let sid = mid.qualified_inst(*sid, inst.clone());
            for (field_name, expected_type) in field_map {
                let field_str = field_name
                    .display(display_context.env.symbol_pool())
                    .to_string();
                let (mut field_decls, is_variant) =
                    unification_context.get_struct_field_decls(&sid, *field_name);
                if field_decls.is_empty() {
                    errors.push(format!(
                        "field `{}` not declared in {} `{}`",
                        field_str,
                        if is_variant {
                            "any of the variants of enum"
                        } else {
                            "struct"
                        },
                        ty.display(display_context)
                    ))
                } else {
                    let (variant_opt, decl_type) = field_decls.pop().unwrap();
                    let different_type_variants = field_decls
                        .into_iter()
                        .filter_map(|(variant_opt, other_ty)| {
                            if other_ty != decl_type {
                                Some((variant_opt, other_ty))
                            } else {
                                None
                            }
                        })
                        .collect_vec();
                    if !different_type_variants.is_empty() {
                        errors.push(format!(
                            "cannot select field `{}` since it has different \
                            types in variants of enum `{}`",
                            field_str,
                            ty.display(display_context)
                        ));
                        let diff_str = iter::once((variant_opt, decl_type))
                            .chain(different_type_variants)
                            .map(|(variant_opt, decl_type)| {
                                format!(
                                    "type `{}` in variant `{}`",
                                    decl_type.display(display_context),
                                    variant_opt
                                        .unwrap()
                                        .display(display_context.env.symbol_pool())
                                )
                            })
                            .join(" and ");
                        hints.push(format!("field `{}` has {}", field_str, diff_str))
                    } else {
                        // type error
                        errors.push(format!(
                            "field `{}` has type `{}` instead of expected type `{}`",
                            field_str,
                            decl_type.display(display_context),
                            expected_type.display(display_context)
                        ))
                    }
                }
            }
            errors.join(", ")
        } else {
            format!(
                "expected a struct{} but found `{}`",
                if field_map.is_empty() {
                    "".to_owned()
                } else {
                    format!(
                        " with {}",
                        Self::print_fields(display_context.env, field_map.keys().cloned(),)
                    )
                },
                ty.display(display_context)
            )
        };
        (msg, hints)
    }

    fn print_fields(env: &GlobalEnv, names: impl Iterator<Item = Symbol>) -> String {
        names
            .map(|n| format!("field `{}`", n.display(env.symbol_pool()),))
            .join(" and ")
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
    /// If present, the module name in which context the type is displayed. Used to shorten type names.
    pub module_name: Option<ModuleName>,
    /// Whether to display type variables. If false, they will be displayed as `_`, otherwise as `_<n>`.
    pub display_type_vars: bool,
    /// Modules which are in `use` and do not need address qualification.
    pub used_modules: BTreeSet<ModuleId>,
    /// Whether to use `m::T` for representing types, for stable output in docgen
    pub use_module_qualification: bool,
    /// Var types that are recursive and should appear as `..` in display
    pub recursive_vars: Option<BTreeSet<u32>>,
}

impl<'a> TypeDisplayContext<'a> {
    pub fn new(env: &'a GlobalEnv) -> TypeDisplayContext<'a> {
        Self {
            env,
            type_param_names: None,
            subs_opt: None,
            builder_struct_table: None,
            module_name: None,
            display_type_vars: false,
            used_modules: BTreeSet::new(),
            use_module_qualification: false,
            recursive_vars: None,
        }
    }

    pub fn with_type_vars(&self) -> Self {
        Self {
            display_type_vars: true,
            ..self.clone()
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
            module_name: None,
            display_type_vars: false,
            used_modules: BTreeSet::new(),
            use_module_qualification: false,
            recursive_vars: None,
        }
    }

    pub fn add_subs(self, subs: &'a Substitution) -> Self {
        Self {
            subs_opt: Some(subs),
            ..self
        }
    }

    pub fn map_var_to_self(&self, idx: u32) -> Self {
        Self {
            recursive_vars: if let Some(existing_set) = &self.recursive_vars {
                let mut new_set = existing_set.clone();
                new_set.insert(idx);
                Some(new_set)
            } else {
                Some(BTreeSet::from([idx]))
            },
            ..self.clone()
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
            Fun(a, t, abilities) => {
                f.write_str("|")?;
                write!(f, "{}", a.display(self.context))?;
                f.write_str("|")?;
                if !t.is_unit() {
                    write!(f, "{}", t.display(self.context))?;
                }
                if !abilities.is_empty() {
                    write!(f, " with {}", abilities)
                } else {
                    Ok(())
                }
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
                if let Some(recursive_vars) = &self.context.recursive_vars {
                    if recursive_vars.contains(idx) {
                        return f.write_str("..");
                    }
                }
                if let Some(ty) = self.context.subs_opt.and_then(|s| s.subs.get(idx)) {
                    write!(f, "{}", ty.display(self.context))
                } else if let Some(ctrs) =
                    self.context.subs_opt.and_then(|s| s.constraints.get(idx))
                {
                    let ctrs = ctrs.iter().filter(|c| !c.2.hidden()).collect_vec();
                    if ctrs.is_empty() {
                        f.write_str(&self.type_var_str(*idx))
                    } else {
                        let recursive_context = self.context.map_var_to_self(*idx);
                        let out = ctrs
                            .iter()
                            .map(|(_, _, c)| c.display(&recursive_context).to_string())
                            .join(" + ");
                        f.write_str(&out)
                    }
                } else {
                    f.write_str(&self.type_var_str(*idx))
                }
            },
            Error => f.write_str("*error*"),
        }
    }
}

impl<'a> TypeDisplay<'a> {
    fn type_var_str(&self, idx: u32) -> String {
        if self.context.display_type_vars {
            format!("_{}", idx)
        } else {
            "_".to_string()
        }
    }

    #[allow(clippy::assigning_clones)]
    fn struct_str(&self, mid: ModuleId, sid: StructId) -> String {
        let env = self.context.env;
        let mut str = if let Some(builder_table) = self.context.builder_struct_table {
            let qsym = builder_table.get(&(mid, sid)).expect("type known");
            qsym.display(self.context.env).to_string()
        } else {
            let struct_env = env.get_module(mid).into_struct(sid);
            let module_name = struct_env.module_env.get_name();
            let module_str = if self.context.use_module_qualification
                || self.context.used_modules.contains(&mid)
                || Some(module_name) == self.context.module_name.as_ref()
            {
                module_name.display(env).to_string()
            } else {
                module_name.display_full(env).to_string()
            };
            format!(
                "{}::{}",
                module_str,
                struct_env.get_name().display(env.symbol_pool())
            )
        };
        if !self.context.use_module_qualification {
            if let Some(mname) = &self.context.module_name {
                let s = format!("{}::", mname.name().display(self.context.env.symbol_pool()));
                if let Some(shortcut) = str.strip_prefix(&s) {
                    if let Some(tparams) = &self.context.type_param_names {
                        // Avoid name clash with type parameter
                        if !tparams.contains(&self.context.env.symbol_pool().make(shortcut)) {
                            str = shortcut.to_owned()
                        }
                    } else {
                        str = shortcut.to_owned();
                    }
                }
            }
        }
        str
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

/// A trait which allows to infer abilities of types
pub trait AbilityInference: AbilityContext {
    /// Infers the abilities of the type. The returned boolean indicates whether
    /// the type is a phantom type parameter,
    fn infer_abilities(&self, ty: &Type) -> (bool, AbilitySet) {
        let res = match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::Bool
                | PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64
                | PrimitiveType::U128
                | PrimitiveType::U256
                | PrimitiveType::Num
                | PrimitiveType::Range
                | PrimitiveType::EventStore
                | PrimitiveType::Address => (false, AbilitySet::PRIMITIVES),
                PrimitiveType::Signer => (false, AbilitySet::SIGNER),
            },
            Type::Vector(et) => (
                false,
                AbilitySet::VECTOR.intersect(self.infer_abilities(et).1),
            ),
            Type::Struct(mid, sid, ty_args) => (
                false,
                self.infer_struct_abilities(mid.qualified(*sid), ty_args),
            ),
            Type::TypeParameter(i) => {
                let param = self.type_param(*i);
                (param.1.is_phantom, param.1.abilities)
            },
            Type::Var(_) => (false, AbilitySet::EMPTY),
            Type::Reference(_, _) => (false, AbilitySet::REFERENCES),
            Type::Tuple(et) => (
                false,
                et.iter()
                    .map(|ty| self.infer_abilities(ty).1)
                    .reduce(|a, b| a.intersect(b))
                    .unwrap_or(AbilitySet::PRIMITIVES),
            ),
            Type::Fun(_, _, abilities) => (false, *abilities),
            Type::TypeDomain(_) | Type::ResourceDomain(_, _, _) | Type::Error => {
                (false, AbilitySet::EMPTY)
            },
        };
        res
    }

    fn infer_struct_abilities(&self, qid: QualifiedId<StructId>, ty_args: &[Type]) -> AbilitySet {
        let (_, ty_params, struct_abilities) = self.struct_signature(qid);
        let ty_args_abilities_meet = ty_args
            .iter()
            .zip(ty_params)
            .map(|(ty_arg, param)| {
                let ty_arg_abilities = self.infer_abilities(ty_arg).1;
                if param.1.is_phantom {
                    // phantom type parameters don't participate in ability derivations
                    AbilitySet::ALL
                } else {
                    ty_arg_abilities
                }
            })
            .fold(AbilitySet::ALL, AbilitySet::intersect);
        // a struct has copy/drop/store if it's declared with the ability
        // and all it's fields have the ability
        // a struct has key if it's declared with key
        // and all fields have store
        let result = struct_abilities.intersect(ty_args_abilities_meet);
        if struct_abilities.has_ability(Ability::Key)
            && ty_args_abilities_meet.has_ability(Ability::Store)
        {
            result.add(Ability::Key)
        } else {
            result.remove(Ability::Key)
        }
    }
}

/// A helper to infer abilities based on an environment and type parameters.
pub struct AbilityInferer<'a> {
    env: &'a GlobalEnv,
    type_params: &'a [TypeParameter],
}

impl<'a> AbilityInferer<'a> {
    pub fn new(env: &'a GlobalEnv, type_params: &'a [TypeParameter]) -> Self {
        Self { env, type_params }
    }
}

impl<'a> AbilityContext for AbilityInferer<'a> {
    fn type_param(&self, idx: u16) -> TypeParameter {
        self.type_params[idx as usize].clone()
    }

    fn struct_signature(
        &self,
        qid: QualifiedId<StructId>,
    ) -> (Symbol, Vec<TypeParameter>, AbilitySet) {
        let struct_env = self.env.get_struct(qid);
        (
            struct_env.get_name(),
            struct_env.get_type_parameters().to_vec(),
            struct_env.get_abilities(),
        )
    }
}

impl<'a> AbilityInference for AbilityInferer<'a> {}
