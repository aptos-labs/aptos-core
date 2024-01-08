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
use internment::LocalIntern;
use itertools::Itertools;
use move_binary_format::{
    file_format,
    file_format::{CodeOffset, Visibility},
};
use move_core_types::account_address::AccountAddress;
use num::BigInt;
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashSet},
    fmt,
    fmt::{Debug, Error, Formatter},
    hash::Hash,
    ops::Deref,
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
    pub callees: BTreeSet<QualifiedId<SpecFunId>>,
    pub is_recursive: RefCell<Option<bool>>,
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

impl std::fmt::Display for ConditionKind {
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

impl std::fmt::Display for QuantKind {
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
    Module,
    Struct(ModuleId, StructId),
    Function(ModuleId, FunId),
    FunctionCode(ModuleId, FunId, usize),
    Schema(ModuleId, SchemaId, Vec<TypeParameter>),
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
    pub kind: file_format::AccessKind,
    pub negated: bool,
    pub resource: (Loc, ResourceSpecifier),
    pub address: (Loc, AddressSpecifier),
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
    Lambda(NodeId, Pattern, Exp),
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

    // ---------------------------------------------------------
    // Subsequent expressions only appear in imperative context
    /// Represents the return from a function
    Return(NodeId, Exp),
    /// Represents a sequence of effects, the last value also being the result.
    Sequence(NodeId, Vec<Exp>),
    /// Represents a loop, with a body expression.
    Loop(NodeId, Exp),
    /// Represents a loop continuation for the enclosing loop. The bool indicates whether the
    /// loop is continued (true) or broken (false).
    LoopCont(NodeId, bool),
    /// Assignment to a pattern. Can be a tuple pattern and a tuple expression.  Note that Assign
    /// does *not* introduce new variables; they apparently be introduced by a Block or Lambda, or
    /// as a function formal parameter.
    Assign(NodeId, Pattern, Exp),
    /// Mutation of a lhs reference, as in `*lhs = rhs`.
    Mutate(NodeId, Exp, Exp),
    /// Represents a specification block, type is ().
    SpecBlock(NodeId, Spec),
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
                | ExpData::LoopCont(_, _)
                | ExpData::Return(_, _)
        )
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
        let var_collector = |id: &NodeId, sym: &Symbol| {
            if !vars.iter().any(|(s, _)| s == sym) {
                vars.push((*sym, env.get_node_type(*id)));
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
    fn visit_free_local_vars<F>(&self, mut node_symbol_visitor: F)
    where
        F: FnMut(&NodeId, &Symbol),
    {
        let mut shadowed: BTreeMap<Symbol, usize> = BTreeMap::new();
        let mut visitor = |post: bool, e: &ExpData| {
            use ExpData::*;
            let decls = match e {
                Lambda(_, pat, _) | Block(_, pat, _, _) => {
                    pat.vars().iter().map(|(_, d)| *d).collect_vec()
                },
                Quant(_, _, ranges, ..) => ranges
                    .iter()
                    .flat_map(|(pat, _)| pat.vars().into_iter().map(|(_, name)| name))
                    .collect_vec(),
                _ => vec![],
            };
            if !post {
                // Visit the Assigned pat on the way down, before visiting the RHS expression
                if let Assign(_, pat, _) = e {
                    for (id, sym) in pat.vars().iter() {
                        if shadowed.get(sym).cloned().unwrap_or(0) == 0 {
                            node_symbol_visitor(id, sym);
                        }
                    }
                } else {
                    for sym in &decls {
                        shadowed
                            .entry(*sym)
                            .and_modify(|curr| *curr += 1)
                            .or_insert(1);
                    }
                }
            }
            if post {
                if let LocalVar(id, sym) = e {
                    if shadowed.get(sym).cloned().unwrap_or(0) == 0 {
                        node_symbol_visitor(id, sym);
                    }
                } else {
                    for sym in &decls {
                        if let Some(x) = shadowed.get_mut(sym) {
                            *x -= 1;
                        }
                    }
                }
            }
            true // keep going
        };
        self.visit_pre_post(&mut visitor);
    }

    /// Returns just the free local variables in this expression.
    pub fn free_vars(&self) -> BTreeSet<Symbol> {
        let mut vars = BTreeSet::new();
        let just_vars_collector = |_id: &NodeId, sym: &Symbol| {
            vars.insert(*sym);
        };
        self.visit_free_local_vars(just_vars_collector);
        vars
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

    /// Returns the temporaries used in this expression. Result is ordered by occurrence.
    pub fn used_temporaries(&self, env: &GlobalEnv) -> Vec<(TempIndex, Type)> {
        let mut temps = vec![];
        let mut visitor = |e: &ExpData| {
            if let ExpData::Temporary(id, idx) = e {
                if !temps.iter().any(|(i, _)| i == idx) {
                    temps.push((*idx, env.get_node_type(*id)));
                }
            }
            true // keep going
        };
        self.visit_post_order(&mut visitor);
        temps
    }

    /// Returns the Move functions called by this expression
    pub fn called_funs(&self) -> BTreeSet<QualifiedId<FunId>> {
        let mut called = BTreeSet::new();
        let mut visitor = |e: &ExpData| {
            if let ExpData::Call(_, Operation::MoveFunction(mid, fid), _) = e {
                called.insert(mid.qualified(*fid));
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

    /// Given that this expression is (part of) a loop body, returns `true` if
    /// there is an early exit from the body of the nearest enclosing loop,
    /// i.e., the expression contains a `continue` or `break` statement outside
    /// of any nested loop.
    pub fn has_loop_exit(&self) -> bool {
        let mut loop_count = 0; // Count internal nested loops.
        let mut has_exit = false;
        let mut visitor = |post: bool, e: &ExpData| {
            match e {
                ExpData::Loop(_, _) => loop_count += if post { -1 } else { 1 },
                ExpData::LoopCont(_, _) if loop_count == 0 => {
                    has_exit = true;
                    return false; // found an exit, exit visit early
                },
                _ => {},
            }
            true
        };
        self.visit_pre_post(&mut visitor);
        has_exit
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
        let _ = self.visit_pre_post_impl(&mut |x, e| if visitor(x, e) { Some(()) } else { None });
    }

    /// Visitor implementation uses `Option<()>` to implement short-cutting without verbosity.
    /// - `visitor` returns `None` to indicate that visit should stop early, and `Some(())` to continue.
    /// - `visit_pre_post` returns `None` if visitor returned `None`.
    fn visit_pre_post_impl<F>(&self, visitor: &mut F) -> Option<()>
    where
        F: FnMut(bool, &ExpData) -> Option<()>,
    {
        use ExpData::*;
        visitor(false, self)?;
        match self {
            Call(_, _, args) => {
                for exp in args {
                    exp.visit_pre_post_impl(visitor)?;
                }
            },
            Invoke(_, target, args) => {
                target.visit_pre_post_impl(visitor)?;
                for exp in args {
                    exp.visit_pre_post_impl(visitor)?;
                }
            },
            Lambda(_, _, body) => body.visit_pre_post_impl(visitor)?,
            Quant(_, _, ranges, triggers, condition, body) => {
                for (_, range) in ranges {
                    range.visit_pre_post_impl(visitor)?;
                }
                for trigger in triggers {
                    for e in trigger {
                        e.visit_pre_post_impl(visitor)?;
                    }
                }
                if let Some(exp) = condition {
                    exp.visit_pre_post_impl(visitor)?;
                }
                body.visit_pre_post_impl(visitor)?;
            },
            Block(_, _, binding, body) => {
                if let Some(exp) = binding {
                    exp.visit_pre_post_impl(visitor)?;
                }
                body.visit_pre_post_impl(visitor)?;
            },
            IfElse(_, c, t, e) => {
                c.visit_pre_post_impl(visitor)?;
                t.visit_pre_post_impl(visitor)?;
                e.visit_pre_post_impl(visitor)?;
            },
            Loop(_, e) => e.visit_pre_post_impl(visitor)?,
            Return(_, e) => e.visit_pre_post_impl(visitor)?,
            Sequence(_, es) => {
                for e in es {
                    e.visit_pre_post_impl(visitor)?;
                }
            },
            Assign(_, _, e) => e.visit_pre_post_impl(visitor)?,
            Mutate(_, lhs, rhs) => {
                lhs.visit_pre_post_impl(visitor)?;
                rhs.visit_pre_post_impl(visitor)?;
            },
            SpecBlock(_, spec) => Self::visit_pre_post_spec_impl(spec, visitor)?,
            // Explicitly list all enum variants
            LoopCont(..) | Value(..) | LocalVar(..) | Temporary(..) | Invalid(..) => {},
        }
        visitor(true, self)
    }

    fn visit_pre_post_spec_impl<F>(spec: &Spec, visitor: &mut F) -> Option<()>
    where
        F: FnMut(bool, &ExpData) -> Option<()>,
    {
        for cond in &spec.conditions {
            Self::visit_pre_post_cond_impl(cond, visitor)?;
        }
        for impl_spec in spec.on_impl.values() {
            Self::visit_pre_post_spec_impl(impl_spec, visitor)?;
        }
        for cond in spec.update_map.values() {
            Self::visit_pre_post_cond_impl(cond, visitor)?;
        }
        Some(())
    }

    fn visit_pre_post_cond_impl<F>(cond: &Condition, visitor: &mut F) -> Option<()>
    where
        F: FnMut(bool, &ExpData) -> Option<()>,
    {
        cond.exp.visit_pre_post_impl(visitor)?;
        for exp in &cond.additional_exps {
            exp.visit_pre_post_impl(visitor)?;
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
                    Select(mid, sid, ..) | UpdateField(mid, sid, ..) | Pack(mid, sid) => {
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

impl<'a> ExpRewriterFunctions for ExpRewriter<'a> {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    MoveFunction(ModuleId, FunId),
    SpecFunction(ModuleId, SpecFunId, Option<Vec<MemoryLabel>>),
    Pack(ModuleId, StructId),
    Tuple,

    // Specification specific
    Select(ModuleId, StructId, FieldId),
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
    Freeze,
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
    Struct(NodeId, QualifiedInstId<StructId>, Vec<Pattern>),
    Error(NodeId),
}

impl Pattern {
    /// Returns the node id of the pattern.
    pub fn node_id(&self) -> NodeId {
        match self {
            Pattern::Var(id, _)
            | Pattern::Wildcard(id)
            | Pattern::Tuple(id, _)
            | Pattern::Struct(id, _, _)
            | Pattern::Error(id) => *id,
        }
    }

    /// Returns the variables in this pattern, per node_id and name.
    pub fn vars(&self) -> Vec<(NodeId, Symbol)> {
        let mut result = vec![];
        Self::collect_vars(&mut result, self);
        result
    }

    /// Returns true if this pattern is a simple variable or tuple of variables.
    pub fn is_simple_decl(&self) -> bool {
        match self {
            Pattern::Var(..) => true,
            Pattern::Tuple(_, pats) => pats.iter().all(|p| matches!(p, Pattern::Var(..))),
            _ => false,
        }
    }

    fn collect_vars(r: &mut Vec<(NodeId, Symbol)>, p: &Pattern) {
        use Pattern::*;
        match p {
            Struct(_, _, args) | Tuple(_, args) => {
                for arg in args {
                    Self::collect_vars(r, arg)
                }
            },
            Var(id, name) => r.push((*id, *name)),
            _ => {},
        }
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
    ByteArray(Vec<u8>),
    AddressArray(Vec<Address>), // TODO: merge AddressArray to Vector type in the future
    Vector(Vec<Value>),
}

// enables `env.display(&value)`
impl<'a> fmt::Display for EnvDisplay<'a, Value> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self.val {
            Value::Address(address) => write!(f, "{}", self.env.display(address)),
            Value::Number(int) => write!(f, "{}", int),
            Value::Bool(b) => write!(f, "{}", b),
            // TODO(tzakian): Figure out a better story for byte array displays
            Value::ByteArray(bytes) => write!(f, "{:?}", bytes),
            Value::AddressArray(array) => write!(f, "{:?}", array),
            Value::Vector(array) => write!(f, "{:?}", array),
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
            Tuple
                | Index
                | Slice
                | Range
                | Implies
                | Iff
                | Identical
                | Add
                | Sub
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
                | Not
                | Cast
                | Len
                | Vector
        )
    }

    /// Whether the operation allows to take reference parameters instead of values. This applies
    /// currently to equality which can be used on `(T, T)`, `(T, &T)`, etc.
    pub fn allows_ref_param_for_value(&self) -> bool {
        matches!(self, Operation::Eq | Operation::Neq)
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
}

// enables `env.display(address)`
impl<'a> fmt::Display for EnvDisplay<'a, Address> {
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
        addr: move_compiler::shared::NumericalAddress,
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

    /// Return the pseudo module name used for scripts, incorporating the `index`.
    /// Our compiler infrastructure uses `MAX_ADDRESS` for pseudo modules created from scripts.
    pub fn pseudo_script_name(pool: &SymbolPool, index: usize) -> ModuleName {
        let name = pool.make(format!("{}_{}", SCRIPT_MODULE_NAME, index).as_str());
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

impl<'a> fmt::Display for ModuleNameDisplay<'a> {
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

impl<'a> fmt::Display for QualifiedSymbolDisplay<'a> {
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
        }
    }

    /// Creates a display of an expression which can be used in formatting, based
    /// on a function env for getting names of locals and type parameters.
    pub fn display_for_fun<'a>(&'a self, fun_env: FunctionEnv<'a>) -> ExpDisplay<'a> {
        ExpDisplay {
            env: fun_env.module_env.env,
            exp: self,
            fun_env: Some(fun_env),
            verbose: false,
        }
    }

    fn display_cont<'a>(&'a self, other: &ExpDisplay<'a>) -> ExpDisplay<'a> {
        ExpDisplay {
            env: other.env,
            exp: self,
            fun_env: other.fun_env.clone(),
            verbose: other.verbose,
        }
    }

    #[allow(unused)]
    pub fn display_verbose<'a>(&'a self, env: &'a GlobalEnv) -> ExpDisplay<'a> {
        ExpDisplay {
            env,
            exp: self,
            fun_env: None,
            verbose: true,
        }
    }
}

/// Helper type for expression display.
pub struct ExpDisplay<'a> {
    env: &'a GlobalEnv,
    exp: &'a ExpData,
    fun_env: Option<FunctionEnv<'a>>,
    verbose: bool,
}

impl<'a> fmt::Display for ExpDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use ExpData::*;
        if self.verbose {
            write!(f, "(")?;
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
                    write!(f, "{}", name.display(self.env.symbol_pool()))
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
            Lambda(_, pat, body) => {
                write!(f, "|{}| {}", self.fmt_pattern(pat), body.display_cont(self))
            },
            Block(_, pat, binding, body) => {
                write!(
                    f,
                    "{{\n  let {}{};\n  {}\n}}",
                    indent(self.fmt_pattern(pat)),
                    if let Some(exp) = binding {
                        indent(format!(" = {}", exp.display_cont(self)))
                    } else {
                        "".to_string()
                    },
                    indent(body.display_cont(self))
                )
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
                write!(
                    f,
                    "if {} {{\n  {}\n}} else {{\n  {}\n}}",
                    cond.display_cont(self),
                    indent(if_exp.display_cont(self)),
                    indent(else_exp.display_cont(self))
                )
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
            LoopCont(_, true) => write!(f, "continue"),
            LoopCont(_, false) => write!(f, "break"),
            Return(_, e) => write!(f, "return {}", e.display_cont(self)),
            Assign(_, lhs, rhs) => {
                write!(f, "{} = {}", self.fmt_pattern(lhs), rhs.display_cont(self))
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

impl<'a> ExpDisplay<'a> {
    fn type_ctx(&self) -> TypeDisplayContext<'a> {
        if let Some(fe) = &self.fun_env {
            fe.get_type_display_ctx()
        } else {
            TypeDisplayContext::new(self.env)
        }
    }

    fn fmt_patterns(&self, patterns: &[Pattern]) -> String {
        patterns.iter().map(|pat| self.fmt_pattern(pat)).join(", ")
    }

    pub fn fmt_pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Var(id, name) => {
                let tctx = self.type_ctx();
                let ty = self.env.get_node_type(*id);
                format!(
                    "{}: {}",
                    name.display(self.env.symbol_pool()),
                    ty.display(&tctx)
                )
            },
            Pattern::Tuple(_, args) => format!("({})", self.fmt_patterns(args)),
            Pattern::Struct(_, struct_id, args) => {
                let tctx = self.type_ctx();
                let inst_str = if !struct_id.inst.is_empty() {
                    format!(
                        "<{}>",
                        struct_id.inst.iter().map(|ty| ty.display(&tctx)).join(", ")
                    )
                } else {
                    "".to_string()
                };
                let struct_env = self.env.get_struct(struct_id.to_qualified_id());
                let field_names = struct_env.get_fields().map(|f| f.get_name());
                let args_str = args
                    .iter()
                    .zip(field_names)
                    .map(|(pat, sym)| {
                        let field_name = self.env.symbol_pool().string(sym);
                        let pattern_str = self.fmt_pattern(pat);
                        if &pattern_str != field_name.as_ref() {
                            format!("{}: {}", field_name.as_ref(), self.fmt_pattern(pat))
                        } else {
                            pattern_str
                        }
                    })
                    .join(", ");
                format!(
                    "{}{}{{ {} }}",
                    struct_env.get_full_name_str(),
                    inst_str,
                    args_str
                )
            },
            Pattern::Wildcard(_) => "_".to_string(),
            Pattern::Error(_) => "<error>".to_string(),
        }
    }

    fn fmt_quant_ranges(&self, ranges: &[(Pattern, Exp)]) -> String {
        ranges
            .iter()
            .map(|(pat, domain)| {
                format!("{}: {}", self.fmt_pattern(pat), domain.display_cont(self))
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
    /// Creates a display of an operation which can be used in formatting.
    pub fn display<'a>(&'a self, env: &'a GlobalEnv, node_id: NodeId) -> OperationDisplay<'a> {
        OperationDisplay {
            env,
            oper: self,
            node_id,
            tctx: TypeDisplayContext::new(env),
        }
    }

    fn display_for_exp<'a>(
        &'a self,
        exp_display: &'a ExpDisplay,
        node_id: NodeId,
    ) -> OperationDisplay<'a> {
        let tctx = if let Some(fe) = &exp_display.fun_env {
            fe.get_type_display_ctx()
        } else {
            TypeDisplayContext::new(exp_display.env)
        };
        OperationDisplay {
            env: exp_display.env,
            oper: self,
            node_id,
            tctx,
        }
    }
}

/// Helper type for operation display.
pub struct OperationDisplay<'a> {
    env: &'a GlobalEnv,
    node_id: NodeId,
    oper: &'a Operation,
    tctx: TypeDisplayContext<'a>,
}

impl<'a> fmt::Display for OperationDisplay<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        use Operation::*;
        match self.oper {
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
            MoveFunction(mid, fid) => {
                write!(
                    f,
                    "{}",
                    self.env
                        .get_function(mid.qualified(*fid))
                        .get_full_name_str()
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
            Pack(mid, sid) => write!(f, "pack {}", self.struct_str(mid, sid)),
            Select(mid, sid, fid) => {
                write!(f, "select {}", self.field_str(mid, sid, fid))
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
                type_inst.iter().map(|ty| ty.display(&self.tctx)).join(", ")
            )?;
        }
        Ok(())
    }
}

impl<'a> OperationDisplay<'a> {
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
        let module_env = self.env.get_module(*mid);
        let struct_env = module_env.get_struct(*sid);
        format!(
            "{}::{}",
            module_env.get_name().display(self.env),
            struct_env.get_name().display(self.env.symbol_pool()),
        )
    }

    fn field_str(&self, mid: &ModuleId, sid: &StructId, fid: &FieldId) -> String {
        let struct_env = self.env.get_module(*mid).into_struct(*sid);
        let field_name = struct_env.get_field(*fid).get_name();
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

impl<'a> fmt::Display for EnvDisplay<'a, Condition> {
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

impl<'a> fmt::Display for EnvDisplay<'a, Spec> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "spec {{")?;
        for cond in &self.val.conditions {
            writeln!(f, "  {}", self.env.display(cond))?
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}
