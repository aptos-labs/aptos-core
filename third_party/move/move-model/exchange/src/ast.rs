// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! XAST: the source-level exchange format for the compiler-v2 typed AST.
//!
//! Where the XIR wrapper in the crate root exchanges *stackless bytecode* for
//! a semantic consumer, XAST exchanges the move-model AST of a module —
//! declarations, typed expressions, and specifications — for source-level
//! consumers such as the Lean transpiler (`third_party/move/lean/transpiler`).
//! It is a faithful, policy-free snapshot of the model after compiler v2's
//! env checking and rewriting (inlining, spec rewriting, match transforms) and
//! before AST optimization; all transpilation policy lives in the consumer.
//!
//! **These types are the normative schema.**  The wire format is JSON as
//! serde derives it, with the same conventions as the XIR format: structs are
//! objects with snake_case field names, enums are externally tagged with
//! snake_case variant names (unit variant → bare string, newtype variant →
//! single-key object with the payload, struct variant → single-key object
//! with the named fields).
//!
//! Conventions specific to XAST:
//!
//! - References are **by name**, never by model index: a declaration of
//!   another module is a [`QualifiedName`] (module reference plus declaration
//!   name), a module a [`ModuleRef`] (address, optional address alias, name).
//!   Model ids are unstable across builds and never appear on the wire.
//! - **Interned tables.**  Types, locations, module references, and qualified
//!   names are stored once in top-level tables of the export unit
//!   ([`XastModule::types`], [`XastModule::locs`], [`XastModule::modules`],
//!   [`XastModule::names`]) and referenced by index ([`TypeId`], [`LocId`],
//!   [`ModuleId`], [`NameId`]) from every node, so a per-node type and
//!   location cost a few bytes instead of a tree.  Type table entries
//!   reference their component types by index as well; the producer interns
//!   bottom-up, so a type's components always have smaller indices.
//! - Every expression and pattern node carries its **type**; calls carry their
//!   type instantiation.  The model keeps these in side tables keyed by node
//!   id; XAST records them per node (as indices).
//! - Numbers travel as decimal strings (the model uses big integers);
//!   addresses as `0x`-prefixed hex literals.  The model's redundant constant
//!   encodings (`ByteArray`, `AddressArray`, `Vector`) are normalized to one
//!   [`Value::Vector`] form.
//! - Locations are `[file, start, end]` byte offsets into the file table
//!   [`XastModule::sources`]; declaration and expression nodes carry them.
//! - Condition payloads are un-overloaded: kind-specific named fields replace
//!   the model's positional `additional_exps`.
//! - Function-typed parameters, their `Invoke` expressions, and the closed
//!   family of behavior predicates are represented so retained specification
//!   helpers can cross the boundary. Function-value construction (lambdas and
//!   closures) remains out of scope and is rejected by the producer.
//!
//! Version history: version 1 introduced this format; version 2 added function
//! types and `Invoke` expressions for retained inline helpers; version 3 added
//! resolved intrinsic-type function-role bindings; version 4 added behavior
//! predicates over function values.

use serde::{Deserialize, Serialize};

/// Schema identifier of an XAST module document.
pub const XAST_SCHEMA: &str = "move-xast-module";
/// Current version of the XAST format.
pub const XAST_VERSION: u64 = 4;

/// Index into [`XastModule::types`].
pub type TypeId = usize;
/// Index into [`XastModule::locs`].
pub type LocId = usize;
/// Index into [`XastModule::modules`].
pub type ModuleId = usize;
/// Index into [`XastModule::names`].
pub type NameId = usize;

// =================================================================================================
// Module

/// A declaration left out of the export, by name, with the reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skipped {
    pub name: String,
    pub reason: String,
}

/// A Move module's typed AST.  The top-level object of an XAST document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XastModule {
    /// Always [`XAST_SCHEMA`].
    pub schema: String,
    /// Always [`XAST_VERSION`].
    pub version: u64,
    /// The module's address as a `0x`-prefixed hex literal.
    pub address: String,
    /// The named address the module was declared under, if any.
    pub address_alias: Option<String>,
    /// The module name.
    pub name: String,
    /// The module's doc comment (empty if none).
    pub doc: String,
    /// Location of the module declaration.
    pub loc: LocId,
    /// The package's named-address table (name, `0x`-hex address), so a
    /// consumer can re-declare aliases the module or its friends use.
    pub named_addresses: Vec<NamedAddress>,
    /// Friend modules.
    pub friends: Vec<ModuleId>,
    /// Module-level spec properties (pragmas), raw.
    pub pragmas: Vec<Pragma>,
    pub constants: Vec<Constant>,
    /// Structs and enums.
    pub structs: Vec<Struct>,
    pub functions: Vec<Function>,
    pub spec_funs: Vec<SpecFun>,
    /// Ghost variables.
    pub spec_vars: Vec<SpecVar>,
    /// Module-level global invariants, update invariants, and axioms.
    pub invariants: Vec<Invariant>,
    /// Declarations the producer left out, each with the reason: functions
    /// and spec functions that construct function values (lambdas or
    /// closures), which the format does not cover.
    #[serde(default)]
    pub skipped: Vec<Skipped>,
    /// Ordinary (non-doc) comments of the module's source files, with their
    /// spans.  compiler v2 discards them; the producer scans the source text.
    /// Doc comments travel on the declarations they document.
    pub comments: Vec<Comment>,
    // ---- interned tables ----
    /// File table referenced by [`Loc::file`]: source paths as the compiler
    /// saw them.
    pub sources: Vec<String>,
    /// Location table referenced by [`LocId`].
    pub locs: Vec<Loc>,
    /// Type table referenced by [`TypeId`]; entries reference their
    /// components by index (always smaller).
    pub types: Vec<Type>,
    /// Module reference table referenced by [`ModuleId`].
    pub modules: Vec<ModuleRef>,
    /// Qualified name table referenced by [`NameId`].
    pub names: Vec<QualifiedName>,
}

impl XastModule {
    /// Checks schema and version.
    pub fn check_version(&self) -> Result<(), String> {
        if self.schema != XAST_SCHEMA {
            return Err(format!(
                "unexpected XAST schema `{}` (expected `{}`)",
                self.schema, XAST_SCHEMA
            ));
        }
        if self.version != XAST_VERSION {
            return Err(format!(
                "unsupported XAST version {} (expected {})",
                self.version, XAST_VERSION
            ));
        }
        Ok(())
    }

    /// Pretty-printed JSON, for files and baselines.
    pub fn to_pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("serialization succeeds")
    }

    /// Parses a document and checks its version.
    pub fn from_json(text: &str) -> Result<XastModule, String> {
        let module: XastModule = serde_json::from_str(text).map_err(|e| e.to_string())?;
        module.check_version()?;
        Ok(module)
    }
}

/// A named address of the package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedAddress {
    pub name: String,
    /// `0x`-prefixed hex literal.
    pub address: String,
}

/// Reference to a module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModuleRef {
    /// `0x`-prefixed hex literal.
    pub address: String,
    pub address_alias: Option<String>,
    pub name: String,
}

/// Reference to a declaration (struct, function, spec function, ...) of a
/// module.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QualifiedName {
    pub module: ModuleId,
    pub name: String,
}

/// A source location: byte offsets into the file [`XastModule::sources`]`[file]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Loc {
    pub file: usize,
    pub start: u32,
    pub end: u32,
}

/// An ordinary source comment, delimiters included (`// ...`, `/* ... */`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub loc: LocId,
    pub text: String,
    /// Whether only whitespace precedes the comment on its line (a comment
    /// on a line of its own leads the next item; one after code trails the
    /// item before it).
    pub own_line: bool,
}

// =================================================================================================
// Pragmas, properties, attributes

/// A spec property (`pragma name = value;` or a condition property
/// `[name = value]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pragma {
    pub name: String,
    pub value: PragmaValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PragmaValue {
    Value(Value),
    Name(String),
    QualifiedName(String),
}

/// A Move attribute (`#[name]`, `#[name(args)]`, `#[name = value]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Attribute {
    Apply { name: String, args: Vec<Attribute> },
    Assign { name: String, value: AttributeValue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributeValue {
    Value(Value),
    Name {
        module: Option<ModuleId>,
        name: String,
    },
}

// =================================================================================================
// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ability {
    Copy,
    Drop,
    Store,
    Key,
}

/// A type parameter of a struct, function, or spec function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: String,
    pub abilities: Vec<Ability>,
    pub is_phantom: bool,
}

/// A Move or spec type. Error types and type variables do not occur (the
/// producer rejects them). Component types are [`TypeId`]s into the type
/// table.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
    /// Spec-only unbounded integer.
    Num,
    /// Spec-only range.
    Range,
    /// Spec-only event store.
    EventStore,
    Tuple(Vec<TypeId>),
    Vector(TypeId),
    Struct {
        name: NameId,
        args: Vec<TypeId>,
    },
    /// A Move function value. `args` is canonically a tuple type (unit for no
    /// arguments); `result` may likewise be a tuple for multiple returns.
    Function {
        args: TypeId,
        result: TypeId,
        abilities: Vec<Ability>,
    },
    /// Index into the enclosing declaration's type parameters.
    TypeParam(u16),
    Reference {
        mutable: bool,
        ty: TypeId,
    },
    /// Spec-only: the domain of a type (`forall x: T`).
    TypeDomain(TypeId),
    /// Spec-only: the resource domain of a struct.
    ResourceDomain {
        name: NameId,
        args: Option<Vec<TypeId>>,
    },
    /// Spec-only.
    StateDomain,
}

// =================================================================================================
// Values

/// A constant value.  Numbers are decimal strings, addresses `0x`-hex.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    Address(String),
    Number(String),
    Bool(bool),
    Vector(Vec<Value>),
    Tuple(Vec<Value>),
}

// =================================================================================================
// Declarations

/// A named constant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    pub doc: String,
    pub loc: LocId,
    pub ty: TypeId,
    pub value: Value,
}

/// A struct or enum declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    pub name: String,
    pub doc: String,
    pub loc: LocId,
    pub abilities: Vec<Ability>,
    pub type_params: Vec<TypeParam>,
    pub attributes: Vec<Attribute>,
    pub is_native: bool,
    /// The fields of a struct; empty for an enum (see `variants`).
    pub fields: Vec<Field>,
    /// The variants of an enum; `None` for a struct.
    pub variants: Option<Vec<Variant>>,
    /// The struct spec: data invariants and pragmas.
    pub spec: Spec,
    /// A resolved intrinsic declaration. The model builder consumes the
    /// source's role properties while validating them, so they are carried
    /// separately from `spec.pragmas` rather than reconstructed by consumers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intrinsic: Option<Intrinsic>,
}

/// A function assigned to one semantic role of an intrinsic type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntrinsicBinding {
    /// The intrinsic role (`map_new`, `map_spec_get`, and so on).
    pub role: String,
    /// The resolved Move or specification function.
    pub target: NameId,
}

/// A validated intrinsic-type declaration and its resolved role bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intrinsic {
    /// The intrinsic model name (`map`, currently).
    pub name: String,
    /// Roles bound to executable Move functions.
    pub move_functions: Vec<IntrinsicBinding>,
    /// Roles bound to specification functions.
    pub spec_functions: Vec<IntrinsicBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub loc: LocId,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub doc: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Private,
    Public,
    Friend,
    Package,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionKind {
    Regular,
    /// An inline function retained in verify mode (explicit spec, no
    /// function-typed parameters); first-order, with its calls kept as calls.
    InlineRetained,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: TypeId,
}

/// A function declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub doc: String,
    pub loc: LocId,
    pub visibility: Visibility,
    pub is_entry: bool,
    pub kind: FunctionKind,
    /// Whether the first parameter is a Move 2 receiver (`self`).
    pub is_receiver: bool,
    pub attributes: Vec<Attribute>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    /// The result type; a tuple for multiple returns, the unit tuple for none.
    pub result: TypeId,
    /// Resolved pragmas: the function's own, with module-level pragmas
    /// inherited where the function does not set them.
    pub pragmas: Vec<Pragma>,
    pub spec: Spec,
    /// The body; `None` for native functions.
    pub body: Option<Exp>,
}

/// A spec function.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecFun {
    pub name: String,
    pub doc: String,
    pub loc: LocId,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub result: TypeId,
    pub uninterpreted: bool,
    pub is_native: bool,
    /// Generated from a Move function (`spec fun` mirror of a pure function).
    pub is_move_fun: bool,
    pub uses_old: bool,
    /// The body; `None` for uninterpreted and native spec functions.
    pub body: Option<Exp>,
    /// Conditions attached to the spec function (uninterpreted ones).
    pub spec: Spec,
}

/// A ghost variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecVar {
    pub name: String,
    pub loc: LocId,
    pub type_params: Vec<TypeParam>,
    pub ty: TypeId,
    pub init: Option<Exp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvariantKind {
    Global,
    GlobalUpdate,
    Axiom,
}

/// A module-level global invariant, update invariant, or axiom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invariant {
    pub kind: InvariantKind,
    pub loc: LocId,
    /// Type parameters bound by the invariant (`invariant<T> ...`).
    pub type_params: Vec<String>,
    pub properties: Vec<Pragma>,
    pub exp: Exp,
}

// =================================================================================================
// Specifications

/// A specification block (of a function, struct, spec function, or an
/// in-body `spec { }` block).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spec {
    pub loc: Option<LocId>,
    /// The block's own pragmas, raw (no inheritance applied).
    pub pragmas: Vec<Pragma>,
    pub conditions: Vec<Condition>,
    /// The frame specification (`modifies`, `reads_of`/`modifies_of`
    /// access declarations), if any.
    pub frame: Option<Frame>,
}

/// A frame specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// `modifies global<R>(a)` targets: `Global` calls with their address.
    pub modifies: Vec<Exp>,
    /// Resource types declared as read.
    pub reads: Vec<TypeId>,
    /// `modifies_of<f> *`: any memory may be modified.
    pub modifies_all: bool,
    /// `reads_of<f> *`: any memory may be read.
    pub reads_all: bool,
}

/// The kind of a condition, mirroring the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionKind {
    /// `let post name = exp;`
    LetPost {
        name: String,
    },
    /// `let name = exp;`
    LetPre {
        name: String,
    },
    Assert,
    Assume,
    Decreases,
    AbortsIf,
    AbortsWith,
    SucceedsIf,
    Emits,
    Ensures,
    Requires,
    StructInvariant,
    FunctionInvariant,
    LoopInvariant,
    GlobalInvariant {
        type_params: Vec<String>,
    },
    GlobalInvariantUpdate {
        type_params: Vec<String>,
    },
    SchemaInvariant,
    Axiom {
        type_params: Vec<String>,
    },
    Update,
}

/// A spec condition.  `exp` is the condition's main expression in every
/// kind (the value of a `let`, the condition of `aborts_if`, the first code of
/// `aborts_with`, the message of `emits`, the right-hand side of `update`);
/// the kind-specific payloads are named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    pub kind: ConditionKind,
    pub loc: LocId,
    /// Condition properties (`[abstract]`, `[concrete]`, `[injected]`, ...).
    pub properties: Vec<Pragma>,
    pub exp: Exp,
    /// `aborts_if cond with code`: the code.
    pub abort_code: Option<Exp>,
    /// `aborts_with c1, c2, ...`: the codes after the first (which is `exp`).
    pub additional_codes: Vec<Exp>,
    /// `emits msg to handle [if cond]`: the handle.
    pub emits_handle: Option<Exp>,
    /// `emits msg to handle if cond`: the condition.
    pub emits_condition: Option<Exp>,
    /// `update lhs = exp`: the target.
    pub update_target: Option<Exp>,
}

// =================================================================================================
// Expressions

/// Whether a call node was written in a concise surface form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceSyntax {
    /// Receiver-style call `x.f(args)`.
    ReceiverCall,
    /// Index notation `v[i]`, `R[a]`.
    IndexNotation,
}

/// A typed expression node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exp {
    /// The node's result type.
    pub ty: TypeId,
    pub loc: LocId,
    #[serde(flatten)]
    pub node: ExpNode,
}

/// The expression forms, mirroring the model's `ExpData` minus the rejected
/// variants (`Invalid` and `Lambda`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExpNode {
    Value {
        value: Value,
        /// The named constant of the module the source wrote at this node,
        /// if any: compiler v2 folds constants into their values, and the
        /// producer recovers the name from the source span (an identifier
        /// naming a module constant of equal value).
        constant: Option<String>,
    },
    /// A local variable (introduced by a `let`, a pattern, or a quantifier).
    Local {
        name: String,
    },
    /// A function parameter, by index.
    Param {
        index: usize,
    },
    Call {
        op: Operation,
        /// The type instantiation of the operation (e.g. the type arguments
        /// of a called function or packed struct).
        inst: Vec<TypeId>,
        args: Vec<Exp>,
        surface: Option<SurfaceSyntax>,
    },
    /// Invoke a function-valued expression.
    Invoke {
        function: Box<Exp>,
        args: Vec<Exp>,
    },
    /// `let pattern [= binding]; body`.
    Block {
        pattern: Pattern,
        binding: Option<Box<Exp>>,
        body: Box<Exp>,
    },
    If {
        cond: Box<Exp>,
        then_branch: Box<Exp>,
        else_branch: Box<Exp>,
    },
    Match {
        scrutinee: Box<Exp>,
        arms: Vec<MatchArm>,
    },
    Sequence {
        exps: Vec<Exp>,
    },
    Loop {
        body: Box<Exp>,
    },
    /// `break`/`continue` of the loop `nest` levels out (0 = innermost).
    LoopCont {
        nest: usize,
        is_continue: bool,
    },
    Return {
        value: Box<Exp>,
    },
    Assign {
        pattern: Pattern,
        value: Box<Exp>,
    },
    /// `*target = value`.
    Mutate {
        target: Box<Exp>,
        value: Box<Exp>,
    },
    /// An in-body `spec { ... }` block.
    SpecBlock {
        spec: Spec,
    },
    Quant {
        quant: QuantKind,
        ranges: Vec<QuantRange>,
        triggers: Vec<Vec<Exp>>,
        condition: Option<Box<Exp>>,
        body: Box<Exp>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchArm {
    pub loc: LocId,
    pub pattern: Pattern,
    pub guard: Option<Exp>,
    pub body: Exp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantKind {
    Forall,
    Exists,
    Choose,
    ChooseMin,
}

/// One binder of a quantifier with its domain expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantRange {
    pub pattern: Pattern,
    pub domain: Exp,
}

/// A typed pattern node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    pub ty: TypeId,
    pub loc: LocId,
    #[serde(flatten)]
    pub node: PatternNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PatternNode {
    Var {
        name: String,
    },
    Wildcard,
    Tuple {
        elements: Vec<Pattern>,
    },
    Struct {
        name: NameId,
        inst: Vec<TypeId>,
        variant: Option<String>,
        fields: Vec<Pattern>,
    },
    Literal {
        value: Value,
    },
    Range {
        lower: Option<Value>,
        upper: Option<Value>,
        inclusive: bool,
    },
}

// =================================================================================================
// Operations

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Immutable,
    Mutable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbortKind {
    Code,
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceKind {
    User,
    Auto,
    SubAuto,
}

/// A pre/post memory-label pair of a spec operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRange {
    pub pre: Option<u64>,
    pub post: Option<u64>,
}

/// The closed family of specification predicates over a function value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorKind {
    RequiresOf,
    AbortsOf,
    EnsuresOf,
    ResultOf,
    UnchangedOf,
    FoldsOf,
    WriteOf(usize),
}

/// The operation of a call node, mirroring the model's `Operation` with
/// qualified names in place of ids. Function-value construction via `Closure`
/// is rejected by the producer; behavior predicates are transported directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    MoveFunction(NameId),
    Pack {
        name: NameId,
        variant: Option<String>,
    },
    Tuple,
    Select {
        name: NameId,
        field: String,
    },
    SelectVariants {
        name: NameId,
        fields: Vec<String>,
    },
    TestVariants {
        name: NameId,
        variants: Vec<String>,
    },
    SpecFunction {
        name: NameId,
        range: MemoryRange,
    },
    Behavior {
        kind: BehaviorKind,
        range: MemoryRange,
    },
    UpdateField {
        name: NameId,
        field: String,
    },
    Result(usize),
    Index,
    Slice,
    Range,
    Implies,
    Iff,
    Identical,
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
    Copy,
    Move,
    Not,
    /// An integer cast: checked in code; in specifications also the widening of
    /// a bounded-integer variable read at type `num` — the model's implicit
    /// specification widening, exported explicitly so that the XAST has no
    /// implicit conversions.
    Cast,
    Negate,
    Exists(Option<u64>),
    BorrowGlobal(RefKind),
    Borrow(RefKind),
    Deref,
    MoveTo,
    MoveFrom,
    /// The flag says whether the freeze was explicit in the source.
    Freeze(bool),
    Abort(AbortKind),
    Vector,
    Len,
    TypeValue,
    TypeDomain,
    ResourceDomain,
    StateDomain,
    Global(Option<u64>),
    CanModify,
    Old,
    SaveStateAnchor(u64),
    WithStateAnchor(u64),
    FoldsCaptureAnchor(u64),
    InlineCallSummary,
    Trace(TraceKind),
    SpecPublish(MemoryRange),
    SpecRemove(MemoryRange),
    SpecUpdate(MemoryRange),
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
    AbortFlag,
    AbortCode,
    WellFormed,
    BoxValue,
    UnboxValue,
    EmptyEventStore,
    ExtendEventStore,
    EventStoreIncludes,
    EventStoreIncludedIn,
    NoOp,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc() -> LocId {
        0
    }

    #[test]
    fn wire_shapes_are_pinned() {
        // Unit variant: bare string.
        assert_eq!(serde_json::to_string(&Type::U64).unwrap(), r#""u64""#);
        assert_eq!(serde_json::to_string(&Operation::Add).unwrap(), r#""add""#);
        // Newtype variant: single-key object.
        assert_eq!(
            serde_json::to_string(&Operation::Result(1)).unwrap(),
            r#"{"result":1}"#
        );
        assert_eq!(
            serde_json::to_string(&Operation::Behavior {
                kind: BehaviorKind::WriteOf(3),
                range: MemoryRange {
                    pre: Some(1),
                    post: Some(2),
                },
            })
            .unwrap(),
            r#"{"behavior":{"kind":{"write_of":3},"range":{"pre":1,"post":2}}}"#
        );
        assert_eq!(
            serde_json::to_string(&Type::Vector(3)).unwrap(),
            r#"{"vector":3}"#
        );
        // Struct variant: single-key object with named fields.
        assert_eq!(
            serde_json::to_string(&Type::Reference {
                mutable: true,
                ty: 0
            })
            .unwrap(),
            r#"{"reference":{"mutable":true,"ty":0}}"#
        );
        assert_eq!(
            serde_json::to_string(&Type::Function {
                args: 1,
                result: 2,
                abilities: vec![Ability::Copy]
            })
            .unwrap(),
            r#"{"function":{"args":1,"result":2,"abilities":["copy"]}}"#
        );
        // Expression nodes: the kind tag is flattened beside `ty` and `loc`,
        // which are table indices.
        let exp = Exp {
            ty: 2,
            loc: loc(),
            node: ExpNode::Value {
                value: Value::Number("5".to_string()),
                constant: None,
            },
        };
        assert_eq!(
            serde_json::to_string(&exp).unwrap(),
            r#"{"ty":2,"loc":0,"kind":"value","value":{"number":"5"},"constant":null}"#
        );
        let pat = Pattern {
            ty: 1,
            loc: loc(),
            node: PatternNode::Wildcard,
        };
        assert_eq!(
            serde_json::to_string(&pat).unwrap(),
            r#"{"ty":1,"loc":0,"kind":"wildcard"}"#
        );
        assert_eq!(
            serde_json::to_string(&ConditionKind::LetPost {
                name: "x".to_string()
            })
            .unwrap(),
            r#"{"let_post":{"name":"x"}}"#
        );
    }

    #[test]
    fn round_trips() {
        let exp = Exp {
            ty: 1,
            loc: loc(),
            node: ExpNode::Call {
                op: Operation::Lt,
                inst: vec![2],
                args: vec![
                    Exp {
                        ty: 2,
                        loc: loc(),
                        node: ExpNode::Param { index: 0 },
                    },
                    Exp {
                        ty: 2,
                        loc: loc(),
                        node: ExpNode::Local {
                            name: "y".to_string(),
                        },
                    },
                ],
                surface: Some(SurfaceSyntax::ReceiverCall),
            },
        };
        let text = serde_json::to_string(&exp).unwrap();
        let back: Exp = serde_json::from_str(&text).unwrap();
        assert_eq!(exp, back);

        let invoke = Exp {
            ty: 2,
            loc: loc(),
            node: ExpNode::Invoke {
                function: Box::new(Exp {
                    ty: 3,
                    loc: loc(),
                    node: ExpNode::Param { index: 1 },
                }),
                args: vec![Exp {
                    ty: 2,
                    loc: loc(),
                    node: ExpNode::Param { index: 0 },
                }],
            },
        };
        let text = serde_json::to_string(&invoke).unwrap();
        let back: Exp = serde_json::from_str(&text).unwrap();
        assert_eq!(invoke, back);
    }

    #[test]
    fn version_is_checked() {
        let module = XastModule {
            schema: XAST_SCHEMA.to_string(),
            version: XAST_VERSION + 1,
            address: "0x1".to_string(),
            address_alias: None,
            name: "m".to_string(),
            doc: String::new(),
            loc: loc(),
            named_addresses: vec![],
            friends: vec![],
            pragmas: vec![],
            constants: vec![],
            structs: vec![],
            functions: vec![],
            spec_funs: vec![],
            spec_vars: vec![],
            invariants: vec![],
            skipped: vec![],
            comments: vec![],
            sources: vec![],
            locs: vec![],
            types: vec![],
            modules: vec![],
            names: vec![],
        };
        assert!(module.check_version().is_err());
        let text = module.to_pretty_json();
        assert!(XastModule::from_json(&text).is_err());
    }
}
