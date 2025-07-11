// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Contains definitions for the abstract syntax tree (AST) of the Move language.

use crate::{
    exp_rewriter::ExpRewriterFunctions,
    model::{
        EnvDisplay, FieldId, FunId, FunctionEnv, GlobalEnv, GlobalId, Loc, ModuleId, NodeId,
        Parameter, QualifiedId, QualifiedInstId, SchemaId, SpecFunId, StructId, TypeParameter,
        GHOST_MEMORY_PREFIX, SCRIPT_MODULE_NAME,
    },
    symbol::{Symbol, SymbolPool},
    ty::{ReferenceKind, Type, TypeDisplayContext},
};
use either::Either;
use internment::LocalIntern;
use itertools::{EitherOrBoth, Itertools};
use move_binary_format::file_format::{CodeOffset, Visibility};
use move_core_types::{account_address::AccountAddress, function::ClosureMask};
use num::BigInt;
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Debug, Error, Formatter},
    hash::Hash,
    iter,
    ops::{Deref, Range},
};

// =================================================================================================
/// # Declarations

#[derive(Debug)]
pub struct SpecVarDecl {
    pub loc: Loc,
    pub name: Symbol,
    pub type_params: Vec<TypeParameter>,
    pub type_: Type,
    pub init: Option<Exp>,
}

#[derive(Clone, Debug)]
pub struct SpecFunDecl {
    pub loc: Loc,
    pub name: Symbol,
    pub type_params: Vec<TypeParameter>,
    pub params: Vec<Parameter>,
    pub context_params: Option<Vec<(Symbol, bool)>>,
    pub result_type: Type,
    pub used_memory: BTreeSet<QualifiedInstId<StructId>>,
    pub uninterpreted: bool,
    pub is_move_fun: bool,
    pub is_native: bool,
    pub body: Option<Exp>,
    pub callees: BTreeSet<QualifiedInstId<SpecFunId>>,
    pub is_recursive: RefCell<Option<bool>>,
    /// The instantiations for which this function is known to use generic type reflection.
    pub insts_using_generic_type_reflection: RefCell<BTreeMap<Vec<Type>, bool>>,
    pub spec: RefCell<Spec>,
}

// =================================================================================================
/// # Attributes

#[derive(Debug, Clone)]
pub enum AttributeValue {
    Value(NodeId, Value),
    Name(NodeId, Option<ModuleName>, Symbol),
}

#[derive(Debug, Clone)]
pub enum Attribute {
    Apply(NodeId, Symbol, Vec<Attribute>),
    Assign(NodeId, Symbol, AttributeValue),
}

impl Attribute {
    pub fn name(&self) -> Symbol {
        match self {
            Attribute::Assign(_, s, _) | Attribute::Apply(_, s, _) => *s,
        }
    }

    pub fn has(attrs: &[Attribute], pred: impl Fn(&Attribute) -> bool) -> bool {
        attrs.iter().any(pred)
    }

    pub fn node_id(&self) -> NodeId {
        match self {
            Attribute::Assign(id, _, _) | Attribute::Apply(id, _, _) => *id,
        }
    }
}

// =================================================================================================
/// # Conditions

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum ConditionKind {
    LetPost(Symbol, Loc),
    LetPre(Symbol, Loc),
    Assert,
    Assume,
    Decreases,
    AbortsIf,
    AbortsWith,
    SucceedsIf,
    Modifies,
    Emits,
    Ensures,
    Requires,
    StructInvariant,
    FunctionInvariant,
    LoopInvariant,
    GlobalInvariant(Vec<(Symbol, Loc)>),
    GlobalInvariantUpdate(Vec<(Symbol, Loc)>),
    SchemaInvariant,
    Axiom(Vec<(Symbol, Loc)>),
    Update,
}

impl ConditionKind {
    /// Returns true of this condition allows the `old(..)` expression.
    pub fn allows_old(&self) -> bool {
        use ConditionKind::*;
        matches!(
            self,
            LetPost(..)
                | Assert
                | Assume
                | Emits
                | Ensures
                | LoopInvariant
                | GlobalInvariantUpdate(..)
        )
    }

    /// Returns true if this condition is allowed on a function declaration.
    pub fn allowed_on_fun_decl(&self, _visibility: Visibility) -> bool {
        use ConditionKind::*;
        matches!(
            self,
            Requires
                | AbortsIf
                | AbortsWith
                | SucceedsIf
                | Emits
                | Ensures
                | Modifies
                | FunctionInvariant
                | LetPost(..)
                | LetPre(..)
                | Update
        )
    }

    /// Returns true if this condition is allowed in a function body.
    pub fn allowed_on_fun_impl(&self) -> bool {
        use ConditionKind::*;
        matches!(
            self,
            Assert | Assume | Decreases | LoopInvariant | LetPost(..) | LetPre(..) | Update
        )
    }

    pub fn allowed_on_lambda_spec(&self) -> bool {
        // TODO(#16256): support all conditions allowed in `allowed_on_fun_decl`
        use ConditionKind::*;
        matches!(
            self,
            Requires | AbortsIf | Ensures | FunctionInvariant | LetPre(..)
        )
    }

    /// Returns true if this condition is allowed on a struct.
    pub fn allowed_on_struct(&self) -> bool {
        use ConditionKind::*;
        matches!(self, StructInvariant)
    }

    /// Returns true if this condition is allowed on a module.
    pub fn allowed_on_module(&self) -> bool {
        use ConditionKind::*;
        matches!(
            self,
            GlobalInvariant(..) | GlobalInvariantUpdate(..) | Axiom(..)
        )
    }
}

impl fmt::Display for ConditionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn display_ty_params(
            f: &mut Formatter<'_>,
            ty_params: &[(Symbol, Loc)],
        ) -> std::fmt::Result {
            if !ty_params.is_empty() {
                write!(
                    f,
                    "<{}>",
                    (0..ty_params.len()).map(|i| format!("#{}", i)).join(", ")
                )?;
            }
            Ok(())
        }

        use ConditionKind::*;
        match self {
            LetPost(sym, _loc) => write!(f, "let({:?})", sym),
            LetPre(sym, _loc) => write!(f, "let old({:?})", sym),
            Assert => write!(f, "assert"),
            Assume => write!(f, "assume"),
            Decreases => write!(f, "decreases"),
            AbortsIf => write!(f, "aborts_if"),
            AbortsWith => write!(f, "aborts_with"),
            SucceedsIf => write!(f, "succeeds_if"),
            Modifies => write!(f, "modifies"),
            Emits => write!(f, "emits"),
            Ensures => write!(f, "ensures"),
            Requires => write!(f, "requires"),
            StructInvariant | FunctionInvariant | LoopInvariant => write!(f, "invariant"),
            GlobalInvariant(ty_params) => {
                write!(f, "invariant")?;
                display_ty_params(f, ty_params)
            },
            GlobalInvariantUpdate(ty_params) => {
                write!(f, "invariant")?;
                display_ty_params(f, ty_params)?;
                write!(f, " update")
            },
            SchemaInvariant => {
                write!(f, "invariant")
            },
            Axiom(ty_params) => {
                write!(f, "axiom")?;
                display_ty_params(f, ty_params)
            },
            Update => {
                write!(f, "update")
            },
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Eq, Hash)]
pub enum QuantKind {
    Forall,
    Exists,
    Choose,
    ChooseMin,
}

impl QuantKind {
    /// Returns true of this is a choice like Some or Min.
    pub fn is_choice(self) -> bool {
        matches!(self, QuantKind::Choose | QuantKind::ChooseMin)
    }
}

impl fmt::Display for QuantKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use QuantKind::*;
        match self {
            Forall => write!(f, "forall"),
            Exists => write!(f, "exists"),
            Choose => write!(f, "choose"),
            ChooseMin => write!(f, "choose min"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Condition {
    pub loc: Loc,
    pub kind: ConditionKind,
    pub properties: PropertyBag,
    pub exp: Exp,
    pub additional_exps: Vec<Exp>,
}

impl Condition {
    /// Return all expressions in the condition, the primary one and the additional ones.
    pub fn all_exps(&self) -> impl Iterator<Item = &Exp> {
        std::iter::once(&self.exp).chain(self.additional_exps.iter())
    }

    /// Return all expressions in the condition, the primary one and the additional ones.
    pub fn all_exps_mut(&mut self) -> impl Iterator<Item = &mut Exp> {
        std::iter::once(&mut self.exp).chain(self.additional_exps.iter_mut())
    }
}

// =================================================================================================
/// # Specifications

/// A set of properties stemming from pragmas.
pub type PropertyBag = BTreeMap<Symbol, PropertyValue>;

/// The value of a property.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PropertyValue {
    Value(Value),
    Symbol(Symbol),
    QualifiedSymbol(QualifiedSymbol),
}

/// Specification and properties associated with a language item.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Spec {
    /// The location of this specification, if available.
    pub loc: Option<Loc>,
    /// The set of conditions associated with this item.
    pub conditions: Vec<Condition>,
    /// Any pragma properties associated with this item.
    pub properties: PropertyBag,
    /// If this is a function, specs associated with individual code points. Note: only used
    /// with v1 compile chain.
    pub on_impl: BTreeMap<CodeOffset, Spec>,
    /// The map to store ghost variable update statements inlined in the function body.
    pub update_map: BTreeMap<NodeId, Condition>,
}

impl Spec {
    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
            && self.on_impl.is_empty()
            && self.properties.is_empty()
            && self.update_map.is_empty()
    }

    pub fn filter<P>(&self, pred: P) -> impl Iterator<Item = &Condition>
    where
        P: FnMut(&&Condition) -> bool,
    {
        self.conditions.iter().filter(pred)
    }

    pub fn filter_kind(&self, kind: ConditionKind) -> impl Iterator<Item = &Condition> {
        self.filter(move |c| c.kind == kind)
    }

    pub fn filter_kind_axiom(&self) -> impl Iterator<Item = &Condition> {
        self.filter(move |c| matches!(c.kind, ConditionKind::Axiom(..)))
    }

    pub fn any<P>(&self, pred: P) -> bool
    where
        P: FnMut(&Condition) -> bool,
    {
        self.conditions.iter().any(pred)
    }

    pub fn any_kind(&self, kind: ConditionKind) -> bool {
        self.any(move |c| c.kind == kind)
    }

    /// Returns the functions used (called or loaded as a function value) in this spec, along with
    /// the sites of the calls or loads.
    pub fn used_funs_with_uses(&self) -> BTreeMap<QualifiedId<FunId>, BTreeSet<NodeId>> {
        let mut result = BTreeMap::new();
        for cond in self.conditions.iter().chain(self.update_map.values()) {
            for exp in cond.all_exps() {
                result.append(&mut exp.used_funs_with_uses())
            }
        }
        for on_impl in self.on_impl.values() {
            result.append(&mut on_impl.used_funs_with_uses())
        }
        result
    }

    /// Returns the functions called in this spec.  Does not include any functions used
    /// as function values.
    pub fn called_funs_with_callsites(&self) -> BTreeMap<QualifiedId<FunId>, BTreeSet<NodeId>> {
        let mut result = BTreeMap::new();
        for cond in self.conditions.iter().chain(self.update_map.values()) {
            for exp in cond.all_exps() {
                result.append(&mut exp.called_funs_with_callsites())
            }
        }
        for on_impl in self.on_impl.values() {
            result.append(&mut on_impl.called_funs_with_callsites())
        }
        result
    }

    pub fn visit_positions<F>(&self, visitor: &mut F)
    where
        F: FnMut(VisitorPosition, &ExpData) -> Option<()>,
    {
        let _ = ExpData::visit_positions_spec_impl(self, visitor);
    }

    pub fn visit_post_order<F>(&self, visitor: &mut F)
    where
        F: FnMut(&ExpData),
    {
        self.visit_positions(&mut |pos, exp| {
            if matches!(pos, VisitorPosition::Post) {
                visitor(exp);
            }
            Some(())
        });
    }

    /// Returns the temporaries used in this spec block. Result is ordered by occurrence.
    pub fn used_temporaries_with_types(&self, env: &GlobalEnv) -> Vec<(TempIndex, Type)> {
        let mut temps = vec![];
        let mut visitor = |e: &ExpData| {
            if let ExpData::Temporary(id, idx) = e {
                if !temps.iter().any(|(i, _)| i == idx) {
                    temps.push((*idx, env.get_node_type(*id)));
                }
            }
        };
        self.visit_post_order(&mut visitor);
        temps
    }

    /// Returns the temporaries used in this spec block. Result is ordered by occurrence.
    pub fn used_temporaries(&self) -> BTreeSet<TempIndex> {
        let mut temps = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            if let ExpData::Temporary(_, idx) = e {
                temps.insert(*idx);
            }
        };
        self.visit_post_order(&mut visitor);
        temps
    }
}

/// Information about a specification block in the source. This is used for documentation
/// generation. In the object model, the original locations and documentation of spec blocks
/// is reduced to conditions on a `Spec`, with expansion of schemas. This data structure
/// allows us to discover the original spec blocks and their content.
#[derive(Debug, Clone)]
pub struct SpecBlockInfo {
    /// The location of the entire spec block.
    pub loc: Loc,
    /// The target of the spec block.
    pub target: SpecBlockTarget,
    /// The locations of all members of the spec block.
    pub member_locs: Vec<Loc>,
}

/// Describes the target of a spec block.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecBlockTarget {
    /// The block is associated with the current module.
    Module(ModuleId),
    /// The block is associated with the structure.
    Struct(ModuleId, StructId),
    /// The block is associated with the function.
    Function(ModuleId, FunId),
    /// The block is associated with bytecode of the given function at given code offset.
    FunctionCode(ModuleId, FunId, usize),
    /// The block is associated with a specification schema.
    Schema(ModuleId, SchemaId, Vec<TypeParameter>),
    /// The block is associated with a specification function.
    SpecFunction(ModuleId, SpecFunId),
    /// The block is inline in an expression.
    Inline,
}

/// Describes a global invariant.
#[derive(Debug, Clone)]
pub struct GlobalInvariant {
    pub id: GlobalId,
    pub loc: Loc,
    pub kind: ConditionKind,
    pub mem_usage: BTreeSet<QualifiedInstId<StructId>>,
    pub declaring_module: ModuleId,
    pub properties: PropertyBag,
    pub cond: Exp,
}

// =================================================================================================
/// # Use Declarations

/// Represents a `use` declaration in the source.
#[derive(Debug, Clone)]
pub struct UseDecl {
    /// Location covered by this declaration.
    pub loc: Loc,
    /// The name of the module.
    pub module_name: ModuleName,
    /// The resolved module id, if it is known.
    pub module_id: Option<ModuleId>,
    /// An optional alias assigned to the module.
    pub alias: Option<Symbol>,
    /// A list of member uses, with optional aliasing.
    pub members: Vec<(Loc, Symbol, Option<Symbol>)>,
}

// =================================================================================================
/// # Friend Declarations

/// Represents a `friend` declaration in the source.
#[derive(Debug, Clone)]
pub struct FriendDecl {
    /// Location covered by this declaration.
    pub loc: Loc,
    /// The name of the friend module.
    pub module_name: ModuleName,
    /// The resolved module id, if it is known.
    pub module_id: Option<ModuleId>,
}

// =================================================================================================
/// # Access Specifiers

/// Access specifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AccessSpecifier {
    pub loc: Loc,
    pub kind: AccessSpecifierKind,
    pub negated: bool,
    pub resource: (Loc, ResourceSpecifier),
    pub address: (Loc, AddressSpecifier),
}

impl AccessSpecifier {
    pub fn used_vars(&self) -> Vec<Symbol> {
        match &self.address.1 {
            AddressSpecifier::Call(_, var) | AddressSpecifier::Parameter(var) => {
                vec![*var]
            },
            _ => vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccessSpecifierKind {
    Reads,
    Writes,
    LegacyAcquires,
}

impl AccessSpecifierKind {
    pub fn subsumes(&self, other: &Self) -> bool {
        use AccessSpecifierKind::*;
        matches!((self, other), (_, Reads) | (Writes, Writes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceSpecifier {
    Any,
    DeclaredAtAddress(Address),
    DeclaredInModule(ModuleId),
    Resource(QualifiedInstId<StructId>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AddressSpecifier {
    Any,
    Address(Address),
    Parameter(Symbol),
    Call(QualifiedInstId<FunId>, Symbol),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Hash, Default)]
pub enum LambdaCaptureKind {
    /// No modifier (e.g., inlining)
    #[default]
    Default,
    /// Copy
    Copy,
    /// Move
    Move,
}

impl fmt::Display for LambdaCaptureKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LambdaCaptureKind::Default => {
                write!(f, "")
            },
            LambdaCaptureKind::Copy => {
                write!(f, "copy")
            },
            LambdaCaptureKind::Move => write!(f, "move"),
        }
    }
}

impl ResourceSpecifier {
    /// Checks whether this resource specifier matches the given struct. A function
    /// instantiation is passed to instantiate the specifier in the calling context
    /// of the function where it is declared for.
    pub fn matches(
        &self,
        env: &GlobalEnv,
        fun_inst: &[Type],
        struct_id: &QualifiedInstId<StructId>,
    ) -> bool {
        use ResourceSpecifier::*;
        let struct_env = env.get_struct(struct_id.to_qualified_id());
        match self {
            Any => true,
            DeclaredAtAddress(addr) => struct_env.module_env.get_name().addr() == addr,
            DeclaredInModule(mod_id) => struct_env.module_env.get_id() == *mod_id,
            Resource(spec_struct_id) => {
                // Since this resource specifier is declared for a specific function,
                // need to instantiate it with the function instantiation.
                let spec_struct_id = spec_struct_id.clone().instantiate(fun_inst);
                struct_id.to_qualified_id() == spec_struct_id.to_qualified_id()
                    // If the specified instance has no parameters, every type instance is
                    // allowed, otherwise only the given one.
                    && (spec_struct_id.inst.is_empty() || spec_struct_id.inst == struct_id.inst)
            },
        }
    }

    /// Matches an unqualified struct name. This matches any resource pattern with that name,
    /// regardless of type instantiation.
    pub fn matches_modulo_type_instantiation(
        &self,
        env: &GlobalEnv,
        struct_id: &QualifiedId<StructId>,
    ) -> bool {
        use ResourceSpecifier::*;
        let struct_id = struct_id.instantiate(vec![]);
        match self {
            Resource(spec_struct_id) => Resource(
                // Downgrade to a pattern without instantiation
                spec_struct_id.to_qualified_id().instantiate(vec![]),
            )
            .matches(env, &[], &struct_id),
            _ => self.matches(env, &[], &struct_id),
        }
    }
}

// =================================================================================================
/// # Expressions

/// A type alias for temporaries. Those are locals used in bytecode.
pub type TempIndex = usize;

/// The type of expression data.
///
/// Expression layout follows the following design principles:
///
/// - We try to keep the number of expression variants minimal, for easier treatment in
///   generic traversals. Builtin and user functions are abstracted into a general
///   `Call(.., operation, args)` construct.
/// - Each expression has a unique node id assigned. This id allows to build attribute tables
///   for additional information, like expression type and source location. The id is globally
///   unique.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ExpData {
    /// Represents an invalid expression. This is used as a stub for algorithms which
    /// generate expressions but can fail with multiple errors, like a translator from
    /// some other source into expressions. Consumers of expressions should assume this
    /// variant is not present and can panic when seeing it.
    Invalid(NodeId),
    /// Represents a value.
    Value(NodeId, Value),
    /// Represents a reference to a local variable introduced in the AST.
    LocalVar(NodeId, Symbol),
    /// Represents a reference to a temporary used in bytecode, if this expression is associated
    /// with bytecode.
    /// When compiling from Move source code, represents a parameter to a function: TempIndex
    /// indicates the index into the list of function parameters.
    Temporary(NodeId, TempIndex),
    /// Represents a call to an operation. The `Operation` enum covers all builtin functions
    /// (including operators, constants, ...) as well as user functions.
    Call(NodeId, Operation, Vec<Exp>),
    /// Represents an invocation of a function value, as a lambda.
    Invoke(NodeId, Exp, Vec<Exp>),
    /// Represents a lambda.
    Lambda(
        NodeId,
        Pattern,
        Exp,
        LambdaCaptureKind,
        /// Optional spec block for lambda
        Option<Exp>,
    ),
    /// Represents a quantified formula over multiple variables and ranges.
    Quant(
        NodeId,
        QuantKind,
        /// Ranges
        Vec<(Pattern, Exp)>,
        /// Triggers
        Vec<Vec<Exp>>,
        /// Optional `where` clause
        Option<Exp>,
        /// Body
        Exp,
    ),
    /// Represents a block `Block(id, pattern, optional_binding, scope)` which binds
    /// a pattern, making the bound variables available in scope.
    Block(NodeId, Pattern, Option<Exp>, Exp),
    /// Represents a conditional.
    IfElse(NodeId, Exp, Exp, Exp),
    /// Represents a variant match
    Match(NodeId, Exp, Vec<MatchArm>),

    // ---------------------------------------------------------
    // Subsequent expressions only appear in imperative context
    /// Represents the return from a function
    Return(NodeId, Exp),
    /// Represents a sequence of effects, the last value also being the result.
    Sequence(NodeId, Vec<Exp>),
    /// Represents a loop.
    Loop(NodeId, Exp),
    /// Represents a loop continuation, as in `LoopCont(id, nest, is_continue)`. `nest`
    /// determines how many nesting levels the associated loop is away from the given
    /// expression. For example, `0` means the directly enclosing loop, `1` the
    /// loop enclosing that inner loop, and so on. `is_continue` indicates whether
    /// the loop is continued or broken.
    LoopCont(NodeId, usize, bool),
    /// Assignment to a pattern. Can be a tuple pattern and a tuple expression.  Note that Assign
    /// does *not* introduce new variables; they apparently be introduced by a Block or Lambda, or
    /// as a function formal parameter.
    Assign(NodeId, Pattern, Exp),
    /// Mutation of a lhs reference, as in `*lhs = rhs`.
    Mutate(NodeId, Exp, Exp),
    /// Represents a specification block, type is ().
    SpecBlock(NodeId, Spec),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    pub loc: Loc,
    pub pattern: Pattern,
    pub condition: Option<Exp>,
    pub body: Exp,
}

/// An internalized expression. We do use a wrapper around the underlying internement implementation
/// variant to ensure a unique API (LocalIntern and ArcIntern e.g. differ in the presence of
/// the Copy trait, and by wrapping we effectively remove the Copy from LocalIntern).
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Exp {
    data: LocalIntern<ExpData>,
}

impl AsRef<ExpData> for Exp {
    fn as_ref(&self) -> &ExpData {
        self.data.as_ref()
    }
}

impl Borrow<ExpData> for Exp {
    fn borrow(&self) -> &ExpData {
        self.as_ref()
    }
}

impl Deref for Exp {
    type Target = ExpData;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl Debug for Exp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.data)
    }
}

impl From<ExpData> for Exp {
    fn from(data: ExpData) -> Self {
        Exp {
            data: LocalIntern::new(data),
        }
    }
}

impl From<Exp> for ExpData {
    /// Takes an expression and returns expression data.
    fn from(exp: Exp) -> ExpData {
        exp.as_ref().to_owned()
    }
}

/// Rewrite result
pub enum RewriteResult {
    // A new expression, stopping descending into sub expressions
    Rewritten(Exp),
    // A new expression, descending into sub-expressions of the new one.
    RewrittenAndDescend(Exp),
    // The original expression, descend into sub-expressions
    Unchanged(Exp),
}

/// Visitor position
#[derive(Clone)]
pub enum VisitorPosition {
    Pre,                    // before visiting any subexpressions
    MidMutate,              // after RHS and before LHS of Mutate expression.
    BeforeBody,             // Before body of Block expression.
    BeforeMatchBody(usize), // Before the ith body of a Match arm.
    AfterMatchBody(usize),  // After the ith body of a Match arm.
    BeforeThen,             // Before then clause of IfElse expression.
    BeforeElse,             // Before else clause of IfElse expression.
    PreSequenceValue,       // Before final expr in a Sequence (or before Post, if seq is empty)
    Post,                   // after visiting all subexpressions
}

impl ExpData {
    /// Version of `into` which does not require type annotations.
    pub fn into_exp(self) -> Exp {
        self.into()
    }

    /// Determines whether this is an imperative expression construct
    pub fn is_imperative_construct(&self) -> bool {
        matches!(
            self,
            ExpData::Sequence(_, _)
                | ExpData::Loop(_, _)
                | ExpData::LoopCont(_, _, _)
                | ExpData::Return(_, _)
        )
    }

    pub fn is_directly_borrowable(&self) -> bool {
        use ExpData::*;
        matches!(
            self,
            LocalVar(..) | Temporary(..) | Call(_, Operation::Select(..), _)
        )
    }

    pub fn is_temporary(&self) -> bool {
        use ExpData::*;
        matches!(self, Temporary(..))
    }

    /// Checks for different ways how an unit (void) value is represented. This
    /// can be an empty tuple or an empty sequence.
    pub fn is_unit_exp(&self) -> bool {
        matches!(self, ExpData::Sequence(_, stms) if stms.is_empty())
            || matches!(self, ExpData::Call(_, Operation::Tuple, exps) if exps.is_empty())
    }

    pub fn is_loop_cont(&self, nest: Option<usize>, is_continue: bool) -> bool {
        matches!(self,
            ExpData::LoopCont(_, nest1, is_cont)
            if Some(*nest1) == nest && *is_cont == is_continue)
    }

    pub fn ptr_eq(e1: &Exp, e2: &Exp) -> bool {
        // For the internement based implementations, we can just test equality. Other
        // representations may need different measures.
        e1 == e2
    }

    pub fn node_id(&self) -> NodeId {
        use ExpData::*;
        match self {
            Invalid(node_id)
            | Value(node_id, ..)
            | LocalVar(node_id, ..)
            | Temporary(node_id, ..)
            | Call(node_id, ..)
            | Invoke(node_id, ..)
            | Lambda(node_id, ..)
            | Quant(node_id, ..)
            | Block(node_id, ..)
            | IfElse(node_id, ..)
            | Match(node_id, ..)
            | Sequence(node_id, ..)
            | Loop(node_id, ..)
            | LoopCont(node_id, ..)
            | Return(node_id, ..)
            | Mutate(node_id, ..)
            | Assign(node_id, ..)
            | SpecBlock(node_id, ..) => *node_id,
        }
    }

    pub fn call_args(&self) -> &[Exp] {
        match self {
            ExpData::Call(_, _, args) => args,
            _ => panic!("function must be called on Exp::Call(...)"),
        }
    }

    pub fn node_ids(&self) -> Vec<NodeId> {
        let mut ids = vec![];
        self.visit_post_order(&mut |e| {
            ids.push(e.node_id());
            true // keep going
        });
        ids
    }

    /// Returns the free local variables, inclusive their types, used in this expression.
    /// Result is ordered by occurrence.
    pub fn free_vars_with_types(&self, env: &GlobalEnv) -> Vec<(Symbol, Type)> {
        let mut vars = vec![];
        let var_collector = |id: NodeId, sym: Symbol| {
            if !vars.iter().any(|(s, _)| *s == sym) {
                vars.push((sym, env.get_node_type(id)));
            }
        };
        self.visit_free_local_vars(var_collector);
        vars
    }

    /// Returns the bound local variables with node id in this expression
    pub fn bound_local_vars_with_node_id(&self) -> BTreeMap<Symbol, NodeId> {
        let mut vars = BTreeMap::new();
        let mut visitor = |post: bool, e: &ExpData| {
            use ExpData::*;
            if post {
                if let LocalVar(id, sym) = e {
                    if !vars.iter().any(|(s, _)| s == sym) {
                        vars.insert(*sym, *id);
                    }
                }
            }
            true // keep going
        };
        self.visit_pre_post(&mut visitor);
        vars
    }

    /// Visits free local variables with node id in this expression.
    pub fn visit_free_local_vars<F>(&self, mut node_symbol_visitor: F)
    where
        F: FnMut(NodeId, Symbol),
    {
        fn shadow_or_unshadow_sym(
            sym: &Symbol,
            entering: bool,
            shadow_map: &mut BTreeMap<Symbol, usize>,
        ) {
            if entering {
                shadow_map
                    .entry(*sym)
                    .and_modify(|curr| *curr += 1)
                    .or_insert(1);
            } else if let Some(x) = shadow_map.get_mut(sym) {
                *x -= 1;
            }
        }

        fn for_syms_in_pat_shadow_or_unshadow(
            pat: &Pattern,
            entering: bool,
            shadow_map: &mut BTreeMap<Symbol, usize>,
        ) {
            pat.vars()
                .iter()
                .for_each(|(_, sym)| shadow_or_unshadow_sym(sym, entering, shadow_map))
        }

        fn for_syms_in_ranges_shadow_or_unshadow(
            ranges: &[(Pattern, Exp)],
            entering: bool,
            shadow_map: &mut BTreeMap<Symbol, usize>,
        ) {
            ranges
                .iter()
                .for_each(|(pat, _)| for_syms_in_pat_shadow_or_unshadow(pat, entering, shadow_map));
        }

        fn is_sym_free(sym: &Symbol, shadow_map: &BTreeMap<Symbol, usize>) -> bool {
            shadow_map.get(sym).cloned().unwrap_or(0) == 0
        }

        let mut shadow_map: BTreeMap<Symbol, usize> = BTreeMap::new();
        let mut visitor = |pos: VisitorPosition, e: &ExpData| {
            use ExpData::*;
            use VisitorPosition::*;
            match (e, pos) {
                (Lambda(_, pat, ..), Pre) | (Block(_, pat, _, _), BeforeBody) => {
                    // Add declared variables to shadow; in the Block case,
                    // do it only after processing bindings.
                    for_syms_in_pat_shadow_or_unshadow(pat, true, &mut shadow_map);
                },
                (Lambda(_, pat, ..), Post) | (Block(_, pat, _, _), Post) => {
                    // Remove declared variables from shadow
                    for_syms_in_pat_shadow_or_unshadow(pat, false, &mut shadow_map);
                },
                (Match(_, _, arms), BeforeMatchBody(idx)) => {
                    // Add declared variables to shadow
                    for_syms_in_pat_shadow_or_unshadow(&arms[idx].pattern, true, &mut shadow_map)
                },
                (Match(_, _, arms), AfterMatchBody(idx)) => {
                    for_syms_in_pat_shadow_or_unshadow(&arms[idx].pattern, false, &mut shadow_map)
                },
                (Quant(_, _, ranges, ..), Pre) => {
                    for_syms_in_ranges_shadow_or_unshadow(ranges, true, &mut shadow_map);
                },
                (Quant(_, _, ranges, ..), Post) => {
                    for_syms_in_ranges_shadow_or_unshadow(ranges, false, &mut shadow_map);
                },
                (Assign(_, pat, _), Pre) => {
                    // Visit the Assigned pat vars on the way down, before visiting the RHS expression
                    for (id, sym) in pat.vars().iter() {
                        if is_sym_free(sym, &shadow_map) {
                            node_symbol_visitor(*id, *sym);
                        }
                    }
                },
                (LocalVar(id, sym), Pre) => {
                    if is_sym_free(sym, &shadow_map) {
                        node_symbol_visitor(*id, *sym);
                    }
                },
                _ => {},
            };
            true // keep going
        };
        self.visit_positions(&mut visitor);
    }

    /// Returns just the free local variables in this expression.
    pub fn free_vars(&self) -> BTreeSet<Symbol> {
        let mut vars = BTreeSet::new();
        let just_vars_collector = |_id: NodeId, sym: Symbol| {
            vars.insert(sym);
        };
        self.visit_free_local_vars(just_vars_collector);
        vars
    }

    /// Returns the free local variables and the used parameters in this expression.
    /// Requires that we pass `param_symbols`: an ordered list of all parameter symbols
    /// in the function containing this expression.
    pub fn free_vars_and_used_params(&self, param_symbols: &[Symbol]) -> BTreeSet<Symbol> {
        let mut result = self
            .used_temporaries()
            .into_iter()
            .map(|t| param_symbols[t])
            .collect::<BTreeSet<_>>();
        result.append(&mut self.free_vars());
        result
    }

    /// Returns the used memory of this expression.
    pub fn used_memory(
        &self,
        env: &GlobalEnv,
    ) -> BTreeSet<(QualifiedInstId<StructId>, Option<MemoryLabel>)> {
        let mut result = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            use ExpData::*;
            use Operation::*;
            match e {
                Call(id, Exists(label), _) | Call(id, Global(label), _) => {
                    let inst = &env.get_node_instantiation(*id);
                    let (mid, sid, sinst) = inst[0].require_struct();
                    result.insert((mid.qualified_inst(sid, sinst.to_owned()), label.to_owned()));
                },
                Call(id, SpecFunction(mid, fid, labels), _) => {
                    let inst = &env.get_node_instantiation(*id);
                    let module = env.get_module(*mid);
                    let fun = module.get_spec_fun(*fid);
                    for (i, mem) in fun.used_memory.iter().enumerate() {
                        result.insert((
                            mem.to_owned().instantiate(inst),
                            labels.as_ref().map(|l| l[i]),
                        ));
                    }
                },
                _ => {},
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        result
    }

    /// Returns the directly used memory of this expression, without label.
    pub fn directly_used_memory(&self, env: &GlobalEnv) -> BTreeSet<QualifiedInstId<StructId>> {
        let mut result = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            use ExpData::*;
            use Operation::*;
            match e {
                Call(id, Exists(_), _) | Call(id, Global(_), _) => {
                    let inst = &env.get_node_instantiation(*id);
                    let (mid, sid, sinst) = inst[0].require_struct();
                    result.insert(mid.qualified_inst(sid, sinst.to_owned()));
                },
                _ => {},
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        result
    }

    /// Returns the temporaries used in this expression, with types. Result is ordered by occurrence.
    pub fn used_temporaries_with_types(&self, env: &GlobalEnv) -> Vec<(TempIndex, Type)> {
        self.used_temporaries_with_ids()
            .into_iter()
            .map(|(t, i)| (t, env.get_node_type(i)))
            .collect()
    }

    /// Returns the temporaries used in this expression, together with the node id of their usage.
    pub fn used_temporaries_with_ids(&self) -> Vec<(TempIndex, NodeId)> {
        let mut temps = vec![];
        let mut visitor = |e: &ExpData| {
            if let ExpData::Temporary(id, idx) = e {
                if !temps.iter().any(|(i, _)| i == idx) {
                    temps.push((*idx, *id));
                }
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        temps
    }

    /// Returns the temporaries used in this expression.
    pub fn used_temporaries(&self) -> BTreeSet<TempIndex> {
        let mut temps = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            if let ExpData::Temporary(_, idx) = e {
                temps.insert(*idx);
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        temps
    }

    /// Returns the Move functions referenced by this expression
    pub fn used_funs(&self) -> BTreeSet<QualifiedId<FunId>> {
        let mut used = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            match e {
                ExpData::Call(_, Operation::MoveFunction(mid, fid), _)
                | ExpData::Call(_, Operation::Closure(mid, fid, _), _) => {
                    used.insert(mid.qualified(*fid));
                },
                _ => {},
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        used
    }

    /// Returns the Move functions called or referenced by this expression, along with nodes of call sites or references.
    pub fn used_funs_with_uses(&self) -> BTreeMap<QualifiedId<FunId>, BTreeSet<NodeId>> {
        let mut used: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut visitor = |e: &ExpData| {
            match e {
                ExpData::Call(node_id, Operation::MoveFunction(mid, fid), _)
                | ExpData::Call(node_id, Operation::Closure(mid, fid, _), _) => {
                    used.entry(mid.qualified(*fid))
                        .or_default()
                        .insert(*node_id);
                },
                _ => {},
            };
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        used
    }

    /// Returns the Move functions called by this expression
    pub fn called_funs(&self) -> BTreeSet<QualifiedId<FunId>> {
        let mut called = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            if let ExpData::Call(_, Operation::MoveFunction(mid, fid), _) = e {
                called.insert(mid.qualified(*fid));
            };
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        called
    }

    /// Returns the specification functions called by this expression
    pub fn called_spec_funs(&self, env: &GlobalEnv) -> BTreeSet<QualifiedInstId<SpecFunId>> {
        let mut called = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            #[allow(clippy::single_match)] // may need to extend match in the future
            match e {
                ExpData::Call(id, Operation::SpecFunction(mid, fid, _), _) => {
                    let inst = env.get_node_instantiation(*id);
                    called.insert(mid.qualified_inst(*fid, inst));
                },
                _ => {},
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        called
    }

    /// Returns the Move functions called by this expression, along with nodes of call sites.
    pub fn called_funs_with_callsites(&self) -> BTreeMap<QualifiedId<FunId>, BTreeSet<NodeId>> {
        let mut called: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut visitor = |e: &ExpData| {
            if let ExpData::Call(node_id, Operation::MoveFunction(mid, fid), _) = e {
                called
                    .entry(mid.qualified(*fid))
                    .or_default()
                    .insert(*node_id);
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        called
    }

    /// Returns true if the given expression contains a `continue` or
    /// `break` which refers to a loop in the given `nest_range`.
    /// For example, `branches_to(loop { break }, 1..10)` will return false,
    /// but `branches_to(loop { break }, 0..10)` will return true.
    /// count as exit.
    pub fn branches_to(&self, nest_range: Range<usize>) -> bool {
        let branch_cond = |loop_nest: usize, nest: usize, _: bool| {
            nest >= loop_nest && nest_range.contains(&(nest - loop_nest))
        };
        self.customizable_branches_to(branch_cond)
    }

    /// A customizable version of `branches_to`, allowing to
    /// specify how a `continue` or `break` refers to which loop(s).
    pub fn customizable_branches_to<F>(&self, condition: F) -> bool
    where
        F: Fn(usize, usize, bool) -> bool,
    {
        let mut loop_nest = 0;
        let mut branches = false;
        let mut visitor = |post: bool, e: &ExpData| {
            match e {
                ExpData::Loop(_, _) => {
                    if post {
                        loop_nest -= 1
                    } else {
                        loop_nest += 1
                    }
                },
                ExpData::LoopCont(_, nest, cond) if condition(loop_nest, *nest, *cond) => {
                    branches = true;
                    return false; // found a reference, exit visit early
                },
                _ => {},
            }
            true
        };
        self.visit_pre_post(&mut visitor);
        branches
    }

    /// Compute the bindings of break/continue expressions to the associated loop. This
    /// returns two maps: the first maps loop ids to the ids of the loop-cont statements,
    /// together with whether they are break or continue. The 2nd maps loop-cont ids
    /// to the associated loop ids.
    pub fn compute_loop_bindings(
        &self,
    ) -> (
        BTreeMap<NodeId, BTreeMap<NodeId, bool>>,
        BTreeMap<NodeId, NodeId>,
    ) {
        let mut loop_to_cont = BTreeMap::<NodeId, BTreeMap<NodeId, bool>>::new();
        let mut cont_to_loop = BTreeMap::<NodeId, NodeId>::new();
        let mut loop_stack = vec![];
        let mut visit_binding = |post: bool, exp: &ExpData| {
            use ExpData::*;
            match exp {
                Loop(id, _) => {
                    if !post {
                        loop_to_cont.insert(*id, BTreeMap::new());
                        loop_stack.push(*id);
                    } else {
                        loop_stack.pop().expect("loop stack balanced");
                    }
                },
                LoopCont(id, nest, is_continue) => {
                    if !post && *nest < loop_stack.len() {
                        assert!(
                            *nest < loop_stack.len(),
                            "nest={} out of range for len={}",
                            nest,
                            loop_stack.len()
                        );
                        let loop_id = loop_stack[loop_stack.len() - nest - 1];
                        loop_to_cont
                            .get_mut(&loop_id)
                            .unwrap()
                            .insert(*id, *is_continue);
                        cont_to_loop.insert(*id, loop_id);
                    }
                },
                _ => {},
            }
            true
        };
        self.visit_pre_post(&mut visit_binding);
        (loop_to_cont, cont_to_loop)
    }

    /// Rewrite an expression such that any break/continue nests referring to outer loops
    /// have the given delta added to their nesting. This simulates removing or adding a loop to
    /// the given expression. Nests bound to loops of the given expression are not effected.
    ///
    /// If this is needed elsewhere we can move it out, currently it's a local helper.
    pub fn rewrite_loop_nest(&self, delta: isize) -> Exp {
        LoopNestRewriter {
            loop_depth: 0,
            delta,
        }
        .rewrite_exp(self.clone().into_exp())
    }

    /// Returns true of the given expression is valid for a constant expression.
    /// If not valid, then returns false and adds reasons why not to the argument reasons.
    ///
    /// TODO: this mimics the current allowed expression forms the v1 compiler allows,
    /// but is not documented as such in the book
    pub fn is_valid_for_constant(&self, env: &GlobalEnv, reasons: &mut Vec<(Loc, String)>) -> bool {
        let mut valid = true;
        let mut visitor = |e: &ExpData| {
            match e {
                ExpData::Value(..) | ExpData::Invalid(_) | ExpData::Sequence(_, _) => {},
                ExpData::Call(id, oper, _args) => {
                    // Note that _args are visited separately.  No need to check them here.
                    if !oper.is_builtin_op() {
                        reasons.push((
                            env.get_node_loc(*id),
                            "Invalid call or operation in constant".to_owned(),
                        ));
                        valid = false;
                    }
                },
                _ => {
                    let id = e.node_id();
                    reasons.push((
                        env.get_node_loc(id),
                        "Invalid statement or expression in constant".to_owned(),
                    ));
                    valid = false;
                },
            }
            true // Always keep going, to add all problematic subexpressions to reasons.
        };
        self.visit_pre_order(&mut visitor);
        valid
    }

    /// Visits expression, calling visitor on each sub-expression, depth first.
    /// `visitor` returns false to indicate that visit should stop early.
    pub fn visit_post_order<F>(&self, visitor: &mut F)
    where
        F: FnMut(&ExpData) -> bool,
    {
        self.visit_pre_post(&mut |post, e| {
            if post {
                visitor(e)
            } else {
                true // keep going
            }
        });
    }

    /// Visits expression, calling visitor parent expression, then subexpressions, depth first.
    /// `visitor` returns false to indicate that visit should stop early.
    pub fn visit_pre_order<F>(&self, visitor: &mut F)
    where
        F: FnMut(&ExpData) -> bool,
    {
        self.visit_pre_post(&mut |post, e| {
            if !post {
                visitor(e)
            } else {
                true // keep going
            }
        });
    }

    /// Visits all inline specification blocks in the expression.
    pub fn visit_inline_specs<F>(&self, visitor: &mut F)
    where
        F: FnMut(&Spec) -> bool,
    {
        self.visit_pre_order(&mut |e| {
            if let ExpData::SpecBlock(_, spec) = e {
                visitor(spec)
            } else {
                true
            }
        });
    }

    pub fn any<P>(&self, predicate: &mut P) -> bool
    where
        P: FnMut(&ExpData) -> bool,
    {
        let mut found = false;
        self.visit_pre_order(&mut |e| {
            if predicate(e) {
                found = true;
                false // stop visiting; we're done
            } else {
                true // keep looking
            }
        });
        found
    }

    /// Visits expression, calling visitor on each sub-expression. `visitor(false, ..)` will
    /// be called before descending into expression, and `visitor(true, ..)` after. Notice
    /// we use one function instead of two so a lambda can be passed which encapsulates mutable
    /// references.
    /// - `visitor` returns `false` to indicate that visit should stop early, and `true` to continue.
    pub fn visit_pre_post<F>(&self, visitor: &mut F)
    where
        F: FnMut(bool, &ExpData) -> bool,
    {
        let _ = self.visit_positions_impl(&mut |x, e| {
            use VisitorPosition::*;
            let should_continue = match x {
                Pre => visitor(false, e),
                Post => visitor(true, e),
                MidMutate | BeforeBody | BeforeThen | BeforeElse | BeforeMatchBody(_)
                | AfterMatchBody(_) | PreSequenceValue => true,
            };
            if should_continue {
                Some(())
            } else {
                None
            }
        });
    }

    /// Recursively visits expression, calling visitor for key control points of each sub-expression.
    /// `visitor(Pre, ...)` will be called before descending into each expression, and
    /// `visitor(Post, ...)` will be called after the descent.  For a few expressions,
    /// additional visitor calls will also be made at key control points between recursive
    /// calls:
    /// - for `Mutate(..., lhs, rhs)`, visits `rhs` before `lhs` (following execution control flow),
    ///   with a call to `visitor(MidMutate, ...)` between the two recursive calls
    /// - for `Block(..., binding, body)` visits `binding` before `body`, with a call to
    ///   `visitor(BeforeBody, ...)` between the two recursive calls.
    /// - for `IfElse(..., cond, then, else)` first recursively visits `cond`, then calls
    ///   `visitor(BeforeThen, ...)`, then visits `then`, then calls `visitor(BeforeElse, ...)`,
    ///   then visits `else`.
    ///
    /// In every case, if `visitor` returns `false`, then the visit is stopped early; otherwise
    /// the visit will continue.
    pub fn visit_positions<F>(&self, visitor: &mut F)
    where
        F: FnMut(VisitorPosition, &ExpData) -> bool,
    {
        self.visit_positions_all_visits_return_true(visitor);
    }

    /// Same as `visit_positions`, but returns false iff any visit of a subexpression returns false
    pub fn visit_positions_all_visits_return_true<F>(&self, visitor: &mut F) -> bool
    where
        F: FnMut(VisitorPosition, &ExpData) -> bool,
    {
        self.visit_positions_impl(&mut |x, e| {
            if visitor(x, e) {
                Some(())
            } else {
                None
            }
        })
        .is_some()
    }

    /// Visitor implementation uses `Option<()>` to implement short-cutting without verbosity.
    /// - `visitor` returns `None` to indicate that visit should stop early, and `Some(())` to continue.
    /// - `visit_positions_impl` returns `None` if visitor returned `None`.
    /// See `visit_positions` for more
    fn visit_positions_impl<F>(&self, visitor: &mut F) -> Option<()>
    where
        F: FnMut(VisitorPosition, &ExpData) -> Option<()>,
    {
        use ExpData::*;
        visitor(VisitorPosition::Pre, self)?;
        match self {
            Call(_, _, args) => {
                for exp in args {
                    exp.visit_positions_impl(visitor)?;
                }
            },
            Invoke(_, target, args) => {
                target.visit_positions_impl(visitor)?;
                for exp in args {
                    exp.visit_positions_impl(visitor)?;
                }
            },
            Lambda(_, _, body, _, spec_opt) => {
                body.visit_positions_impl(visitor)?;
                if let Some(spec) = spec_opt {
                    spec.visit_positions_impl(visitor)?;
                }
            },
            Quant(_, _, ranges, triggers, condition, body) => {
                for (_, range) in ranges {
                    range.visit_positions_impl(visitor)?;
                }
                for trigger in triggers {
                    for e in trigger {
                        e.visit_positions_impl(visitor)?;
                    }
                }
                if let Some(exp) = condition {
                    exp.visit_positions_impl(visitor)?;
                }
                body.visit_positions_impl(visitor)?;
            },
            Block(_, _, binding, body) => {
                if let Some(exp) = binding {
                    exp.visit_positions_impl(visitor)?;
                }
                visitor(VisitorPosition::BeforeBody, self)?;
                body.visit_positions_impl(visitor)?;
            },
            IfElse(_, c, t, e) => {
                c.visit_positions_impl(visitor)?;
                visitor(VisitorPosition::BeforeThen, self)?;
                t.visit_positions_impl(visitor)?;
                visitor(VisitorPosition::BeforeElse, self)?;
                e.visit_positions_impl(visitor)?;
            },
            Match(_, d, arms) => {
                d.visit_positions_impl(visitor)?;
                for (i, arm) in arms.iter().enumerate() {
                    visitor(VisitorPosition::BeforeMatchBody(i), self)?;
                    if let Some(c) = &arm.condition {
                        c.visit_positions_impl(visitor)?;
                    }
                    arm.body.visit_positions_impl(visitor)?;
                    visitor(VisitorPosition::AfterMatchBody(i), self)?;
                }
            },
            Loop(_, e) => e.visit_positions_impl(visitor)?,
            Return(_, e) => e.visit_positions_impl(visitor)?,
            Sequence(_, es) => {
                if es.is_empty() {
                    visitor(VisitorPosition::PreSequenceValue, self);
                } else {
                    let last_elt = es.len() - 1;
                    for (i, e) in es.iter().enumerate() {
                        if i == last_elt {
                            visitor(VisitorPosition::PreSequenceValue, self);
                        }
                        e.visit_positions_impl(visitor)?;
                    }
                }
            },
            Assign(_, _, e) => e.visit_positions_impl(visitor)?,
            Mutate(_, lhs, rhs) => {
                rhs.visit_positions_impl(visitor)?;
                visitor(VisitorPosition::MidMutate, self)?;
                lhs.visit_positions_impl(visitor)?;
            },
            SpecBlock(_, spec) => Self::visit_positions_spec_impl(spec, visitor)?,
            // Explicitly list all enum variants
            LoopCont(..) | Value(..) | LocalVar(..) | Temporary(..) | Invalid(..) => {},
        }
        visitor(VisitorPosition::Post, self)
    }

    fn visit_positions_spec_impl<F>(spec: &Spec, visitor: &mut F) -> Option<()>
    where
        F: FnMut(VisitorPosition, &ExpData) -> Option<()>,
    {
        for cond in &spec.conditions {
            Self::visit_positions_cond_impl(cond, visitor)?;
        }
        for impl_spec in spec.on_impl.values() {
            Self::visit_positions_spec_impl(impl_spec, visitor)?;
        }
        for cond in spec.update_map.values() {
            Self::visit_positions_cond_impl(cond, visitor)?;
        }
        for update in spec.update_map.values() {
            Self::visit_positions_cond_impl(update, visitor)?;
        }
        Some(())
    }

    fn visit_positions_cond_impl<F>(cond: &Condition, visitor: &mut F) -> Option<()>
    where
        F: FnMut(VisitorPosition, &ExpData) -> Option<()>,
    {
        cond.exp.visit_positions_impl(visitor)?;
        for exp in &cond.additional_exps {
            exp.visit_positions_impl(visitor)?;
        }
        Some(())
    }

    /// Rewrites this expression and sub-expression based on the rewriter function. The function
    /// returns `RewriteResult:Rewritten(e)` if the expression is rewritten, and passes back
    /// ownership using `RewriteResult:Unchanged(e)` if the expression stays unchanged. This
    /// function stops traversing on `RewriteResult::Rewritten(e)` and descents into sub-expressions
    /// on `RewriteResult::Unchanged(e)`. In order to continue into sub-expressions after rewrite, use
    /// `RewriteResult::RewrittenAndDescend(e)`.
    pub fn rewrite<F>(exp: Exp, exp_rewriter: &mut F) -> Exp
    where
        F: FnMut(Exp) -> RewriteResult,
    {
        ExpRewriter {
            exp_rewriter,
            node_rewriter: &mut |_| None,
            pattern_rewriter: &mut |_, _| None,
        }
        .rewrite_exp(exp)
    }

    pub fn rewrite_exp_and_pattern<F, G>(
        exp: Exp,
        exp_rewriter: &mut F,
        pattern_rewriter: &mut G,
    ) -> Exp
    where
        F: FnMut(Exp) -> RewriteResult,
        G: FnMut(&Pattern, bool) -> Option<Pattern>,
    {
        ExpRewriter {
            exp_rewriter,
            node_rewriter: &mut |_| None,
            pattern_rewriter,
        }
        .rewrite_exp(exp)
    }

    /// Rewrites the node ids in the expression. This is used to rewrite types of
    /// expressions.
    pub fn rewrite_node_id<F>(exp: Exp, node_rewriter: &mut F) -> Exp
    where
        F: FnMut(NodeId) -> Option<NodeId>,
    {
        ExpRewriter {
            exp_rewriter: &mut RewriteResult::Unchanged,
            node_rewriter,
            pattern_rewriter: &mut |_, _| None,
        }
        .rewrite_exp(exp)
    }

    /// Rewrites the expression and for unchanged sub-expressions, the node ids in the expression
    pub fn rewrite_exp_and_node_id<F, G>(
        exp: Exp,
        exp_rewriter: &mut F,
        node_rewriter: &mut G,
    ) -> Exp
    where
        F: FnMut(Exp) -> RewriteResult,
        G: FnMut(NodeId) -> Option<NodeId>,
    {
        ExpRewriter {
            exp_rewriter,
            node_rewriter,
            pattern_rewriter: &mut |_, _| None,
        }
        .rewrite_exp(exp)
    }

    /// A function which can be used by a `node_rewriter` argument to `ExpData::rewrite_node_id` to
    /// instantiate types in an expression based on a type parameter instantiation.
    pub fn instantiate_node(env: &GlobalEnv, id: NodeId, targs: &[Type]) -> Option<NodeId> {
        if targs.is_empty() {
            // shortcut
            return None;
        }
        let node_ty = env.get_node_type(id);
        let new_node_ty = node_ty.instantiate(targs);
        let node_inst = env.get_node_instantiation_opt(id);
        let new_node_inst = node_inst.clone().map(|i| Type::instantiate_vec(i, targs));
        if node_ty != new_node_ty || node_inst != new_node_inst {
            let loc = env.get_node_loc(id);
            let new_id = env.new_node(loc, new_node_ty);
            if let Some(inst) = new_node_inst {
                env.set_node_instantiation(new_id, inst);
            }
            Some(new_id)
        } else {
            None
        }
    }

    /// A function which can be used by a `node_rewriter` argument to `ExpData::rewrite_node_id` to
    /// update node location (`Loc`), in addition to instantiating types.  This is currently only
    /// useful in inlining, but is cleaner to implement here.
    pub fn instantiate_node_new_loc(
        env: &GlobalEnv,
        id: NodeId,
        targs: &[Type],
        new_loc: &Loc,
    ) -> Option<NodeId> {
        let loc = env.get_node_loc(id);
        if loc != *new_loc {
            let node_ty = env.get_node_type(id);
            let new_node_ty = node_ty.instantiate(targs);
            let node_inst = env.get_node_instantiation_opt(id);
            let new_node_inst = node_inst.clone().map(|i| Type::instantiate_vec(i, targs));
            let new_id = env.new_node(new_loc.clone(), new_node_ty);
            if let Some(inst) = new_node_inst {
                env.set_node_instantiation(new_id, inst);
            }
            Some(new_id)
        } else {
            ExpData::instantiate_node(env, id, targs)
        }
    }

    /// Returns the set of module ids used by this expression.
    pub fn module_usage(&self, usage: &mut BTreeSet<ModuleId>) {
        self.visit_post_order(&mut |e| {
            if let ExpData::Call(_, oper, _) = e {
                use Operation::*;
                match oper {
                    SpecFunction(mid, ..)
                    | Pack(mid, ..)
                    | Select(mid, ..)
                    | UpdateField(mid, ..) => {
                        usage.insert(*mid);
                    },
                    _ => {},
                }
            }
            true // keep going
        });
    }

    /// Extract access to ghost memory from expression. Returns a tuple of the instantiated
    /// struct, the field of the selected value, and the expression with the address of the access.
    pub fn extract_ghost_mem_access(
        &self,
        env: &GlobalEnv,
    ) -> Option<(QualifiedInstId<StructId>, FieldId, Exp)> {
        if let ExpData::Call(_, Operation::Select(_, _, field_id), sargs) = self {
            if let ExpData::Call(id, Operation::Global(None), gargs) = sargs[0].as_ref() {
                let ty = &env.get_node_type(*id);
                let (mid, sid, targs) = ty.require_struct();
                if env
                    .symbol_pool()
                    .string(sid.symbol())
                    .starts_with(GHOST_MEMORY_PREFIX)
                {
                    return Some((
                        mid.qualified_inst(sid, targs.to_vec()),
                        *field_id,
                        gargs[0].clone(),
                    ));
                }
            }
        }
        None
    }

    /// Collect struct-related operations
    pub fn struct_usage(&self, usage: &mut BTreeSet<QualifiedId<StructId>>) {
        self.visit_post_order(&mut |e| {
            if let ExpData::Call(_, oper, _) = e {
                use Operation::*;
                match oper {
                    Select(mid, sid, ..) | UpdateField(mid, sid, ..) | Pack(mid, sid, _) => {
                        usage.insert(mid.qualified(*sid));
                    },
                    _ => {},
                }
            }
            true // keep going.
        });
    }

    /// Collect field-related operations
    pub fn field_usage(&self, usage: &mut BTreeSet<(QualifiedId<StructId>, FieldId)>) {
        self.visit_post_order(&mut |e| {
            if let ExpData::Call(_, oper, _) = e {
                use Operation::*;
                match oper {
                    Select(mid, sid, fid) | UpdateField(mid, sid, fid) => {
                        usage.insert((mid.qualified(*sid), *fid));
                    },
                    _ => {},
                }
            }
            true // keep going.
        });
    }

    /// Collect vector-related operations
    pub fn vector_usage(&self, usage: &mut HashSet<Operation>) {
        self.visit_post_order(&mut |e| {
            if let ExpData::Call(_, oper, _) = e {
                use Operation::*;
                match oper {
                    Index | Slice | ConcatVec | EmptyVec | SingleVec | UpdateVec | IndexOfVec
                    | ContainsVec | InRangeVec | RangeVec => {
                        usage.insert(oper.clone());
                    },
                    _ => {},
                }
            }
            true // keep going.
        });
    }

    /// Returns the node id of the inner expression which delivers the result. For blocks,
    /// this traverses into the body.
    pub fn result_node_id(&self) -> NodeId {
        if let ExpData::Block(_, _, _, body) = self {
            body.result_node_id()
        } else {
            self.node_id()
        }
    }
}

struct ExpRewriter<'a> {
    exp_rewriter: &'a mut dyn FnMut(Exp) -> RewriteResult,
    node_rewriter: &'a mut dyn FnMut(NodeId) -> Option<NodeId>,
    pattern_rewriter: &'a mut dyn FnMut(&Pattern, bool) -> Option<Pattern>,
}

impl ExpRewriterFunctions for ExpRewriter<'_> {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        match (*self.exp_rewriter)(exp) {
            RewriteResult::Rewritten(new_exp) => new_exp,
            RewriteResult::RewrittenAndDescend(new_exp) => self.rewrite_exp_descent(new_exp),
            RewriteResult::Unchanged(old_exp) => self.rewrite_exp_descent(old_exp),
        }
    }

    fn rewrite_node_id(&mut self, id: NodeId) -> Option<NodeId> {
        (*self.node_rewriter)(id)
    }

    fn rewrite_pattern(&mut self, pat: &Pattern, entering_scope: bool) -> Option<Pattern> {
        (*self.pattern_rewriter)(pat, entering_scope)
    }
}

/// A rewriter for lifting loop nests.
struct LoopNestRewriter {
    loop_depth: usize,
    delta: isize,
}

impl ExpRewriterFunctions for LoopNestRewriter {
    fn rewrite_exp(&mut self, exp: Exp) -> Exp {
        match exp.as_ref() {
            ExpData::LoopCont(id, nest, cont) if *nest >= self.loop_depth => {
                let new_nest = (*nest as isize) + self.delta;
                assert!(
                    new_nest >= 0,
                    "loop removed which has break/continue references?"
                );
                ExpData::LoopCont(*id, new_nest as usize, *cont).into_exp()
            },
            ExpData::Loop(_, _) => {
                self.loop_depth += 1;
                let result = self.rewrite_exp_descent(exp);
                self.loop_depth -= 1;
                result
            },
            _ => self.rewrite_exp_descent(exp),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    MoveFunction(ModuleId, FunId),
    Pack(ModuleId, StructId, /*variant*/ Option<Symbol>),
    Closure(ModuleId, FunId, ClosureMask),
    Tuple,
    Select(ModuleId, StructId, FieldId),
    SelectVariants(
        ModuleId,
        StructId,
        /* fields from different variants */ Vec<FieldId>,
    ),
    TestVariants(ModuleId, StructId, /* variants */ Vec<Symbol>),

    // Specification specific
    SpecFunction(ModuleId, SpecFunId, Option<Vec<MemoryLabel>>),
    UpdateField(ModuleId, StructId, FieldId),
    Result(usize),
    Index,
    Slice,
    Range,
    Implies,
    Iff,
    Identical,

    // Binary operators
    Add,
    Sub,
    Mul,
    Mod,
    Div,
    BitOr,
    BitAnd,
    Xor,
    Shl,
    Shr,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,

    // Copy and Move
    Copy,
    Move,

    // Unary operators
    Not,
    Cast,

    // Builtin functions (impl and spec)
    Exists(Option<MemoryLabel>),

    // Builtin functions (impl only)
    BorrowGlobal(ReferenceKind),
    Borrow(ReferenceKind),
    Deref,
    MoveTo,
    MoveFrom,
    Freeze(/*explicit*/ bool),
    Abort,
    Vector,

    // Builtin functions (spec only)
    Len,
    TypeValue,
    TypeDomain,
    ResourceDomain,
    Global(Option<MemoryLabel>),
    CanModify,
    Old,
    Trace(TraceKind),

    EmptyVec,
    SingleVec,
    UpdateVec,
    ConcatVec,
    IndexOfVec,
    ContainsVec,
    InRangeRange,
    InRangeVec,
    RangeVec,
    MaxU8,
    MaxU16,
    MaxU32,
    MaxU64,
    MaxU128,
    MaxU256,
    Bv2Int,
    Int2Bv,

    // Functions which support the transformation and translation process.
    AbortFlag,
    AbortCode,
    WellFormed,
    BoxValue,
    UnboxValue,
    EmptyEventStore,
    ExtendEventStore,
    EventStoreIncludes,
    EventStoreIncludedIn,

    // Operation with no effect
    NoOp,
}

/// A label used for referring to a specific memory in Global and Exists expressions.
pub type MemoryLabel = GlobalId;

/// A pattern, either a variable, a tuple, or a struct instantiation applied to a sequence of patterns.
/// Carries a node_id which has (at least) a type and location.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Var(NodeId, Symbol),
    Wildcard(NodeId),
    Tuple(NodeId, Vec<Pattern>),
    Struct(
        // Struct(_, struct_id, optional_variant, patterns)
        NodeId,
        QualifiedInstId<StructId>,
        Option<Symbol>,
        Vec<Pattern>,
    ),
    Error(NodeId),
}

impl Pattern {
    /// Returns the node id of the pattern.
    pub fn node_id(&self) -> NodeId {
        match self {
            Pattern::Var(id, _)
            | Pattern::Wildcard(id)
            | Pattern::Tuple(id, _)
            | Pattern::Struct(id, _, _, _)
            | Pattern::Error(id) => *id,
        }
    }

    /// Returns the variables in this pattern, per node_id and name.
    pub fn vars(&self) -> Vec<(NodeId, Symbol)> {
        let mut result = vec![];
        Self::collect_vars(&mut result, self);
        result
    }

    /// Flatten a pattern: if its a tuple, return the elements, otherwise
    /// make a singleton.
    pub fn flatten(self) -> Vec<Pattern> {
        if let Pattern::Tuple(_, pats) = self {
            pats
        } else {
            vec![self]
        }
    }

    /// Returns true if this pattern is a simple variable or tuple of variables.
    pub fn is_simple_decl(&self) -> bool {
        match self {
            Pattern::Var(..) => true,
            Pattern::Tuple(_, pats) => pats.iter().all(|p| matches!(p, Pattern::Var(..))),
            _ => false,
        }
    }

    /// Returns true if this pattern contains no struct.
    pub fn has_no_struct(&self) -> bool {
        use Pattern::*;
        match self {
            Var(..) | Wildcard(..) | Error(..) => true,
            Tuple(_, pats) => pats.iter().all(|p| p.has_no_struct()),
            Struct(..) => false,
        }
    }

    fn collect_vars(r: &mut Vec<(NodeId, Symbol)>, p: &Pattern) {
        use Pattern::*;
        match p {
            Struct(_, _, _, args) | Tuple(_, args) => {
                for arg in args {
                    Self::collect_vars(r, arg)
                }
            },
            Var(id, name) => r.push((*id, *name)),
            _ => {},
        }
    }

    /// Walks `self` and `exp` in parallel to pair any `Pattern` variables with subexpressions.
    /// If pattern and expression are the same shape (e.g., `Pattern::Struct` on LHS matched with
    /// `Operation::Pack` call on RHS with same `StructId`; `Pattern::Tuple` on LHS matched with
    /// `Value::Tuple` or `Operation::Tuple` call with same arity and types on RHS), then each
    /// `Symbol` in the result will be paired wtih `Some(exp)`.  If shapes differ, then symbols
    /// in `Pattern::Var` subpatterns will be paired with `None` in the result vector.
    pub fn vars_and_exprs(&self, exp: &Exp) -> Vec<(Symbol, Option<Exp>)> {
        let mut result = vec![];
        let _shape_matched = Self::collect_vars_exprs_from_expr(&mut result, self, Some(exp));
        result
    }

    // Implementation of `vars_and_exprs`:
    //
    // Recursively walks `Pattern` `p` and (optional) `Exp` `opt_exp` in parallel to generate a list
    // of pairs in output parameter `r`.
    //
    // Returns true if pattern matches exp
    fn collect_vars_exprs_from_expr(
        r: &mut Vec<(Symbol, Option<Exp>)>,
        p: &Pattern,
        opt_exp: Option<&Exp>,
    ) -> bool {
        use Pattern::*;
        match p {
            Struct(_nodeid, qsid, _, args) => {
                if let Some(exp) = opt_exp {
                    if let ExpData::Call(_, Operation::Pack(modid, sid, _), actuals) = exp.as_ref()
                    {
                        if *sid == qsid.id && *modid == qsid.module_id {
                            Self::collect_vars_exprs_from_vector_exprs(r, args, actuals)
                        } else {
                            Self::collect_vars_exprs_from_vector_none(r, args);
                            false
                        }
                    } else {
                        Self::collect_vars_exprs_from_vector_none(r, args)
                    }
                } else {
                    Self::collect_vars_exprs_from_vector_none(r, args)
                }
            },
            Tuple(_, args) => {
                if let Some(exp) = opt_exp {
                    match exp.as_ref() {
                        ExpData::Value(_, Value::Tuple(actuals)) => {
                            Self::collect_vars_exprs_from_vector_values(r, args, actuals)
                        },
                        ExpData::Call(_, Operation::Tuple, actuals) => {
                            Self::collect_vars_exprs_from_vector_exprs(r, args, actuals)
                        },
                        _ => Self::collect_vars_exprs_from_vector_none(r, args),
                    }
                } else {
                    Self::collect_vars_exprs_from_vector_none(r, args)
                }
            },
            Var(_, name) => match opt_exp {
                Some(exp) => {
                    r.push((*name, Some(exp.clone())));
                    true
                },
                None => {
                    r.push((*name, None));
                    false
                },
            },
            _ => true,
        }
    }

    // Helper function for `vars_and_exprs`, to match variables in a `Pattern` with pieces of a
    // `Value`.  Pieces are extracted as new `Value` expressions as needed to match vars.
    //
    // Recursively walks `Pattern` `p` and optional `Value` `opt_v` in tandem and appends matching
    // pairs in output var `r`.  New `Value` expressions are created as needed to represent pieces
    // of the input `Value`.
    //
    // Returns true if pattern matches value
    fn collect_vars_exprs_from_value(
        r: &mut Vec<(Symbol, Option<Exp>)>,
        p: &Pattern,
        opt_v: Option<&Value>,
    ) -> bool {
        use Pattern::*;
        match p {
            Struct(_nodeid, _qsid, _variant, args) => {
                Self::collect_vars_exprs_from_vector_none(r, args)
            },
            Tuple(_, args) => {
                if let Some(value) = opt_v {
                    match value {
                        Value::Tuple(actuals) => {
                            Self::collect_vars_exprs_from_vector_values(r, args, actuals)
                        },
                        Value::Vector(actuals) => {
                            Self::collect_vars_exprs_from_vector_values(r, args, actuals)
                        },
                        _ => {
                            Self::collect_vars_exprs_from_vector_none(r, args);
                            false
                        },
                    }
                } else {
                    Self::collect_vars_exprs_from_vector_none(r, args);
                    false
                }
            },
            Var(id, name) => {
                if let Some(value) = opt_v {
                    r.push((*name, Some(ExpData::Value(*id, value.clone()).into_exp())));
                    true
                } else {
                    r.push((*name, None));
                    false
                }
            },
            _ => true,
        }
    }

    // Helper function for `vars_and_exprs`, to match a vector of `Pattern` with a vector of `Exp`.
    //
    // Recursively walks `Pattern`s `pats` and `Exp`s `exps` in tandem and appends matching
    // pairs in output var `r`.
    //
    // Returns true if slice sizes match and all patterns match expressions.
    fn collect_vars_exprs_from_vector_exprs(
        r: &mut Vec<(Symbol, Option<Exp>)>,
        pats: &[Pattern],
        exprs: &[Exp],
    ) -> bool {
        pats.iter().zip_longest(exprs.iter()).all(|pair| {
            match pair {
                EitherOrBoth::Both(pat, expr) => {
                    Self::collect_vars_exprs_from_expr(r, pat, Some(expr))
                },
                EitherOrBoth::Left(pat) => Self::collect_vars_exprs_from_expr(r, pat, None),
                EitherOrBoth::Right(_) => {
                    false // there are extra exprs
                },
            }
        })
    }

    // Helper function for `vars_and_exprs`, to match a vector of `Pattern` with a vector of `Value`.
    //
    // Recursively walks `Pattern`s `pats` and `Exp`s `exps` in tandem and appends matching
    // pairs in output var `r`.  New `Value` expressions are created as needed to represent pieces
    // of input `Value`s.
    //
    // Returns true if slice sizes match and all patterns match values.
    fn collect_vars_exprs_from_vector_values(
        r: &mut Vec<(Symbol, Option<Exp>)>,
        pats: &[Pattern],
        vals: &[Value],
    ) -> bool {
        pats.iter().zip_longest(vals.iter()).all(|pair| match pair {
            EitherOrBoth::Both(pat, value) => {
                Self::collect_vars_exprs_from_value(r, pat, Some(value))
            },
            EitherOrBoth::Left(pat) => Self::collect_vars_exprs_from_value(r, pat, None),
            EitherOrBoth::Right(_) => false,
        })
    }

    // Helper function for `vars_and_exprs`, to match a vector of `Pattern` with no binding.
    //
    // Recursively walks `Pattern`s `pats` and appends pairs matching variables in the pattern with `None`
    // in output var `r`.
    //
    // Returns `false` unless the input slice `pats` is empty.
    fn collect_vars_exprs_from_vector_none(
        r: &mut Vec<(Symbol, Option<Exp>)>,
        pats: &[Pattern],
    ) -> bool {
        pats.iter()
            .all(|pat| Self::collect_vars_exprs_from_value(r, pat, None))
    }

    // Returns a new pattern which is a copy of `self` but with
    // each `Var` subpattern contained in `vars` replaced by
    // a `Wildcard` subpattern.
    pub fn remove_vars(self, vars: &BTreeSet<Symbol>) -> Pattern {
        match self {
            Pattern::Var(id, var) => {
                if vars.contains(&var) {
                    Pattern::Wildcard(id)
                } else {
                    Pattern::Var(id, var)
                }
            },
            Pattern::Tuple(id, patvec) => Pattern::Tuple(
                id,
                patvec
                    .into_iter()
                    .map(|pat| pat.remove_vars(vars))
                    .collect(),
            ),
            Pattern::Struct(id, qsid, variant, patvec) => Pattern::Struct(
                id,
                qsid,
                variant,
                patvec
                    .into_iter()
                    .map(|pat| pat.remove_vars(vars))
                    .collect(),
            ),
            Pattern::Error(..) | Pattern::Wildcard(..) => self,
        }
    }

    /// Does a variable substitution on a pattern.
    ///
    /// Calls `var_map` on every symbol `sym` occurring in a `Var` subpattern of `self`, and if any
    /// call returns `Some(sym2)` such that `sym != sym2`, then creates a `clone` of `self` but with
    /// every `sym3` replaced by `sym4` iff `Some(sym4) = var_map(sym3)`.  Otherwise, returns
    /// `None` as there are no substitutions to be done.
    pub fn replace_vars<'a, F>(&self, var_map: &'a F) -> Option<Pattern>
    where
        F: Fn(&Symbol) -> Option<&'a Symbol>,
    {
        match self {
            Pattern::Var(id, var) => {
                if let Some(new_var) = var_map(var) {
                    if new_var != var {
                        Some(Pattern::Var(*id, *new_var))
                    } else {
                        None
                    }
                } else {
                    None
                }
            },
            Pattern::Tuple(_, patvec) | Pattern::Struct(_, _, _, patvec) => {
                let pat_out: Vec<_> = patvec.iter().map(|pat| pat.replace_vars(var_map)).collect();
                if pat_out.iter().any(|opt_pat| opt_pat.is_some()) {
                    // Need to build a new vec.
                    let new_vec: Vec<_> = std::iter::zip(pat_out, patvec)
                        .map(|(opt_pat, pat)| match opt_pat {
                            Some(new_pat) => new_pat,
                            None => pat.clone(),
                        })
                        .collect();
                    match self {
                        Pattern::Tuple(id, _) => Some(Pattern::Tuple(*id, new_vec)),
                        Pattern::Struct(id, qsid, variant, _) => {
                            Some(Pattern::Struct(*id, qsid.clone(), *variant, new_vec))
                        },
                        _ => None,
                    }
                } else {
                    None
                }
            },
            Pattern::Error(..) | Pattern::Wildcard(..) => None,
        }
    }

    /// Visits pattern, calling visitor on each sub-pattern. `visitor(false, ..)` will be called
    /// before descending into recursive pattern, and `visitor(true, ..)` after. Notice we use one
    /// function instead of two so a lambda can be passed which encapsulates mutable references.
    pub fn visit_pre_post<F>(&self, visitor: &mut F)
    where
        F: FnMut(bool, &Pattern),
    {
        use Pattern::*;
        visitor(false, self);
        match self {
            Var(..) | Wildcard(..) | Error(..) => {},
            Tuple(_, patvec) => {
                for pat in patvec {
                    pat.visit_pre_post(visitor);
                }
            },
            Struct(_, _, _, patvec) => {
                for pat in patvec {
                    pat.visit_pre_post(visitor);
                }
            },
        };
        visitor(true, self);
    }

    pub fn to_string(&self, fun_env: &FunctionEnv) -> String {
        PatDisplay {
            env: fun_env.module_env.env,
            pat: self,
            fun_env: Some(fun_env),
            show_type: false,
        }
        .to_string()
    }

    pub fn display<'a>(&'a self, env: &'a GlobalEnv) -> PatDisplay<'a> {
        PatDisplay {
            env,
            pat: self,
            fun_env: None,
            show_type: true,
        }
    }

    pub fn display_cont<'a>(&'a self, other: &PatDisplay<'a>) -> PatDisplay<'a> {
        PatDisplay {
            env: other.env,
            pat: self,
            fun_env: other.fun_env,
            show_type: other.show_type,
        }
    }

    pub fn display_for_exp<'a>(&'a self, other: &ExpDisplay<'a>) -> PatDisplay<'a> {
        PatDisplay {
            env: other.env,
            pat: self,
            fun_env: other.fun_env,
            show_type: true,
        }
    }
}

#[derive(Clone)]
pub struct PatDisplay<'a> {
    env: &'a GlobalEnv,
    pat: &'a Pattern,
    fun_env: Option<&'a FunctionEnv<'a>>,
    show_type: bool,
}

impl PatDisplay<'_> {
    fn set_show_type(self, show_type: bool) -> Self {
        Self { show_type, ..self }
    }

    fn type_ctx(&self) -> TypeDisplayContext {
        if let Some(fe) = &self.fun_env {
            fe.get_type_display_ctx()
        } else {
            TypeDisplayContext::new(self.env)
        }
    }

    fn fmt_patterns(&self, f: &mut Formatter<'_>, patterns: &[Pattern]) -> Result<(), Error> {
        if let Some(first) = patterns.first() {
            first.display_cont(self).fmt_pattern(f)?;
            for pat in patterns.iter().skip(1) {
                write!(f, ", ")?;
                pat.display_cont(self).fmt_pattern(f)?;
            }
        }
        Ok(())
    }

    fn fmt_pattern(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use Pattern::*;
        let node_id = self.pat.node_id();
        let node_type = self.env.get_node_type(node_id);
        let type_ctx = &self.type_ctx();
        let mut showed_type = false;
        match self.pat {
            Var(_, sym) => {
                write!(f, "{}", sym.display(self.env.symbol_pool()))?;
            },
            Wildcard(_) => write!(f, "_")?,
            Tuple(_, pattern_vec) => {
                write!(f, "(")?;
                self.fmt_patterns(f, pattern_vec)?;
                write!(f, ")")?
            },
            Struct(_, struct_qfid, variant, pattern_vec) => {
                let inst_str = if !struct_qfid.inst.is_empty() {
                    format!(
                        "<{}>",
                        struct_qfid
                            .inst
                            .iter()
                            .map(|ty| ty.display(type_ctx))
                            .join(", ")
                    )
                } else {
                    "".to_string()
                };
                let struct_env = self.env.get_struct(struct_qfid.to_qualified_id());
                let field_names = struct_env
                    .get_fields_optional_variant(*variant)
                    .map(|f| f.get_name());
                let pool = self.env.symbol_pool();
                let args_str = if variant.is_some() && pattern_vec.is_empty() {
                    "".to_string()
                } else {
                    format!(
                        "{{ {} }}",
                        pattern_vec
                            .iter()
                            .zip(field_names)
                            .map(|(pat, sym)| {
                                let field_name = pool.string(sym);
                                let pattern_str =
                                    pat.display_cont(self).set_show_type(false).to_string();
                                if &pattern_str != field_name.as_ref() {
                                    format!("{}: {}", field_name.as_ref(), pattern_str)
                                } else {
                                    pattern_str
                                }
                            })
                            .join(", ")
                    )
                };
                write!(
                    f,
                    "{}{}{}{}",
                    struct_env.get_full_name_str(),
                    optional_variant_suffix(pool, variant),
                    inst_str,
                    args_str
                )?;
                showed_type = true
            },
            Error(_) => write!(f, "Pattern::Error")?,
        }
        if self.show_type && !showed_type {
            write!(f, ": {}", node_type.display(type_ctx))
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for PatDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        self.fmt_pattern(f)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum TraceKind {
    /// A user level TRACE(..) in the source.
    User,
    /// An automatically generated trace
    Auto,
    /// A trace for a sub-expression of an assert or assume. The location of a
    /// Call(.., Trace(SubAuto)) expression identifies the context of the assume or assert.
    /// A backend may print those traces only if the assertion failed.
    SubAuto,
}

impl fmt::Display for TraceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use TraceKind::*;
        match self {
            User => f.write_str("user"),
            Auto => f.write_str("auto"),
            SubAuto => f.write_str("subauto"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Value {
    Address(Address),
    Number(BigInt),
    Bool(bool),
    // Note that the following are slightly redundant, and may represent the same values, depending
    // on type (e.g., a `Vector<Vec<Value::Number>>` might be the same as a `ByteArray(Vec<u8>)`,
    // depending on values and types.
    ByteArray(Vec<u8>),
    AddressArray(Vec<Address>), // TODO: merge AddressArray to Vector type in the future
    Vector(Vec<Value>),
    Tuple(Vec<Value>),
}

impl Value {
    /// Implement an equality relation on values which identifies representations which
    /// implement the same runtime value, assuming that types match.
    ///
    /// If `Address` values are symbolic and differ, then no answer can be given.
    pub fn equivalent(&self, other: &Value) -> Option<bool> {
        // For 2 structurally unequal addresses, are they definitely different?
        // We can only be sure if both are numeric.
        let unequal_addresses_equivalent = |a: &Address, b: &Address| {
            if let (Address::Numerical(_), Address::Numerical(_)) = (a, b) {
                Some(false)
            } else {
                None // Symbolic inequality is not definitive.
            }
        };
        let addresses_equivalent = |a: &Address, b: &Address| {
            if a == b {
                Some(true)
            } else {
                unequal_addresses_equivalent(a, b)
            }
        };
        // `Option<bool>::and()` operation that treats `None` as "unknown"
        let fuzzy_and = |a: Option<bool>, b: Option<bool>| match (a, b) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (Some(true), Some(true)) => Some(true),
            _ => None,
        };
        if self != other {
            // Check for a few cases of overlapping/ambiguous representations
            match (self, other) {
                // Symbolic addresses may be incomparable.
                (Value::Address(addr1), Value::Address(addr2)) => {
                    unequal_addresses_equivalent(addr1, addr2)
                },
                (Value::Vector(x), Value::ByteArray(y))
                | (Value::ByteArray(y), Value::Vector(x)) => {
                    if x.len() == y.len() {
                        Some(iter::zip(x, y).all(|(value, byte)| {
                            if let Value::Number(bigint) = value {
                                bigint == &BigInt::from(*byte)
                            } else {
                                false
                            }
                        }))
                    } else {
                        Some(false)
                    }
                },
                (Value::Vector(x), Value::AddressArray(y))
                | (Value::AddressArray(y), Value::Vector(x)) => {
                    if x.len() == y.len() {
                        iter::zip(x, y)
                            .map(|(value, addr2)| {
                                if let Value::Address(addr1) = value {
                                    addresses_equivalent(addr1, addr2)
                                } else {
                                    Some(false)
                                }
                            })
                            .reduce(&fuzzy_and)
                            .unwrap_or(Some(true))
                    } else {
                        Some(false)
                    }
                },
                (Value::AddressArray(x), Value::AddressArray(y)) => {
                    if x.len() == y.len() {
                        iter::zip(x, y)
                            .map(|(addr1, addr2)| addresses_equivalent(addr1, addr2))
                            .reduce(&fuzzy_and)
                            .unwrap_or(Some(true))
                    } else {
                        Some(false)
                    }
                },
                (Value::Vector(x), Value::Vector(y)) | (Value::Tuple(x), Value::Tuple(y)) => {
                    if x.len() == y.len() {
                        iter::zip(x, y)
                            .map(|(val1, val2)| val1.equivalent(val2))
                            .reduce(&fuzzy_and)
                            .unwrap_or(Some(true))
                    } else {
                        Some(false)
                    }
                },
                _ => Some(false),
            }
        } else {
            Some(true)
        }
    }
}

// enables `env.display(&value)`
impl fmt::Display for EnvDisplay<'_, Value> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.val {
            Value::Address(address) => write!(f, "{}", self.env.display(address)),
            Value::Number(int) => write!(f, "{}", int),
            Value::Bool(b) => write!(f, "{}", b),
            // TODO(tzakian): Figure out a better story for byte array displays
            Value::ByteArray(bytes) => write!(f, "{:?}", bytes),
            Value::AddressArray(array) => write!(f, "a{:?}", array),
            Value::Vector(array) => write!(f, "{:?}", array),
            Value::Tuple(array) => write!(f, "({:?})", array),
        }
    }
}

// =================================================================================================
/// # Purity of Expressions

impl Operation {
    /// Determines whether this operation depends on global memory
    pub fn uses_no_memory<F>(&self, check_pure: &F) -> bool
    where
        F: Fn(ModuleId, SpecFunId) -> bool,
    {
        use Operation::*;
        match self {
            Exists(_) | Global(_) => false,
            SpecFunction(mid, fid, _) => check_pure(*mid, *fid),
            _ => true,
        }
    }

    /// Determines whether this is a builtin operator
    pub fn is_builtin_op(&self) -> bool {
        use Operation::*;
        matches!(
            self,
            Tuple | Index | Slice | Range | Implies | Iff | Identical | Not | Cast | Len | Vector
        ) || self.is_binop()
    }

    /// Determines whether this is a binary operator
    pub fn is_binop(&self) -> bool {
        use Operation::*;
        matches!(
            self,
            Add | Sub
                | Mul
                | Mod
                | Div
                | BitOr
                | BitAnd
                | Xor
                | Shl
                | Shr
                | And
                | Or
                | Eq
                | Neq
                | Lt
                | Gt
                | Le
                | Ge
        )
    }

    /// Checks whether an expression calling the operation is OK to remove from code.  This includes
    /// side-effect-free expressions which are not related to Specs, Assertions, and won't generate
    /// errors or warnings in stackless-bytecode passes.
    pub fn is_ok_to_remove_from_code(&self) -> bool {
        use Operation::*;
        match self {
            MoveFunction(..) => false,       // could abort
            SpecFunction(..) => false,       // Spec
            Pack(..) | Closure(..) => false, // Could yield an undroppable value
            Tuple => true,
            Select(..) => false,         // Move-related
            SelectVariants(..) => false, // Move-related
            UpdateField(..) => false,    // Move-related

            // Specification specific
            Result(..) => false, // Spec
            Index => false,      // Spec
            Slice => false,      // Spec
            Range => false,      // Spec
            Implies => false,    // Spec
            Iff => false,        // Spec
            Identical => false,  // Spec

            // Binary operators
            Add => false, // can overflow
            Sub => false, // can overflow
            Mul => false, // can overflow
            Mod => false, // can overflow
            Div => false, // can overflow
            BitOr => true,
            BitAnd => true,
            Xor => true,
            Shl => false, // can overflow
            Shr => false, // can overflow
            And => false, // can overflow
            Or => true,
            Eq => true,
            Neq => true,
            Lt => true,
            Gt => true,
            Le => true,
            Ge => true,

            // Copy and Move
            Copy => false, // Could yield an undroppable value
            Move => false, // Move-related

            // Unary operators
            Not => true,
            Cast => false, // can overflow

            // Builtin functions (impl and spec)
            Exists(..) => false, // Spec

            // Builtin functions (impl only)
            BorrowGlobal(..) => false, // Move-related
            Borrow(..) => false,       // Move-related
            Deref => false,            // Move-related
            MoveTo => false,           // Move-related
            MoveFrom => false,         // Move-related
            Freeze(_) => false,        // Move-related
            Abort => false,            // Move-related
            Vector => false,           // Move-related

            // Builtin functions (spec only)
            Len => false,            // Spec
            TypeValue => false,      // Spec
            TypeDomain => false,     // Spec
            ResourceDomain => false, // Spec
            Global(..) => false,     // Spec
            CanModify => false,      // Spec
            Old => false,            // Spec
            Trace(..) => false,      // Spec

            EmptyVec => false,     // Spec
            SingleVec => false,    // Spec
            UpdateVec => false,    // Spec
            ConcatVec => false,    // Spec
            IndexOfVec => false,   // Spec
            ContainsVec => false,  // Spec
            InRangeRange => false, // Spec
            InRangeVec => false,   // Spec
            RangeVec => false,     // Spec
            MaxU8 => false,        // Spec
            MaxU16 => false,       // Spec
            MaxU32 => false,       // Spec
            MaxU64 => false,       // Spec
            MaxU128 => false,      // Spec
            MaxU256 => false,      // Spec
            Bv2Int => false,       // Spec
            Int2Bv => false,       // Spec

            // Functions which support the transformation and translation process.
            AbortFlag => false,            // Spec
            AbortCode => false,            // Spec
            WellFormed => false,           // Spec
            BoxValue => false,             // Spec
            UnboxValue => false,           // Spec
            EmptyEventStore => false,      // Spec
            ExtendEventStore => false,     // Spec
            EventStoreIncludes => false,   // Spec
            EventStoreIncludedIn => false, // Spec

            // Operation with no effect
            TestVariants(..) => true, // Cannot abort
            NoOp => true,
        }
    }

    /// Whether the operation allows to take reference parameters instead of values. This applies
    /// currently to equality which can be used on `(T, T)`, `(T, &T)`, etc.
    pub fn allows_ref_param_for_value(&self) -> bool {
        matches!(self, Operation::Eq | Operation::Neq)
    }

    /// Get the string representation, if this is a binary operator.
    /// Returns `None` for non-binary operators.
    pub fn to_string_if_binop(&self) -> Option<&'static str> {
        use Operation::*;
        match self {
            Add => Some("+"),
            Sub => Some("-"),
            Mul => Some("*"),
            Mod => Some("%"),
            Div => Some("/"),
            BitOr => Some("|"),
            BitAnd => Some("&"),
            Xor => Some("^"),
            Shl => Some("<<"),
            Shr => Some(">>"),
            And => Some("&&"),
            Or => Some("||"),
            Eq => Some("=="),
            Neq => Some("!="),
            Lt => Some("<"),
            Gt => Some(">"),
            Le => Some("<="),
            Ge => Some(">="),
            _ => None,
        }
    }
}

impl ExpData {
    /// Determines whether this expression depends on global memory
    pub fn uses_no_memory<F>(&self, check_pure: &F) -> bool
    where
        F: Fn(ModuleId, SpecFunId) -> bool,
    {
        use ExpData::*;
        let mut no_use = true;
        self.visit_pre_order(&mut |exp: &ExpData| {
            if let Call(_, oper, _) = exp {
                if !oper.uses_no_memory(check_pure) {
                    no_use = false;
                    false // we're done, stop visiting
                } else {
                    true // keep looking
                }
            } else {
                true // keep looking
            }
        });
        no_use
    }
}

impl ExpData {
    /// Checks whether the expression is pure, i.e. does not depend on memory or mutable
    /// variables.
    pub fn is_pure(&self, env: &GlobalEnv) -> bool {
        let mut is_pure = true;
        let mut visitor = |e: &ExpData| {
            use ExpData::*;
            use Operation::*;
            match e {
                Temporary(id, _) => {
                    if env.get_node_type(*id).is_mutable_reference() {
                        is_pure = false;
                        return false; // done visiting
                    }
                },
                Call(_, oper, _) => match oper {
                    Exists(..) | Global(..) => is_pure = false,
                    SpecFunction(mid, fid, _) => {
                        let module = env.get_module(*mid);
                        let fun = module.get_spec_fun(*fid);
                        if !fun.used_memory.is_empty() {
                            is_pure = false;
                            return false; // done visiting
                        }
                    },
                    _ => {},
                },
                _ => {},
            }
            true // keep going
        };
        self.visit_pre_order(&mut visitor);
        is_pure
    }

    /// Checks whether the expression is OK to remove from code.  This includes
    /// side-effect-free expressions which are not related to Specs, Assertions,
    /// and won't generate errors or warnings in stackless-bytecode passes.
    pub fn is_ok_to_remove_from_code(&self) -> bool {
        let mut is_pure = true;
        let mut pure_stack = Vec::new();
        let mut visitor = |post: bool, e: &ExpData| {
            use ExpData::*;
            match e {
                Invalid(..) => {
                    // leave it alone to produce better errors.
                    is_pure = false;
                },
                Value(..) => {}, // Ok, keep going
                LocalVar(..) | Temporary(..) => {
                    // Use of a var could affect borrow semantics, so we cannot
                    // remove uses until borrow analysis produces warnings about user code
                    is_pure = false;
                },
                Call(_, oper, _) => {
                    if !oper.is_ok_to_remove_from_code() {
                        is_pure = false;
                    }
                },
                Invoke(..) => {
                    // Leave it alone for now, but with more analysis maybe we can do something.
                    is_pure = false;
                },
                Lambda(..) => {
                    // Lambda captures any side-effects.
                    if !post {
                        pure_stack.push(is_pure);
                    } else {
                        is_pure = pure_stack.pop().expect("unbalanced");
                    }
                },
                Quant(..) => {
                    // Technically pure, but we don't want to eliminate it.
                    is_pure = false;
                },
                Block(..) | IfElse(..) | Match(..) => {}, // depends on contents
                Return(..) => {
                    is_pure = false;
                },
                Sequence(..) => {}, // depends on contents
                Loop(..) | LoopCont(..) | Assign(..) | Mutate(..) => {
                    is_pure = false;
                },
                SpecBlock(..) => {
                    // Technically pure, but we don't want to eliminate it.
                    is_pure = false;
                },
            }
            true
        };
        self.visit_pre_post(&mut visitor);
        is_pure
    }
}

// =================================================================================================
/// # Names

/// Represents an account address, which can be either numerical or a symbol
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub enum Address {
    Numerical(AccountAddress),
    Symbolic(Symbol),
}

impl Address {
    pub fn from_hex(mut s: &str) -> anyhow::Result<Address> {
        if s.starts_with("0x") {
            s = &s[2..]
        }
        let addr = AccountAddress::from_hex_literal(s).map(Address::Numerical)?;
        Ok(addr)
    }

    pub fn expect_numerical(&self) -> AccountAddress {
        if let Address::Numerical(a) = self {
            *a
        } else {
            panic!("expected numerical address, found symbolic")
        }
    }

    pub fn is_one(&self) -> bool {
        matches!(self, Address::Numerical(AccountAddress::ONE))
    }
}

// enables `env.display(address)`
impl fmt::Display for EnvDisplay<'_, Address> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.val {
            Address::Numerical(addr) => write!(f, "0x{}", addr.short_str_lossless()),
            Address::Symbolic(sym) => write!(f, "{}", sym.display(self.env.symbol_pool())),
        }
    }
}

/// Represents a module name, consisting of address and name.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct ModuleName(Address, Symbol);

impl ModuleName {
    pub fn new(addr: Address, name: Symbol) -> ModuleName {
        ModuleName(addr, name)
    }

    pub fn from_address_bytes_and_name(
        addr: legacy_move_compiler::shared::NumericalAddress,
        name: Symbol,
    ) -> ModuleName {
        ModuleName(Address::Numerical(addr.into_inner()), name)
    }

    pub fn from_str(addr: &str, name: Symbol) -> ModuleName {
        let addr = if !addr.starts_with("0x") {
            AccountAddress::from_hex_literal(&format!("0x{}", addr))
        } else {
            AccountAddress::from_hex_literal(addr)
        };
        ModuleName(Address::Numerical(addr.expect("valid address")), name)
    }

    pub fn addr(&self) -> &Address {
        &self.0
    }

    pub fn name(&self) -> Symbol {
        self.1
    }

    pub fn pseudo_script_name_builder(base: &str, index: usize) -> String {
        format!("{}_{}", base, index)
    }

    /// Return the pseudo module name used for scripts, incorporating the `index`.
    /// Our compiler infrastructure uses `MAX_ADDRESS` for pseudo modules created from scripts.
    pub fn pseudo_script_name(pool: &SymbolPool, index: usize) -> ModuleName {
        let name = pool.make(Self::pseudo_script_name_builder(SCRIPT_MODULE_NAME, index).as_str());
        ModuleName(Address::Numerical(AccountAddress::MAX_ADDRESS), name)
    }

    /// Determine whether this is a script.
    pub fn is_script(&self) -> bool {
        self.0 == Address::Numerical(AccountAddress::MAX_ADDRESS)
    }
}

impl ModuleName {
    /// Creates a value implementing the Display trait which shows this name,
    /// excluding address.
    pub fn display<'a>(&'a self, env: &'a GlobalEnv) -> ModuleNameDisplay<'a> {
        ModuleNameDisplay {
            name: self,
            env,
            with_address: false,
        }
    }

    /// Creates a value implementing the Display trait which shows this name,
    /// including address.
    pub fn display_full<'a>(&'a self, env: &'a GlobalEnv) -> ModuleNameDisplay<'a> {
        ModuleNameDisplay {
            name: self,
            env,
            with_address: true,
        }
    }
}

/// A helper to support module names in formatting.
pub struct ModuleNameDisplay<'a> {
    name: &'a ModuleName,
    env: &'a GlobalEnv,
    with_address: bool,
}

impl fmt::Display for ModuleNameDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.with_address && !self.name.is_script() {
            write!(f, "{}::", self.env.display(&self.name.0))?
        }
        write!(f, "{}", self.name.1.display(self.env.symbol_pool()))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
pub struct QualifiedSymbol {
    pub module_name: ModuleName,
    pub symbol: Symbol,
}

impl QualifiedSymbol {
    /// Creates a value implementing the Display trait which shows this symbol,
    /// including module name but excluding address.
    pub fn display<'a>(&'a self, env: &'a GlobalEnv) -> QualifiedSymbolDisplay<'a> {
        QualifiedSymbolDisplay {
            sym: self,
            env,
            with_module: true,
            with_address: false,
        }
    }

    /// Creates a value implementing the Display trait which shows this qualified symbol,
    /// excluding module name.
    pub fn display_simple<'a>(&'a self, env: &'a GlobalEnv) -> QualifiedSymbolDisplay<'a> {
        QualifiedSymbolDisplay {
            sym: self,
            env,
            with_module: false,
            with_address: false,
        }
    }

    /// Creates a value implementing the Display trait which shows this symbol,
    /// including module name with address.
    pub fn display_full<'a>(&'a self, env: &'a GlobalEnv) -> QualifiedSymbolDisplay<'a> {
        QualifiedSymbolDisplay {
            sym: self,
            env,
            with_module: true,
            with_address: true,
        }
    }
}

/// A helper to support qualified symbols in formatting.
pub struct QualifiedSymbolDisplay<'a> {
    sym: &'a QualifiedSymbol,
    env: &'a GlobalEnv,
    with_module: bool,
    with_address: bool,
}

impl fmt::Display for QualifiedSymbolDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.with_module {
            write!(
                f,
                "{}::",
                if self.with_address {
                    self.sym.module_name.display_full(self.env)
                } else {
                    self.sym.module_name.display(self.env)
                }
            )?;
        }
        write!(f, "{}", self.sym.symbol.display(self.env.symbol_pool()))?;
        Ok(())
    }
}

impl ExpData {
    /// Creates a display of an expression which can be used in formatting.
    pub fn display<'a>(&'a self, env: &'a GlobalEnv) -> ExpDisplay<'a> {
        ExpDisplay {
            env,
            exp: self,
            fun_env: None,
            verbose: false,
            annotator: None,
            tctx: Either::Left(TypeDisplayContext::new(env)),
        }
    }

    /// Creates a display of an expression which can be used in formatting, based
    /// on a function env for getting names of locals and type parameters.
    pub fn display_for_fun<'a>(&'a self, fun_env: &'a FunctionEnv<'a>) -> ExpDisplay<'a> {
        let tctx = Either::Left(fun_env.get_type_display_ctx());
        ExpDisplay {
            env: fun_env.module_env.env,
            exp: self,
            fun_env: Some(fun_env),
            verbose: false,
            annotator: None,
            tctx,
        }
    }

    fn display_cont<'a>(&'a self, other: &'a ExpDisplay<'a>) -> ExpDisplay<'a> {
        ExpDisplay {
            env: other.env,
            exp: self,
            fun_env: other.fun_env,
            verbose: other.verbose,
            annotator: other.annotator,
            tctx: Either::Right(other.get_tctx()),
        }
    }

    pub fn display_verbose<'a>(&'a self, env: &'a GlobalEnv) -> ExpDisplay<'a> {
        ExpDisplay {
            env,
            exp: self,
            fun_env: None,
            verbose: true,
            annotator: None,
            tctx: Either::Left(TypeDisplayContext::new(env)),
        }
    }

    pub fn display_with_annotator<'a, F>(
        &'a self,
        env: &'a GlobalEnv,
        annotator: &'a F,
    ) -> ExpDisplay<'a>
    where
        F: Fn(NodeId) -> String,
    {
        ExpDisplay {
            env,
            exp: self,
            fun_env: None,
            verbose: false,
            annotator: Some(annotator),
            tctx: Either::Left(TypeDisplayContext::new(env)),
        }
    }
}

/// Helper type for expression display.
pub struct ExpDisplay<'a> {
    env: &'a GlobalEnv,
    exp: &'a ExpData,
    fun_env: Option<&'a FunctionEnv<'a>>,
    verbose: bool,
    annotator: Option<&'a dyn Fn(NodeId) -> String>,
    tctx: Either<TypeDisplayContext<'a>, &'a TypeDisplayContext<'a>>,
}

impl<'a> ExpDisplay<'a> {
    fn get_tctx(&'a self) -> &'a TypeDisplayContext<'a> {
        match &self.tctx {
            Either::Left(tctx) => tctx,
            Either::Right(tctx_ref) => tctx_ref,
        }
    }
}

impl fmt::Display for ExpDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use ExpData::*;
        if self.verbose {
            let node_id = self.exp.node_id();
            write!(f, "{}:(", node_id.as_usize())?;
        }
        if let Some(an) = &self.annotator {
            let node_id = self.exp.node_id();
            let s = (*an)(node_id);
            if !s.is_empty() {
                write!(f, "{{{}}} ", s)?;
            }
        }
        match self.exp {
            Invalid(_) => write!(f, "*invalid*"),
            Value(_, v) => write!(f, "{}", self.env.display(v)),
            LocalVar(_, name) => {
                write!(f, "{}", name.display(self.env.symbol_pool()))
            },
            Temporary(_, idx) => {
                if let Some(name) = self
                    .fun_env
                    .as_ref()
                    .and_then(|fe| fe.get_parameters().get(*idx).map(|p| p.0))
                {
                    if self.verbose {
                        write!(f, "$t{}={}", idx, name.display(self.env.symbol_pool()))
                    } else {
                        write!(f, "{}", name.display(self.env.symbol_pool()))
                    }
                } else {
                    write!(f, "$t{}", idx)
                }
            },
            Call(node_id, oper, args) => {
                write!(
                    f,
                    "{}({})",
                    oper.display_for_exp(self, *node_id),
                    self.fmt_exps(args)
                )
            },
            Lambda(id, pat, body, capture_kind, spec_opt) => {
                if self.verbose {
                    write!(
                        f,
                        "{}: {}{}|{}| {}",
                        id.as_usize(),
                        if *capture_kind != LambdaCaptureKind::Default {
                            " "
                        } else {
                            ""
                        },
                        capture_kind,
                        pat.display_for_exp(self),
                        body.display_cont(self)
                    )?;
                } else {
                    write!(
                        f,
                        "{}{}|{}| {}",
                        if *capture_kind != LambdaCaptureKind::Default {
                            " "
                        } else {
                            ""
                        },
                        capture_kind,
                        pat.display_for_exp(self),
                        body.display_cont(self)
                    )?;
                }
                if let Some(spec) = spec_opt {
                    write!(f, "{}", spec.display_cont(self))?;
                }
                Ok(())
            },
            Block(id, pat, binding, body) => {
                if self.verbose {
                    write!(
                        f,
                        "{{\n  {}: let {}{};\n  {}\n}}",
                        indent(id.as_usize()),
                        indent(pat.display_for_exp(self)),
                        if let Some(exp) = binding {
                            indent(format!(" = {}", exp.display_cont(self)))
                        } else {
                            "".to_string()
                        },
                        indent(body.display_cont(self))
                    )
                } else {
                    write!(
                        f,
                        "{{\n  let {}{};\n  {}\n}}",
                        indent(pat.display_for_exp(self)),
                        if let Some(exp) = binding {
                            indent(format!(" = {}", exp.display_cont(self)))
                        } else {
                            "".to_string()
                        },
                        indent(body.display_cont(self))
                    )
                }
            },
            Quant(_, kind, ranges, triggers, opt_where, body) => {
                let triggers_str = triggers
                    .iter()
                    .map(|trigger| format!("{{{}}}", self.fmt_exps(trigger)))
                    .collect_vec()
                    .join("");
                let where_str = if let Some(exp) = opt_where {
                    format!(" where {}", exp.display_cont(self))
                } else {
                    "".to_string()
                };
                write!(
                    f,
                    "{} {}{}{}: {}",
                    kind,
                    self.fmt_quant_ranges(ranges),
                    triggers_str,
                    where_str,
                    body.display_cont(self)
                )
            },
            Invoke(_, fun, args) => {
                write!(f, "({})({})", fun.display_cont(self), self.fmt_exps(args))
            },
            IfElse(_, cond, if_exp, else_exp) => {
                // Special case `if (c) simple_exp`
                match (if_exp.as_ref(), else_exp.as_ref()) {
                    (e, Sequence(_, stms)) if !matches!(e, Sequence(..)) && stms.is_empty() => {
                        write!(
                            f,
                            "if ({}) {}",
                            cond.display_cont(self),
                            if_exp.display_cont(self)
                        )
                    },
                    (_, Sequence(_, stms)) if stms.is_empty() => {
                        write!(
                            f,
                            "if {} {{\n  {}\n}}",
                            cond.display_cont(self),
                            indent(if_exp.display_cont(self)),
                        )
                    },
                    _ => {
                        write!(
                            f,
                            "if {} {{\n  {}\n}} else {{\n  {}\n}}",
                            cond.display_cont(self),
                            indent(if_exp.display_cont(self)),
                            indent(else_exp.display_cont(self))
                        )
                    },
                }
            },
            Match(_, discriminator, arms) => {
                writeln!(f, "match ({}) {{", discriminator.display_cont(self))?;
                for arm in arms {
                    write!(f, "  {}", indent(arm.pattern.display_for_exp(self)))?;
                    if let Some(c) = &arm.condition {
                        write!(f, " if {}", c.display_cont(self))?
                    }
                    writeln!(f, " => {{")?;
                    writeln!(f, "    {}", indent(indent(arm.body.display_cont(self))))?;
                    writeln!(f, "  }}")?
                }
                writeln!(f, "}}")
            },
            Sequence(_, es) => {
                for (i, e) in es.iter().enumerate() {
                    if i > 0 {
                        writeln!(f, ";")?
                    }
                    write!(f, "{}", e.display_cont(self))?
                }
                Ok(())
            },
            Loop(_, e) => {
                write!(f, "loop {{\n  {}\n}}", indent(e.display_cont(self)))
            },
            LoopCont(_, nest, continues) => {
                write!(
                    f,
                    "{}{}",
                    if *continues { "continue" } else { "break" },
                    if *nest > 0 {
                        format!("[{}]", nest)
                    } else {
                        "".to_string()
                    }
                )
            },
            Return(_, e) => write!(f, "return {}", e.display_cont(self)),
            Assign(_, lhs, rhs) => {
                write!(
                    f,
                    "{} = {}",
                    lhs.display_for_exp(self),
                    rhs.display_cont(self)
                )
            },
            Mutate(_, lhs, rhs) => {
                write!(f, "{} = {}", lhs.display_cont(self), rhs.display_cont(self))
            },
            SpecBlock(_, spec) => {
                write!(f, "{}", self.env.display(spec))
            },
        }?;
        if self.verbose {
            let node_id = self.exp.node_id();
            let node_type = self.env.get_node_type(node_id);
            let type_ctx = self.type_ctx();
            write!(f, ") : {}", node_type.display(&type_ctx))
        } else {
            Ok(())
        }
    }
}

fn indent(fmt: impl fmt::Display) -> String {
    let s = fmt.to_string();
    s.replace('\n', "\n  ")
}

impl ExpDisplay<'_> {
    fn type_ctx(&self) -> TypeDisplayContext {
        if let Some(fe) = &self.fun_env {
            fe.get_type_display_ctx()
        } else {
            TypeDisplayContext::new(self.env)
        }
    }

    fn fmt_quant_ranges(&self, ranges: &[(Pattern, Exp)]) -> String {
        ranges
            .iter()
            .map(|(pat, domain)| {
                format!(
                    "{}: {}",
                    pat.display_for_exp(self),
                    domain.display_cont(self)
                )
            })
            .join(", ")
    }

    fn fmt_exps(&self, exps: &[Exp]) -> String {
        exps.iter()
            .map(|e| e.display_cont(self).to_string())
            .join(", ")
    }
}

impl Operation {
    fn display_with_context<'a>(
        &'a self,
        env: &'a GlobalEnv,
        node_id: NodeId,
        tctx: TypeDisplayContext<'a>,
    ) -> OperationDisplay<'a> {
        OperationDisplay {
            env,
            oper: self,
            node_id,
            tctx: Either::Left(tctx),
        }
    }

    fn display_with_context_ref<'a>(
        &'a self,
        env: &'a GlobalEnv,
        node_id: NodeId,
        tctx: &'a TypeDisplayContext<'a>,
    ) -> OperationDisplay<'a> {
        OperationDisplay {
            env,
            oper: self,
            node_id,
            tctx: Either::Right(tctx),
        }
    }

    /// Creates a display of an operation which can be used in formatting.
    pub fn display<'a>(&'a self, env: &'a GlobalEnv, node_id: NodeId) -> OperationDisplay<'a> {
        self.display_with_context(env, node_id, env.get_type_display_ctx())
    }

    /// Creates a display of an operation using the type display ctx from the function.
    pub fn display_with_fun_env<'a>(
        &'a self,
        env: &'a GlobalEnv,
        fun_env: &'a FunctionEnv,
        node_id: NodeId,
    ) -> OperationDisplay<'a> {
        self.display_with_context(env, node_id, fun_env.get_type_display_ctx())
    }

    fn display_for_exp<'a>(
        &'a self,
        exp_display: &'a ExpDisplay,
        node_id: NodeId,
    ) -> OperationDisplay<'a> {
        let tctx = exp_display.get_tctx();
        self.display_with_context_ref(exp_display.env, node_id, tctx)
    }
}

/// Helper type for operation display.
pub struct OperationDisplay<'a> {
    env: &'a GlobalEnv,
    node_id: NodeId,
    oper: &'a Operation,
    tctx: Either<TypeDisplayContext<'a>, &'a TypeDisplayContext<'a>>,
}

impl<'a> OperationDisplay<'a> {
    fn get_tctx(&'a self) -> &'a TypeDisplayContext<'a> {
        match &self.tctx {
            Either::Left(tctx) => tctx,
            Either::Right(tctx_ref) => tctx_ref,
        }
    }
}

impl fmt::Display for OperationDisplay<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use Operation::*;
        match self.oper {
            Cast => {
                let ty = self.env.get_node_type(self.node_id);
                write!(f, "{:?}<{}>", self.oper, ty.display(self.get_tctx()))
            },
            SpecFunction(mid, fid, labels_opt) => {
                write!(f, "{}", self.fun_str(mid, fid))?;
                if let Some(labels) = labels_opt {
                    write!(
                        f,
                        "[{}]",
                        labels.iter().map(|l| format!("{}", l)).join(", ")
                    )?;
                }
                Ok(())
            },
            MoveFunction(mid, fid) | Closure(mid, fid, _) => {
                let prefix = if let Closure(_, _, mask) = self.oper {
                    format!("closure#{}", mask)
                } else {
                    "".to_string()
                };
                write!(
                    f,
                    "{}{}",
                    prefix,
                    self.env
                        .get_function_opt(mid.qualified(*fid))
                        .map(|fun| fun.get_full_name_str())
                        .unwrap_or_else(|| "<?unknown function?>".to_string())
                )
            },
            Global(label_opt) => {
                write!(f, "global")?;
                if let Some(label) = label_opt {
                    write!(f, "[{}]", label)?
                }
                Ok(())
            },
            Exists(label_opt) => {
                write!(f, "exists")?;
                if let Some(label) = label_opt {
                    write!(f, "[{}]", label)?
                }
                Ok(())
            },
            Pack(mid, sid, variant) => write!(
                f,
                "pack {}{}",
                self.struct_str(mid, sid),
                optional_variant_suffix(self.env.symbol_pool(), variant)
            ),
            Select(mid, sid, fid) => {
                write!(f, "select {}", self.field_str(mid, sid, fid))
            },
            SelectVariants(mid, sid, fids) => {
                write!(
                    f,
                    "select_variants {}",
                    fids.iter()
                        .map(|fid| self.field_str(mid, sid, fid))
                        .join("|")
                )
            },
            TestVariants(mid, sid, variants) => {
                write!(
                    f,
                    "test_variants {}::{}",
                    self.struct_str(mid, sid),
                    variants
                        .iter()
                        .map(|v| v.display(self.env.symbol_pool()).to_string())
                        .join("|")
                )
            },
            UpdateField(mid, sid, fid) => {
                write!(f, "update {}", self.field_str(mid, sid, fid))
            },
            Result(t) => write!(f, "result{}", t),
            _ => write!(f, "{:?}", self.oper),
        }?;

        // If operation has a type instantiation, add it.
        let type_inst = self.env.get_node_instantiation(self.node_id);
        if !type_inst.is_empty() {
            write!(
                f,
                "<{}>",
                type_inst
                    .iter()
                    .map(|ty| ty.display(self.get_tctx()))
                    .join(", ")
            )?;
        }
        Ok(())
    }
}

impl OperationDisplay<'_> {
    fn fun_str(&self, mid: &ModuleId, fid: &SpecFunId) -> String {
        let module_env = self.env.get_module(*mid);
        let fun = module_env.get_spec_fun(*fid);
        format!(
            "{}::{}",
            module_env.get_name().display(self.env),
            fun.name.display(self.env.symbol_pool()),
        )
    }

    fn struct_str(&self, mid: &ModuleId, sid: &StructId) -> String {
        let module_env_opt = self.env.get_module_opt(*mid);
        let struct_env_str = module_env_opt
            .clone()
            .map(|module_env| {
                module_env
                    .get_struct(*sid)
                    .get_name()
                    .display(self.env.symbol_pool())
                    .to_string()
            })
            .unwrap_or_else(|| "None".to_string());
        format!(
            "{}::{}",
            module_env_opt
                .map(|module_env| module_env.get_name().display(self.env).to_string())
                .unwrap_or_else(|| "None".to_string()),
            struct_env_str
        )
    }

    fn field_str(&self, mid: &ModuleId, sid: &StructId, fid: &FieldId) -> String {
        let field_name = fid.symbol();
        format!(
            "{}.{}",
            self.struct_str(mid, sid),
            field_name.display(self.env.symbol_pool())
        )
    }
}

impl fmt::Display for MemoryLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "@{}", self.as_usize())
    }
}

impl fmt::Display for EnvDisplay<'_, Condition> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.val.kind {
            ConditionKind::LetPre(name, _loc) => write!(
                f,
                "let {} = {};",
                name.display(self.env.symbol_pool()),
                self.val.exp.display(self.env)
            )?,
            ConditionKind::LetPost(name, _loc) => write!(
                f,
                "let post {} = {};",
                name.display(self.env.symbol_pool()),
                self.val.exp.display(self.env)
            )?,
            ConditionKind::Emits => {
                let exps = self.val.all_exps().collect_vec();
                write!(
                    f,
                    "emit {} to {}",
                    exps[0].display(self.env),
                    exps[1].display(self.env)
                )?;
                if exps.len() > 2 {
                    write!(f, " if {}", exps[2].display(self.env))?;
                }
                write!(f, ";")?
            },
            ConditionKind::Update => write!(
                f,
                "update {} = {};",
                self.val.additional_exps[0].display(self.env),
                self.val.exp.display(self.env)
            )?,
            _ => write!(f, "{} {};", self.val.kind, self.val.exp.display(self.env))?,
        }
        Ok(())
    }
}

impl fmt::Display for EnvDisplay<'_, Spec> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "spec {{")?;
        for cond in &self.val.conditions {
            writeln!(f, "  {}", self.env.display(cond))?
        }
        writeln!(f, "}}")?;
        for (code_offset, spec) in &self.val.on_impl {
            writeln!(f, "{} -> {}", code_offset, self.env.display(spec))?
        }
        Ok(())
    }
}

fn optional_variant_suffix(pool: &SymbolPool, variant: &Option<Symbol>) -> String {
    if let Some(v) = variant {
        format!("::{}", v.display(pool))
    } else {
        String::new()
    }
}

impl fmt::Display for AccessSpecifierKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            AccessSpecifierKind::Reads => f.write_str("reads"),
            AccessSpecifierKind::Writes => f.write_str("writes"),
            AccessSpecifierKind::LegacyAcquires => f.write_str("acquires"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{Address, Value},
        symbol::Symbol,
        AccountAddress,
    };
    use num::BigInt;
    #[test]
    fn test_value_equivalence() {
        // Some test values
        let v_true = Value::Bool(true);
        let v_false = Value::Bool(false);

        let v_3 = Value::Number(BigInt::from(3));
        let v_5 = Value::Number(BigInt::from(5));
        let v_1000 = Value::Number(BigInt::from(1000));

        let bv_empty = Value::ByteArray(vec![]);
        let bv_3_5 = Value::ByteArray(vec![3, 5]);
        let bv_5_3 = Value::ByteArray(vec![5, 3]);
        let bv_3_5_5 = Value::ByteArray(vec![3, 5, 5]);

        let addr_01 =
            Address::Numerical(AccountAddress::from_hex_literal("0xcafebeef").expect("success"));
        let addr_02 =
            Address::Numerical(AccountAddress::from_hex_literal("0x01").expect("success"));
        let addr_s1 = Address::Symbolic(Symbol::new(1));
        let addr_s2 = Address::Symbolic(Symbol::new(2));
        let av_01 = Value::Address(addr_01.clone());
        let av_02 = Value::Address(addr_02.clone());
        let av_s1 = Value::Address(addr_s1.clone());
        let av_s2 = Value::Address(addr_s2.clone());

        let av_empty = Value::AddressArray(vec![]);
        let av_a1_a2 = Value::AddressArray(vec![addr_01.clone(), addr_02.clone()]);
        let av_a2_a1 = Value::AddressArray(vec![addr_02.clone(), addr_01.clone()]);
        let av_a1_a2_a2 =
            Value::AddressArray(vec![addr_01.clone(), addr_02.clone(), addr_02.clone()]);

        let av_s1_s2 = Value::AddressArray(vec![addr_s1.clone(), addr_s2.clone()]);
        let av_s2_s1 = Value::AddressArray(vec![addr_s2.clone(), addr_s1.clone()]);
        let av_s1_s2_s2 =
            Value::AddressArray(vec![addr_s1.clone(), addr_s2.clone(), addr_s2.clone()]);
        let av_a1_s1_a2 =
            Value::AddressArray(vec![addr_01.clone(), addr_s1.clone(), addr_02.clone()]);
        let av_a1_s2_a2 =
            Value::AddressArray(vec![addr_01.clone(), addr_s2.clone(), addr_02.clone()]);
        let av_a2_s1_a1 =
            Value::AddressArray(vec![addr_02.clone(), addr_s1.clone(), addr_01.clone()]);
        let av_a2_s2_a1 =
            Value::AddressArray(vec![addr_02.clone(), addr_s2.clone(), addr_01.clone()]);
        let av_s1_a1_s2 =
            Value::AddressArray(vec![addr_s1.clone(), addr_01.clone(), addr_s2.clone()]);
        let av_s1_a2_s2 =
            Value::AddressArray(vec![addr_s1.clone(), addr_02.clone(), addr_s1.clone()]);

        let vect_empty = Value::Vector(vec![]);
        let vect_3_5 = Value::Vector(vec![v_3.clone(), v_5.clone()]);
        let vect_5_3 = Value::Vector(vec![v_5.clone(), v_3.clone()]);
        let vect_3_5_5 = Value::Vector(vec![v_3.clone(), v_5.clone(), v_5.clone()]);

        let vect_a1_a2 = Value::Vector(vec![av_01.clone(), av_02.clone()]);
        let vect_a2_a1 = Value::Vector(vec![av_02.clone(), av_01.clone()]);
        let vect_a1_a2_a2 = Value::Vector(vec![av_01.clone(), av_02.clone(), av_02.clone()]);
        let vect_s1_s2 = Value::Vector(vec![av_s1.clone(), av_s2.clone()]);
        let vect_s2_s1 = Value::Vector(vec![av_s2.clone(), av_s1.clone()]);

        let vect_s1_s2_s2 = Value::Vector(vec![av_s1.clone(), av_s2.clone(), av_s2.clone()]);
        let vect_a1_s1_a2 = Value::Vector(vec![av_01.clone(), av_s1.clone(), av_02.clone()]);
        let vect_a1_s2_a2 = Value::Vector(vec![av_01.clone(), av_s2.clone(), av_02.clone()]);
        let vect_a2_s1_a1 = Value::Vector(vec![av_02.clone(), av_s1.clone(), av_01.clone()]);
        let vect_a2_s2_a1 = Value::Vector(vec![av_02.clone(), av_s2.clone(), av_01.clone()]);
        let vect_s1_a1_s2 = Value::Vector(vec![av_s1.clone(), av_01.clone(), av_s2.clone()]);
        let vect_s1_a2_s2 = Value::Vector(vec![av_s1.clone(), av_02.clone(), av_s2.clone()]);

        // Each of these should be different from the others.
        let distinct_entities = vec![
            &v_true,
            &v_false,
            &v_3,
            &v_5,
            &v_1000,
            &av_01,
            &av_02,
            &av_a1_a2,
            &av_a2_a1,
            &av_a1_a2_a2,
            &vect_3_5,
            &vect_5_3,
            &vect_3_5_5,
        ];

        for val1 in &distinct_entities {
            for val2 in &distinct_entities {
                assert!(Some(val1 == val2) == val1.equivalent(val2));
                assert!(Some(val1 == val2) == val2.equivalent(val1));
            }
        }

        // These entities are equivalent, although not equal.
        let overlapping_entities = vec![
            (&bv_empty, &vect_empty),
            (&bv_3_5, &vect_3_5),
            (&bv_5_3, &vect_5_3),
            (&bv_3_5_5, &vect_3_5_5),
            (&av_empty, &vect_empty),
            (&av_a1_a2, &vect_a1_a2),
            (&av_a2_a1, &vect_a2_a1),
            (&av_a1_a2_a2, &vect_a1_a2_a2),
        ];

        for (val1, val2) in &overlapping_entities {
            assert!(val1.equivalent(val2) == Some(true));
            assert!(val2.equivalent(val1) == Some(true));
        }

        // With symbolic addresses, things are not always clear.

        // symbolic AddressArrays of different lengths are distinct, as are AddressArrays pairs that
        // have some distinct numerical address element pairs.
        let symbolic_entities_distinct = vec![
            (&av_s1, &av_s1_s2),
            (&av_s1_s2, &av_s1_s2_s2),
            (&av_s1_a1_s2, &av_s1_a2_s2),
            (&av_s1, &av_empty),
            (&av_s1_s2, &av_empty),
            (&av_a1_s1_a2, &av_a2_s1_a1),
            (&av_a1_s1_a2, &av_a2_s2_a1),
            (&av_s1, &vect_s1_s2),
            (&av_s1_s2, &vect_s1_s2_s2),
            (&av_s1_a1_s2, &vect_s1_a2_s2),
            (&av_s1, &vect_empty),
            (&av_s1_s2, &vect_empty),
            (&av_a1_s1_a2, &vect_a2_s1_a1),
            (&av_a1_s1_a2, &vect_a2_s2_a1),
            (&vect_s1_s2, &vect_s1_s2_s2),
            (&vect_s1_a1_s2, &vect_s1_a2_s2),
            (&vect_s1_s2, &vect_empty),
            (&vect_a1_s1_a2, &vect_a2_s1_a1),
            (&vect_a1_s1_a2, &vect_a2_s2_a1),
        ];

        for (val1, val2) in &symbolic_entities_distinct {
            assert!(val1.equivalent(val2) == Some(false));
            assert!(val2.equivalent(val1) == Some(false));
        }

        let ambiguous_pairs = vec![
            (&av_s1_s2, &av_s2_s1),
            (&av_s1_s2, &av_a1_a2),
            (&av_a1_s1_a2, &av_a1_s2_a2),
            (&av_a1_s1_a2, &av_a1_a2_a2),
            (&av_s1_s2, &vect_s2_s1),
            (&av_s1_s2, &vect_a1_a2),
            (&av_a1_s1_a2, &vect_a1_s2_a2),
            (&av_a1_s1_a2, &vect_a1_a2_a2),
            (&vect_s1_s2, &vect_s2_s1),
            (&vect_s1_s2, &vect_a1_a2),
            (&vect_a1_s1_a2, &vect_a1_s2_a2),
            (&vect_a1_s1_a2, &vect_a1_a2_a2),
        ];

        for (val1, val2) in &ambiguous_pairs {
            assert!(val1.equivalent(val2).is_none());
            assert!(val2.equivalent(val1).is_none());
        }

        let symbolic_examples = vec![
            &av_s1,
            &av_s2,
            &av_a1_a2,
            &av_a2_a1,
            &av_a1_a2_a2,
            &av_s1_s2,
            &av_s2_s1,
            &av_s1_s2_s2,
            &av_a1_s1_a2,
            &av_a1_s2_a2,
            &av_a2_s1_a1,
            &av_s1_a1_s2,
            &av_s1_a2_s2,
            &vect_s1_s2,
            &vect_s2_s1,
            &vect_s1_s2_s2,
            &vect_a1_s1_a2,
            &vect_a1_s2_a2,
            &vect_a2_s1_a1,
            &vect_s1_a1_s2,
            &vect_s1_a2_s2,
        ];

        for val1 in &symbolic_examples {
            assert!(val1.equivalent(&((*val1).clone())) == Some(true));
        }
    }
}
