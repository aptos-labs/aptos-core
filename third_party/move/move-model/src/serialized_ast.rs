// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Deterministically serializable stripped AST for Move expressions.
//!
//! # Purpose
//!
//! This module defines a *stripped* representation of the Move expression AST that can be
//! deterministically serialized to bytes (via BCS) and stored in `.mv` bytecode metadata.
//! The primary use cases are:
//!
//! - **Modular compilation**: `public inline` function bodies are absent from bytecode.  By
//!   storing the serialized body in metadata, downstream packages can inline the function
//!   without needing the original source.
//!
//! - **Modular verification**: function specs stored in metadata allow the Move Prover to
//!   verify a package using only its dependencies' pre/post-conditions, not their full source.
//!
//! # Determinism requirements
//!
//! The same AST structure must always produce the same bytes.  The following normalizations
//! are applied during the conversion pass:
//!
//! | Source AST element | Serialized form |
//! |--------------------|-----------------|
//! | `NodeId` (per-compilation opaque index) | Dropped; result type stored separately in `SerExp` |
//! | `Symbol` (interned string index) | Resolved to the string content |
//! | `ModuleId` (per-compilation opaque index) | Resolved to `"0x<hex>::<name>"` |
//! | `StructId(Symbol)`, `FunId(Symbol)`, `FieldId(Symbol)` | Resolved to the symbol string |
//! | `SpecFunId(RawIndex)` | Resolved to the spec function's name string |
//! | `Address::Numerical` | 32-byte big-endian address bytes |
//! | `Address::Symbolic` | `\xff` prefix + UTF-8 name bytes |
//! | `BigInt` | `[sign, be_magnitude...]` bytes |

use crate::{
    ast::{
        AbortKind, Address, BehaviorKind, BehaviorState, ExpData, LambdaCaptureKind, MatchArm,
        ModuleName, Operation, Pattern, QuantKind, Spec, TraceKind, Value,
    },
    model::{FieldId, FunId, GlobalEnv, ModuleId, QualifiedInstId, SpecFunId, StructId},
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_core_types::{ability::AbilitySet, account_address::AccountAddress, metadata::Metadata};
use num::BigInt;
use num_traits::Signed;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// =============================================================================
// Metadata key

/// The key used to identify inline function body metadata in a `CompiledModule`.
pub const INLINE_BODIES_METADATA_KEY: &[u8] = b"move:inline_bodies:v1";

// =============================================================================
// Top-level container

/// Top-level container stored in `CompiledModule` metadata under
/// [`INLINE_BODIES_METADATA_KEY`].  Maps the unqualified function name to its
/// serialized body.  Only `public inline` functions are included.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InlineFunctionBodies {
    /// Sorted by function name for determinism.
    pub functions: BTreeMap<String, SerFunctionBody>,
}

impl InlineFunctionBodies {
    /// Serialize to bytes using BCS.
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        bcs::to_bytes(self).map_err(|e| anyhow::anyhow!("BCS serialization failed: {}", e))
    }

    /// Deserialize from BCS bytes.
    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(|e| anyhow::anyhow!("BCS deserialization failed: {}", e))
    }

    /// Build a [`Metadata`] entry suitable for storing in a `CompiledModule`.
    pub fn into_metadata(self) -> anyhow::Result<Metadata> {
        Ok(Metadata {
            key: INLINE_BODIES_METADATA_KEY.to_vec(),
            value: self.to_bytes()?,
        })
    }

    /// Extract from the metadata list of a `CompiledModule`, if present.
    pub fn from_metadata(metadata: &[Metadata]) -> anyhow::Result<Option<Self>> {
        for entry in metadata {
            if entry.key == INLINE_BODIES_METADATA_KEY {
                return Ok(Some(Self::from_bytes(&entry.value)?));
            }
        }
        Ok(None)
    }
}

// =============================================================================
// Serialized function body

/// Serialized representation of a single `public inline` function body.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerFunctionBody {
    /// Type parameter names in declaration order.
    pub type_params: Vec<String>,
    /// Parameters: (name, type) in declaration order.
    pub params: Vec<(String, SerType)>,
    /// Return types.
    pub return_types: Vec<SerType>,
    /// The function body expression (typed).
    pub body: SerExp,
}

// =============================================================================
// SerType

/// Serializable type — mirrors [`Type`] with all opaque IDs resolved to strings.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerType {
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
    // Spec-only primitives
    Num,
    Range,
    EventStore,
    Tuple(Vec<SerType>),
    Vector(Box<SerType>),
    /// Fully-qualified struct: `"0x<addr>::<module>::<Struct>"`.
    Struct(String, Vec<SerType>),
    TypeParameter(u16),
    /// Function type: `(arg_type, result_type, ability_bitmask)`.
    Fun(Box<SerType>, Box<SerType>, u8),
    /// `true` = mutable reference.
    Reference(bool, Box<SerType>),
    // Spec-only types
    TypeDomain(Box<SerType>),
    ResourceDomain(String, Vec<SerType>),
    /// Placeholder for types that shouldn't appear after type-checking.
    Unknown,
}

// =============================================================================
// SerExp — typed wrapper

/// A typed serialized expression.  Every node carries its result type so the
/// deserializer can recreate properly-typed `NodeId`s in `GlobalEnv` without
/// re-running the type checker.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerExp {
    /// Result type of this expression node.
    pub ty: SerType,
    /// The expression kind.
    pub kind: SerExpKind,
}

// =============================================================================
// SerExpKind

/// The expression kind — mirrors [`ExpData`] with `NodeId` dropped.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerExpKind {
    Invalid,
    Value(SerValue),
    /// Local variable reference — the string is the variable name.
    LocalVar(String),
    /// Parameter/temporary reference — the usize is the positional index.
    Temporary(usize),
    Call(SerOperation, Vec<SerExp>),
    Invoke(Box<SerExp>, Vec<SerExp>),
    Lambda(
        SerPattern,
        Box<SerExp>,
        SerLambdaCaptureKind,
        Option<Box<SerExp>>,
    ),
    Quant(
        SerQuantKind,
        Vec<(SerPattern, SerExp)>,
        Vec<Vec<SerExp>>,
        Option<Box<SerExp>>,
        Box<SerExp>,
    ),
    Block(SerPattern, Option<Box<SerExp>>, Box<SerExp>),
    IfElse(Box<SerExp>, Box<SerExp>, Box<SerExp>),
    Match(Box<SerExp>, Vec<SerMatchArm>),
    Return(Box<SerExp>),
    Sequence(Vec<SerExp>),
    Loop(Box<SerExp>),
    /// `(loop_nesting_depth, is_continue)`.
    LoopCont(usize, bool),
    Assign(SerPattern, Box<SerExp>),
    Mutate(Box<SerExp>, Box<SerExp>),
    SpecBlock(SerSpec),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerMatchArm {
    pub pattern: SerPattern,
    pub condition: Option<SerExp>,
    pub body: SerExp,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerLambdaCaptureKind {
    Default,
    Copy,
    Move,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerQuantKind {
    Forall,
    Exists,
    Choose,
    ChooseMin,
}

// =============================================================================
// SerOperation

/// Serializable operation — mirrors [`Operation`] with all opaque IDs resolved.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerOperation {
    /// `"0x<addr>::<module>::<fun_name>"`.
    MoveFunction(String),
    /// `"0x<addr>::<module>::<Struct>"`, optional variant name.
    Pack(String, Option<String>),
    /// `"0x<addr>::<module>::<fun_name>"`, closure mask bits.
    Closure(String, u64),
    Tuple,
    /// `"0x<addr>::<module>::<Struct>"`, `"<field_name>"`.
    Select(String, String),
    /// `"0x<addr>::<module>::<Struct>"`, list of field names from different variants.
    SelectVariants(String, Vec<String>),
    /// `"0x<addr>::<module>::<Struct>"`, list of variant names.
    TestVariants(String, Vec<String>),
    /// `"0x<addr>::<module>::<spec_fun_name>"`, optional memory labels.
    SpecFunction(String, Option<Vec<u64>>),
    /// `"0x<addr>::<module>::<Struct>"`, `"<field_name>"`.
    UpdateField(String, String),
    Behavior(SerBehaviorKind, Option<u64>, Option<u64>),
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
    Cast,
    Negate,
    /// Optional memory label (raw id, spec-only).
    Exists(Option<u64>),
    /// `true` = mutable.
    BorrowGlobal(bool),
    /// `true` = mutable.
    Borrow(bool),
    Deref,
    MoveTo,
    MoveFrom,
    /// `true` = explicit freeze.
    Freeze(bool),
    Abort(SerAbortKind),
    Vector,
    Len,
    TypeValue,
    TypeDomain,
    ResourceDomain,
    /// Optional memory label (spec-only).
    Global(Option<u64>),
    CanModify,
    Old,
    Trace(SerTraceKind),
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerAbortKind {
    Code,
    Message,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerTraceKind {
    User,
    Auto,
    SubAuto,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerBehaviorKind {
    RequiresOf,
    AbortsOf,
    EnsuresOf,
    ModifiesOf,
    ResultOf,
}

// =============================================================================
// SerPattern

/// Serializable pattern — mirrors [`Pattern`] with `NodeId` dropped and types included.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerPattern {
    /// Variable binding: `(name, type)`.
    Var(String, SerType),
    Wildcard,
    Tuple(Vec<SerPattern>),
    /// Struct pattern: `(fully_qualified_struct_name, optional_variant, sub_patterns)`.
    Struct(String, Option<String>, Vec<SerPattern>),
    LiteralValue(SerValue),
    Error,
}

// =============================================================================
// SerValue

/// Serializable value — mirrors [`Value`].
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerValue {
    /// 32-byte big-endian account address, or `\xff` + UTF-8 for symbolic.
    Address(Vec<u8>),
    /// `[sign_byte=0(pos)/1(neg), be_magnitude...]`.
    Number(Vec<u8>),
    Bool(bool),
    ByteArray(Vec<u8>),
    AddressArray(Vec<Vec<u8>>),
    Vector(Vec<SerValue>),
    Tuple(Vec<SerValue>),
}

// =============================================================================
// SerSpec

/// Simplified serializable spec (enough for assert!/assume! in inline bodies).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerSpec {
    pub conditions: Vec<SerCondition>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SerCondition {
    /// `ConditionKind` formatted as a string (e.g., `"assert"`, `"ensures"`).
    pub kind: String,
    pub exp: SerExp,
    pub additional_exps: Vec<SerExp>,
}

// =============================================================================
// AstSerializer

/// Converts live [`GlobalEnv`] AST into the deterministically serializable stripped form.
pub struct AstSerializer<'a> {
    env: &'a GlobalEnv,
}

impl<'a> AstSerializer<'a> {
    pub fn new(env: &'a GlobalEnv) -> Self {
        Self { env }
    }

    // -------------------------------------------------------------------------
    // Public entry points

    /// Serialize all `public inline` functions in the given module.
    /// Returns `None` if the module has no inline functions with bodies.
    pub fn serialize_module_inline_bodies(
        &self,
        module_id: ModuleId,
    ) -> Option<InlineFunctionBodies> {
        let module = self.env.get_module(module_id);
        let mut functions = BTreeMap::new();
        for func in module.get_functions() {
            if func.is_inline() {
                if let Some(body) = func.get_def() {
                    let fun_name = self.env.symbol_pool().string(func.get_name()).to_string();
                    let type_params = func
                        .get_type_parameters()
                        .iter()
                        .map(|tp| self.env.symbol_pool().string(tp.0).to_string())
                        .collect();
                    let params = func
                        .get_parameters()
                        .iter()
                        .map(|p| {
                            (
                                self.env.symbol_pool().string(p.0).to_string(),
                                self.ser_type(&p.1),
                            )
                        })
                        .collect();
                    let return_types = match func.get_result_type() {
                        Type::Tuple(ts) => ts.iter().map(|t| self.ser_type(t)).collect(),
                        t => vec![self.ser_type(&t)],
                    };
                    let ser_body = self.ser_exp(body.as_ref());
                    functions.insert(fun_name, SerFunctionBody {
                        type_params,
                        params,
                        return_types,
                        body: ser_body,
                    });
                }
            }
        }
        if functions.is_empty() {
            None
        } else {
            Some(InlineFunctionBodies { functions })
        }
    }

    // -------------------------------------------------------------------------
    // Type conversion

    pub fn ser_type(&self, ty: &Type) -> SerType {
        use Type::*;
        match ty {
            Primitive(p) => self.ser_primitive(p),
            Tuple(ts) => SerType::Tuple(ts.iter().map(|t| self.ser_type(t)).collect()),
            Vector(inner) => SerType::Vector(Box::new(self.ser_type(inner))),
            Struct(mid, sid, inst) => SerType::Struct(
                self.struct_name_str(*mid, *sid),
                inst.iter().map(|t| self.ser_type(t)).collect(),
            ),
            TypeParameter(idx) => SerType::TypeParameter(*idx),
            Fun(arg, result, abilities) => SerType::Fun(
                Box::new(self.ser_type(arg)),
                Box::new(self.ser_type(result)),
                abilities.into_u8(),
            ),
            Reference(kind, inner) => SerType::Reference(
                matches!(kind, ReferenceKind::Mutable),
                Box::new(self.ser_type(inner)),
            ),
            TypeDomain(inner) => SerType::TypeDomain(Box::new(self.ser_type(inner))),
            ResourceDomain(mid, sid, inst_opt) => SerType::ResourceDomain(
                self.struct_name_str(*mid, *sid),
                inst_opt
                    .as_ref()
                    .map(|ts| ts.iter().map(|t| self.ser_type(t)).collect())
                    .unwrap_or_default(),
            ),
            Error | Var(_) => SerType::Unknown,
        }
    }

    fn ser_primitive(&self, p: &PrimitiveType) -> SerType {
        use PrimitiveType::*;
        match p {
            Bool => SerType::Bool,
            U8 => SerType::U8,
            U16 => SerType::U16,
            U32 => SerType::U32,
            U64 => SerType::U64,
            U128 => SerType::U128,
            U256 => SerType::U256,
            I8 => SerType::I8,
            I16 => SerType::I16,
            I32 => SerType::I32,
            I64 => SerType::I64,
            I128 => SerType::I128,
            I256 => SerType::I256,
            Address => SerType::Address,
            Signer => SerType::Signer,
            Num => SerType::Num,
            Range => SerType::Range,
            EventStore => SerType::EventStore,
        }
    }

    // -------------------------------------------------------------------------
    // Expression conversion — each node also records its result type.

    pub fn ser_exp(&self, exp: &ExpData) -> SerExp {
        let ty = self.ser_type(&self.env.get_node_type(exp.node_id()));
        let kind = self.ser_exp_kind(exp);
        SerExp { ty, kind }
    }

    fn ser_exp_kind(&self, exp: &ExpData) -> SerExpKind {
        use ExpData::*;
        match exp {
            Invalid(_) => SerExpKind::Invalid,
            Value(_, v) => SerExpKind::Value(self.ser_value(v)),
            LocalVar(_, sym) => {
                SerExpKind::LocalVar(self.env.symbol_pool().string(*sym).to_string())
            },
            Temporary(_, idx) => SerExpKind::Temporary(*idx),
            Call(_, op, args) => SerExpKind::Call(
                self.ser_operation(op),
                args.iter().map(|a| self.ser_exp(a.as_ref())).collect(),
            ),
            Invoke(_, f, args) => SerExpKind::Invoke(
                Box::new(self.ser_exp(f.as_ref())),
                args.iter().map(|a| self.ser_exp(a.as_ref())).collect(),
            ),
            Lambda(_, pat, body, cap, spec) => SerExpKind::Lambda(
                self.ser_pattern_with_env(pat),
                Box::new(self.ser_exp(body.as_ref())),
                self.ser_capture_kind(cap),
                spec.as_ref().map(|s| Box::new(self.ser_exp(s.as_ref()))),
            ),
            Quant(_, kind, ranges, triggers, cond, body) => SerExpKind::Quant(
                self.ser_quant_kind(kind),
                ranges
                    .iter()
                    .map(|(p, e)| (self.ser_pattern_with_env(p), self.ser_exp(e.as_ref())))
                    .collect(),
                triggers
                    .iter()
                    .map(|ts| ts.iter().map(|t| self.ser_exp(t.as_ref())).collect())
                    .collect(),
                cond.as_ref().map(|c| Box::new(self.ser_exp(c.as_ref()))),
                Box::new(self.ser_exp(body.as_ref())),
            ),
            Block(_, pat, opt_init, body) => SerExpKind::Block(
                self.ser_pattern_with_env(pat),
                opt_init
                    .as_ref()
                    .map(|e| Box::new(self.ser_exp(e.as_ref()))),
                Box::new(self.ser_exp(body.as_ref())),
            ),
            IfElse(_, cond, then_, else_) => SerExpKind::IfElse(
                Box::new(self.ser_exp(cond.as_ref())),
                Box::new(self.ser_exp(then_.as_ref())),
                Box::new(self.ser_exp(else_.as_ref())),
            ),
            Match(_, scrutinee, arms) => SerExpKind::Match(
                Box::new(self.ser_exp(scrutinee.as_ref())),
                arms.iter()
                    .map(|arm| SerMatchArm {
                        pattern: self.ser_pattern_with_env(&arm.pattern),
                        condition: arm.condition.as_ref().map(|c| self.ser_exp(c.as_ref())),
                        body: self.ser_exp(arm.body.as_ref()),
                    })
                    .collect(),
            ),
            Return(_, val) => SerExpKind::Return(Box::new(self.ser_exp(val.as_ref()))),
            Sequence(_, items) => {
                SerExpKind::Sequence(items.iter().map(|e| self.ser_exp(e.as_ref())).collect())
            },
            Loop(_, body) => SerExpKind::Loop(Box::new(self.ser_exp(body.as_ref()))),
            LoopCont(_, nest, is_continue) => SerExpKind::LoopCont(*nest, *is_continue),
            Assign(_, pat, rhs) => SerExpKind::Assign(
                self.ser_pattern_with_env(pat),
                Box::new(self.ser_exp(rhs.as_ref())),
            ),
            Mutate(_, lhs, rhs) => SerExpKind::Mutate(
                Box::new(self.ser_exp(lhs.as_ref())),
                Box::new(self.ser_exp(rhs.as_ref())),
            ),
            SpecBlock(_, spec) => SerExpKind::SpecBlock(self.ser_spec(spec)),
        }
    }

    // -------------------------------------------------------------------------
    // Pattern conversion

    fn ser_pattern_with_env(&self, pat: &Pattern) -> SerPattern {
        use Pattern::*;
        match pat {
            Var(nid, sym) => {
                let ty = self.env.get_node_type(*nid);
                SerPattern::Var(
                    self.env.symbol_pool().string(*sym).to_string(),
                    self.ser_type(&ty),
                )
            },
            Wildcard(_) => SerPattern::Wildcard,
            Tuple(_, pats) => {
                SerPattern::Tuple(pats.iter().map(|p| self.ser_pattern_with_env(p)).collect())
            },
            Struct(_, qinst, variant, pats) => SerPattern::Struct(
                self.struct_inst_name_str(qinst),
                variant
                    .as_ref()
                    .map(|v| self.env.symbol_pool().string(*v).to_string()),
                pats.iter().map(|p| self.ser_pattern_with_env(p)).collect(),
            ),
            LiteralValue(_, val) => SerPattern::LiteralValue(self.ser_value(val)),
            Error(_) => SerPattern::Error,
        }
    }

    // -------------------------------------------------------------------------
    // Operation conversion

    pub fn ser_operation(&self, op: &Operation) -> SerOperation {
        use Operation::*;
        match op {
            MoveFunction(mid, fid) => SerOperation::MoveFunction(self.fun_name_str(*mid, *fid)),
            Pack(mid, sid, variant) => SerOperation::Pack(
                self.struct_name_str(*mid, *sid),
                variant
                    .as_ref()
                    .map(|v| self.env.symbol_pool().string(*v).to_string()),
            ),
            Closure(mid, fid, mask) => {
                SerOperation::Closure(self.fun_name_str(*mid, *fid), mask.bits())
            },
            Tuple => SerOperation::Tuple,
            Select(mid, sid, fid) => {
                SerOperation::Select(self.struct_name_str(*mid, *sid), self.field_name_str(*fid))
            },
            SelectVariants(mid, sid, fields) => SerOperation::SelectVariants(
                self.struct_name_str(*mid, *sid),
                fields.iter().map(|f| self.field_name_str(*f)).collect(),
            ),
            TestVariants(mid, sid, variants) => SerOperation::TestVariants(
                self.struct_name_str(*mid, *sid),
                variants
                    .iter()
                    .map(|v| self.env.symbol_pool().string(*v).to_string())
                    .collect(),
            ),
            SpecFunction(mid, sfid, labels_opt) => {
                let name = self.spec_fun_name_str(*mid, *sfid);
                let labels = labels_opt
                    .as_ref()
                    .map(|ls| ls.iter().map(|l| l.as_usize() as u64).collect());
                SerOperation::SpecFunction(name, labels)
            },
            UpdateField(mid, sid, fid) => SerOperation::UpdateField(
                self.struct_name_str(*mid, *sid),
                self.field_name_str(*fid),
            ),
            Behavior(kind, state) => SerOperation::Behavior(
                self.ser_behavior_kind(kind),
                state.pre.map(|l| l.as_usize() as u64),
                state.post.map(|l| l.as_usize() as u64),
            ),
            Result(idx) => SerOperation::Result(*idx),
            Index => SerOperation::Index,
            Slice => SerOperation::Slice,
            Range => SerOperation::Range,
            Implies => SerOperation::Implies,
            Iff => SerOperation::Iff,
            Identical => SerOperation::Identical,
            Add => SerOperation::Add,
            Sub => SerOperation::Sub,
            Mul => SerOperation::Mul,
            Mod => SerOperation::Mod,
            Div => SerOperation::Div,
            BitOr => SerOperation::BitOr,
            BitAnd => SerOperation::BitAnd,
            Xor => SerOperation::Xor,
            Shl => SerOperation::Shl,
            Shr => SerOperation::Shr,
            And => SerOperation::And,
            Or => SerOperation::Or,
            Eq => SerOperation::Eq,
            Neq => SerOperation::Neq,
            Lt => SerOperation::Lt,
            Gt => SerOperation::Gt,
            Le => SerOperation::Le,
            Ge => SerOperation::Ge,
            Copy => SerOperation::Copy,
            Move => SerOperation::Move,
            Not => SerOperation::Not,
            Cast => SerOperation::Cast,
            Negate => SerOperation::Negate,
            Exists(label) => SerOperation::Exists(label.map(|l| l.as_usize() as u64)),
            BorrowGlobal(kind) => {
                SerOperation::BorrowGlobal(matches!(kind, ReferenceKind::Mutable))
            },
            Borrow(kind) => SerOperation::Borrow(matches!(kind, ReferenceKind::Mutable)),
            Deref => SerOperation::Deref,
            MoveTo => SerOperation::MoveTo,
            MoveFrom => SerOperation::MoveFrom,
            Freeze(explicit) => SerOperation::Freeze(*explicit),
            Abort(kind) => SerOperation::Abort(match kind {
                AbortKind::Code => SerAbortKind::Code,
                AbortKind::Message => SerAbortKind::Message,
            }),
            Vector => SerOperation::Vector,
            Len => SerOperation::Len,
            TypeValue => SerOperation::TypeValue,
            TypeDomain => SerOperation::TypeDomain,
            ResourceDomain => SerOperation::ResourceDomain,
            Global(label) => SerOperation::Global(label.map(|l| l.as_usize() as u64)),
            CanModify => SerOperation::CanModify,
            Old => SerOperation::Old,
            Trace(kind) => SerOperation::Trace(match kind {
                TraceKind::User => SerTraceKind::User,
                TraceKind::Auto => SerTraceKind::Auto,
                TraceKind::SubAuto => SerTraceKind::SubAuto,
            }),
            EmptyVec => SerOperation::EmptyVec,
            SingleVec => SerOperation::SingleVec,
            UpdateVec => SerOperation::UpdateVec,
            ConcatVec => SerOperation::ConcatVec,
            IndexOfVec => SerOperation::IndexOfVec,
            ContainsVec => SerOperation::ContainsVec,
            InRangeRange => SerOperation::InRangeRange,
            InRangeVec => SerOperation::InRangeVec,
            RangeVec => SerOperation::RangeVec,
            MaxU8 => SerOperation::MaxU8,
            MaxU16 => SerOperation::MaxU16,
            MaxU32 => SerOperation::MaxU32,
            MaxU64 => SerOperation::MaxU64,
            MaxU128 => SerOperation::MaxU128,
            MaxU256 => SerOperation::MaxU256,
            Bv2Int => SerOperation::Bv2Int,
            Int2Bv => SerOperation::Int2Bv,
            AbortFlag => SerOperation::AbortFlag,
            AbortCode => SerOperation::AbortCode,
            WellFormed => SerOperation::WellFormed,
            BoxValue => SerOperation::BoxValue,
            UnboxValue => SerOperation::UnboxValue,
            EmptyEventStore => SerOperation::EmptyEventStore,
            ExtendEventStore => SerOperation::ExtendEventStore,
            EventStoreIncludes => SerOperation::EventStoreIncludes,
            EventStoreIncludedIn => SerOperation::EventStoreIncludedIn,
            NoOp => SerOperation::NoOp,
        }
    }

    // -------------------------------------------------------------------------
    // Value conversion

    pub fn ser_value(&self, v: &Value) -> SerValue {
        use Value::*;
        match v {
            Address(addr) => SerValue::Address(self.ser_address_bytes(addr)),
            Number(n) => SerValue::Number(ser_bigint(n)),
            Bool(b) => SerValue::Bool(*b),
            ByteArray(bs) => SerValue::ByteArray(bs.clone()),
            AddressArray(addrs) => {
                SerValue::AddressArray(addrs.iter().map(|a| self.ser_address_bytes(a)).collect())
            },
            Vector(vs) => SerValue::Vector(vs.iter().map(|v| self.ser_value(v)).collect()),
            Tuple(vs) => SerValue::Tuple(vs.iter().map(|v| self.ser_value(v)).collect()),
        }
    }

    fn ser_address_bytes(&self, addr: &Address) -> Vec<u8> {
        match addr {
            Address::Numerical(account_addr) => account_addr.into_bytes().to_vec(),
            Address::Symbolic(sym) => {
                // \xff prefix (not a valid first byte of an AccountAddress) followed by UTF-8.
                let mut bytes = vec![0xFFu8];
                bytes.extend_from_slice(self.env.symbol_pool().string(*sym).as_bytes());
                bytes
            },
        }
    }

    // -------------------------------------------------------------------------
    // Spec conversion

    pub fn ser_spec(&self, spec: &Spec) -> SerSpec {
        SerSpec {
            conditions: spec
                .conditions
                .iter()
                .map(|c| SerCondition {
                    kind: c.kind.to_string(),
                    exp: self.ser_exp(c.exp.as_ref()),
                    additional_exps: c
                        .additional_exps
                        .iter()
                        .map(|e| self.ser_exp(e.as_ref()))
                        .collect(),
                })
                .collect(),
        }
    }

    // -------------------------------------------------------------------------
    // Misc enum conversions

    fn ser_capture_kind(&self, k: &LambdaCaptureKind) -> SerLambdaCaptureKind {
        match k {
            LambdaCaptureKind::Default => SerLambdaCaptureKind::Default,
            LambdaCaptureKind::Copy => SerLambdaCaptureKind::Copy,
            LambdaCaptureKind::Move => SerLambdaCaptureKind::Move,
        }
    }

    fn ser_quant_kind(&self, k: &QuantKind) -> SerQuantKind {
        match k {
            QuantKind::Forall => SerQuantKind::Forall,
            QuantKind::Exists => SerQuantKind::Exists,
            QuantKind::Choose => SerQuantKind::Choose,
            QuantKind::ChooseMin => SerQuantKind::ChooseMin,
        }
    }

    fn ser_behavior_kind(&self, k: &BehaviorKind) -> SerBehaviorKind {
        match k {
            BehaviorKind::RequiresOf => SerBehaviorKind::RequiresOf,
            BehaviorKind::AbortsOf => SerBehaviorKind::AbortsOf,
            BehaviorKind::EnsuresOf => SerBehaviorKind::EnsuresOf,
            BehaviorKind::ModifiesOf => SerBehaviorKind::ModifiesOf,
            BehaviorKind::ResultOf => SerBehaviorKind::ResultOf,
        }
    }

    // -------------------------------------------------------------------------
    // Canonical name helpers

    fn module_name_str(&self, mid: ModuleId) -> String {
        let module = self.env.get_module(mid);
        let name = module.get_name();
        let addr_str = self.address_to_str(name.addr());
        let mod_str = self.env.symbol_pool().string(name.name()).to_string();
        format!("{}::{}", addr_str, mod_str)
    }

    fn address_to_str(&self, addr: &Address) -> String {
        match addr {
            Address::Numerical(account_addr) => {
                format!("0x{}", account_addr.to_canonical_string())
            },
            Address::Symbolic(sym) => {
                format!("@{}", self.env.symbol_pool().string(*sym))
            },
        }
    }

    fn fun_name_str(&self, mid: ModuleId, fid: FunId) -> String {
        let fun_name = self.env.symbol_pool().string(fid.symbol()).to_string();
        format!("{}::{}", self.module_name_str(mid), fun_name)
    }

    fn struct_name_str(&self, mid: ModuleId, sid: StructId) -> String {
        let struct_name = self.env.symbol_pool().string(sid.symbol()).to_string();
        format!("{}::{}", self.module_name_str(mid), struct_name)
    }

    fn struct_inst_name_str(&self, qinst: &QualifiedInstId<StructId>) -> String {
        self.struct_name_str(qinst.module_id, qinst.id)
        // Type instantiation is encoded in the pattern's sub-patterns, not in the name.
    }

    fn spec_fun_name_str(&self, mid: ModuleId, sfid: SpecFunId) -> String {
        let qid = mid.qualified(sfid);
        let decl = self.env.get_spec_fun(qid);
        let fun_name = self.env.symbol_pool().string(decl.name).to_string();
        format!("{}::{}", self.module_name_str(mid), fun_name)
    }

    fn field_name_str(&self, fid: FieldId) -> String {
        self.env.symbol_pool().string(fid.symbol()).to_string()
    }
}

// =============================================================================
// AstDeserializer

/// Converts serialized [`SerFunctionBody`] values back into live [`GlobalEnv`] AST,
/// injecting the reconstructed body into the appropriate function stub so the
/// inliner can expand it when processing downstream packages.
pub struct AstDeserializer<'a> {
    env: &'a mut GlobalEnv,
}

impl<'a> AstDeserializer<'a> {
    pub fn new(env: &'a mut GlobalEnv) -> Self {
        Self { env }
    }

    /// Deserialize all inline function bodies from `bodies` and inject them into
    /// the function stubs belonging to `module_id`.  Errors are returned per-function;
    /// a failure on one function does not prevent others from being injected.
    pub fn inject_inline_bodies(
        &mut self,
        module_id: ModuleId,
        bodies: &InlineFunctionBodies,
    ) -> Vec<(String, anyhow::Error)> {
        let mut errors = vec![];
        for (fun_name, body) in &bodies.functions {
            if let Err(e) = self.inject_function_body(module_id, fun_name, body) {
                errors.push((fun_name.clone(), e));
            }
        }
        errors
    }

    fn inject_function_body(
        &mut self,
        module_id: ModuleId,
        fun_name: &str,
        body: &SerFunctionBody,
    ) -> anyhow::Result<()> {
        let exp = self.deser_exp(&body.body)?;
        let sym = self.env.symbol_pool().make(fun_name);
        let qid = module_id.qualified(FunId::new(sym));
        self.env.set_function_def(qid, exp);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Expression deserialization

    fn deser_exp(&mut self, ser_exp: &SerExp) -> anyhow::Result<crate::ast::Exp> {
        let ty = self.deser_type(&ser_exp.ty)?;
        let loc = self.env.unknown_loc();
        let nid = self.env.new_node(loc, ty);
        let exp_data = self.deser_exp_kind(&ser_exp.kind, nid)?;
        Ok(exp_data.into_exp())
    }

    fn deser_exp_kind(
        &mut self,
        kind: &SerExpKind,
        nid: crate::model::NodeId,
    ) -> anyhow::Result<ExpData> {
        use ExpData as ED;
        use SerExpKind::*;
        Ok(match kind {
            Invalid => ED::Invalid(nid),
            Value(v) => ED::Value(nid, self.deser_value(v)?),
            LocalVar(name) => {
                let sym = self.env.symbol_pool().make(name.as_str());
                ED::LocalVar(nid, sym)
            },
            Temporary(idx) => ED::Temporary(nid, *idx),
            Call(op, args) => {
                let dop = self.deser_operation(op)?;
                let dargs = args
                    .iter()
                    .map(|a| self.deser_exp(a))
                    .collect::<anyhow::Result<_>>()?;
                ED::Call(nid, dop, dargs)
            },
            Invoke(f, args) => {
                let df = self.deser_exp(f)?;
                let dargs = args
                    .iter()
                    .map(|a| self.deser_exp(a))
                    .collect::<anyhow::Result<_>>()?;
                ED::Invoke(nid, df, dargs)
            },
            Lambda(pat, body, cap, spec) => {
                let dpat = self.deser_pattern(pat)?;
                let dbody = self.deser_exp(body)?;
                let dcap = deser_capture_kind(cap);
                let dspec = spec.as_ref().map(|s| self.deser_exp(s)).transpose()?;
                ED::Lambda(nid, dpat, dbody, dcap, dspec)
            },
            Quant(qkind, ranges, triggers, cond, body) => {
                let dkind = deser_quant_kind(qkind);
                let dranges = ranges
                    .iter()
                    .map(|(p, e)| Ok((self.deser_pattern(p)?, self.deser_exp(e)?)))
                    .collect::<anyhow::Result<_>>()?;
                let dtriggers = triggers
                    .iter()
                    .map(|ts| {
                        ts.iter()
                            .map(|t| self.deser_exp(t))
                            .collect::<anyhow::Result<_>>()
                    })
                    .collect::<anyhow::Result<_>>()?;
                let dcond = cond.as_ref().map(|c| self.deser_exp(c)).transpose()?;
                let dbody = self.deser_exp(body)?;
                ED::Quant(nid, dkind, dranges, dtriggers, dcond, dbody)
            },
            Block(pat, opt_init, body) => {
                let dpat = self.deser_pattern(pat)?;
                let dinit = opt_init.as_ref().map(|e| self.deser_exp(e)).transpose()?;
                let dbody = self.deser_exp(body)?;
                ED::Block(nid, dpat, dinit, dbody)
            },
            IfElse(cond, then_, else_) => ED::IfElse(
                nid,
                self.deser_exp(cond)?,
                self.deser_exp(then_)?,
                self.deser_exp(else_)?,
            ),
            Match(scrutinee, arms) => {
                let dscrutinee = self.deser_exp(scrutinee)?;
                let darms = arms
                    .iter()
                    .map(|arm| {
                        let loc = self.env.unknown_loc();
                        Ok(MatchArm {
                            loc,
                            pattern: self.deser_pattern(&arm.pattern)?,
                            condition: arm
                                .condition
                                .as_ref()
                                .map(|c| self.deser_exp(c))
                                .transpose()?,
                            body: self.deser_exp(&arm.body)?,
                        })
                    })
                    .collect::<anyhow::Result<_>>()?;
                ED::Match(nid, dscrutinee, darms)
            },
            Return(val) => ED::Return(nid, self.deser_exp(val)?),
            Sequence(items) => {
                let ditems = items
                    .iter()
                    .map(|e| self.deser_exp(e))
                    .collect::<anyhow::Result<_>>()?;
                ED::Sequence(nid, ditems)
            },
            Loop(body) => ED::Loop(nid, self.deser_exp(body)?),
            LoopCont(nest, is_continue) => ED::LoopCont(nid, *nest, *is_continue),
            Assign(pat, rhs) => ED::Assign(nid, self.deser_pattern(pat)?, self.deser_exp(rhs)?),
            Mutate(lhs, rhs) => ED::Mutate(nid, self.deser_exp(lhs)?, self.deser_exp(rhs)?),
            SpecBlock(spec) => ED::SpecBlock(nid, self.deser_spec(spec)?),
        })
    }

    // -------------------------------------------------------------------------
    // Pattern deserialization

    fn deser_pattern(&mut self, pat: &SerPattern) -> anyhow::Result<Pattern> {
        use SerPattern::*;
        Ok(match pat {
            Var(name, ty) => {
                let dty = self.deser_type(ty)?;
                let loc = self.env.unknown_loc();
                let nid = self.env.new_node(loc, dty);
                let sym = self.env.symbol_pool().make(name.as_str());
                Pattern::Var(nid, sym)
            },
            Wildcard => {
                let loc = self.env.unknown_loc();
                let nid = self.env.new_node(loc, Type::Error);
                Pattern::Wildcard(nid)
            },
            Tuple(pats) => {
                let loc = self.env.unknown_loc();
                let dpats = pats
                    .iter()
                    .map(|p| self.deser_pattern(p))
                    .collect::<anyhow::Result<_>>()?;
                let nid = self.env.new_node(loc, Type::Error);
                Pattern::Tuple(nid, dpats)
            },
            Struct(struct_str, variant, pats) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let loc = self.env.unknown_loc();
                let dpats = pats
                    .iter()
                    .map(|p| self.deser_pattern(p))
                    .collect::<anyhow::Result<_>>()?;
                let nid = self.env.new_node(loc, Type::Struct(mid, sid, vec![]));
                let qinst = QualifiedInstId {
                    module_id: mid,
                    id: sid,
                    inst: vec![],
                };
                let dvariant = variant
                    .as_ref()
                    .map(|v| self.env.symbol_pool().make(v.as_str()));
                Pattern::Struct(nid, qinst, dvariant, dpats)
            },
            LiteralValue(v) => {
                let dv = self.deser_value(v)?;
                let loc = self.env.unknown_loc();
                let nid = self.env.new_node(loc, Type::Error);
                Pattern::LiteralValue(nid, dv)
            },
            Error => {
                let loc = self.env.unknown_loc();
                let nid = self.env.new_node(loc, Type::Error);
                Pattern::Error(nid)
            },
        })
    }

    // -------------------------------------------------------------------------
    // Type deserialization

    fn deser_type(&self, ty: &SerType) -> anyhow::Result<Type> {
        use PrimitiveType as PT;
        use SerType::*;
        Ok(match ty {
            Bool => Type::Primitive(PT::Bool),
            U8 => Type::Primitive(PT::U8),
            U16 => Type::Primitive(PT::U16),
            U32 => Type::Primitive(PT::U32),
            U64 => Type::Primitive(PT::U64),
            U128 => Type::Primitive(PT::U128),
            U256 => Type::Primitive(PT::U256),
            I8 => Type::Primitive(PT::I8),
            I16 => Type::Primitive(PT::I16),
            I32 => Type::Primitive(PT::I32),
            I64 => Type::Primitive(PT::I64),
            I128 => Type::Primitive(PT::I128),
            I256 => Type::Primitive(PT::I256),
            Address => Type::Primitive(PT::Address),
            Signer => Type::Primitive(PT::Signer),
            Num => Type::Primitive(PT::Num),
            Range => Type::Primitive(PT::Range),
            EventStore => Type::Primitive(PT::EventStore),
            Tuple(ts) => Type::Tuple(
                ts.iter()
                    .map(|t| self.deser_type(t))
                    .collect::<anyhow::Result<_>>()?,
            ),
            Vector(inner) => Type::Vector(Box::new(self.deser_type(inner)?)),
            Struct(name, args) => {
                let (mid, sid) = self.resolve_struct(name)?;
                let inst = args
                    .iter()
                    .map(|a| self.deser_type(a))
                    .collect::<anyhow::Result<_>>()?;
                Type::Struct(mid, sid, inst)
            },
            TypeParameter(idx) => Type::TypeParameter(*idx),
            Fun(arg, result, abilities_byte) => {
                let abilities = AbilitySet::from_u8(*abilities_byte).ok_or_else(|| {
                    anyhow::anyhow!("invalid ability bitmask: {}", abilities_byte)
                })?;
                Type::Fun(
                    Box::new(self.deser_type(arg)?),
                    Box::new(self.deser_type(result)?),
                    abilities,
                )
            },
            Reference(is_mut, inner) => {
                let kind = if *is_mut {
                    ReferenceKind::Mutable
                } else {
                    ReferenceKind::Immutable
                };
                Type::Reference(kind, Box::new(self.deser_type(inner)?))
            },
            TypeDomain(inner) => Type::TypeDomain(Box::new(self.deser_type(inner)?)),
            ResourceDomain(name, args) => {
                let (mid, sid) = self.resolve_struct(name)?;
                let inst: Vec<Type> = args
                    .iter()
                    .map(|a| self.deser_type(a))
                    .collect::<anyhow::Result<_>>()?;
                Type::ResourceDomain(mid, sid, if inst.is_empty() { None } else { Some(inst) })
            },
            Unknown => Type::Error,
        })
    }

    // -------------------------------------------------------------------------
    // Operation deserialization

    fn deser_operation(&mut self, op: &SerOperation) -> anyhow::Result<Operation> {
        use SerOperation::*;
        Ok(match op {
            MoveFunction(name) => {
                let (mid, fid) = self.resolve_function(name)?;
                Operation::MoveFunction(mid, fid)
            },
            Pack(struct_str, variant) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let dvariant = variant
                    .as_ref()
                    .map(|v| self.env.symbol_pool().make(v.as_str()));
                Operation::Pack(mid, sid, dvariant)
            },
            Closure(name, mask_bits) => {
                let (mid, fid) = self.resolve_function(name)?;
                use move_core_types::function::ClosureMask;
                Operation::Closure(mid, fid, ClosureMask::new(*mask_bits))
            },
            Tuple => Operation::Tuple,
            Select(struct_str, field_name) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let fsym = self.env.symbol_pool().make(field_name.as_str());
                Operation::Select(mid, sid, FieldId::new(fsym))
            },
            SelectVariants(struct_str, field_names) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let fids = field_names
                    .iter()
                    .map(|n| FieldId::new(self.env.symbol_pool().make(n.as_str())))
                    .collect();
                Operation::SelectVariants(mid, sid, fids)
            },
            TestVariants(struct_str, variant_names) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let vsyms = variant_names
                    .iter()
                    .map(|n| self.env.symbol_pool().make(n.as_str()))
                    .collect();
                Operation::TestVariants(mid, sid, vsyms)
            },
            SpecFunction(name, _labels) => {
                // SpecFunction with memory labels: labels are spec-only and not
                // needed for inline function bodies.  Reconstruct without labels.
                let (mid, sfid) = self.resolve_spec_fun(name)?;
                Operation::SpecFunction(mid, sfid, None)
            },
            UpdateField(struct_str, field_name) => {
                let (mid, sid) = self.resolve_struct(struct_str)?;
                let fsym = self.env.symbol_pool().make(field_name.as_str());
                Operation::UpdateField(mid, sid, FieldId::new(fsym))
            },
            Behavior(kind, pre, post) => {
                use crate::model::GlobalId;
                Operation::Behavior(deser_behavior_kind(kind), BehaviorState {
                    pre: pre.map(|v| GlobalId::new(v as usize)),
                    post: post.map(|v| GlobalId::new(v as usize)),
                })
            },
            Result(idx) => Operation::Result(*idx),
            Index => Operation::Index,
            Slice => Operation::Slice,
            Range => Operation::Range,
            Implies => Operation::Implies,
            Iff => Operation::Iff,
            Identical => Operation::Identical,
            Add => Operation::Add,
            Sub => Operation::Sub,
            Mul => Operation::Mul,
            Mod => Operation::Mod,
            Div => Operation::Div,
            BitOr => Operation::BitOr,
            BitAnd => Operation::BitAnd,
            Xor => Operation::Xor,
            Shl => Operation::Shl,
            Shr => Operation::Shr,
            And => Operation::And,
            Or => Operation::Or,
            Eq => Operation::Eq,
            Neq => Operation::Neq,
            Lt => Operation::Lt,
            Gt => Operation::Gt,
            Le => Operation::Le,
            Ge => Operation::Ge,
            Copy => Operation::Copy,
            Move => Operation::Move,
            Not => Operation::Not,
            Cast => Operation::Cast,
            Negate => Operation::Negate,
            Exists(label) => {
                use crate::model::GlobalId;
                Operation::Exists(label.map(|v| GlobalId::new(v as usize)))
            },
            BorrowGlobal(is_mut) => Operation::BorrowGlobal(
                if *is_mut {
                    ReferenceKind::Mutable
                } else {
                    ReferenceKind::Immutable
                },
            ),
            Borrow(is_mut) => Operation::Borrow(
                if *is_mut {
                    ReferenceKind::Mutable
                } else {
                    ReferenceKind::Immutable
                },
            ),
            Deref => Operation::Deref,
            MoveTo => Operation::MoveTo,
            MoveFrom => Operation::MoveFrom,
            Freeze(explicit) => Operation::Freeze(*explicit),
            Abort(kind) => Operation::Abort(match kind {
                SerAbortKind::Code => AbortKind::Code,
                SerAbortKind::Message => AbortKind::Message,
            }),
            Vector => Operation::Vector,
            Len => Operation::Len,
            TypeValue => Operation::TypeValue,
            TypeDomain => Operation::TypeDomain,
            ResourceDomain => Operation::ResourceDomain,
            Global(label) => {
                use crate::model::GlobalId;
                Operation::Global(label.map(|v| GlobalId::new(v as usize)))
            },
            CanModify => Operation::CanModify,
            Old => Operation::Old,
            Trace(kind) => Operation::Trace(match kind {
                SerTraceKind::User => TraceKind::User,
                SerTraceKind::Auto => TraceKind::Auto,
                SerTraceKind::SubAuto => TraceKind::SubAuto,
            }),
            EmptyVec => Operation::EmptyVec,
            SingleVec => Operation::SingleVec,
            UpdateVec => Operation::UpdateVec,
            ConcatVec => Operation::ConcatVec,
            IndexOfVec => Operation::IndexOfVec,
            ContainsVec => Operation::ContainsVec,
            InRangeRange => Operation::InRangeRange,
            InRangeVec => Operation::InRangeVec,
            RangeVec => Operation::RangeVec,
            MaxU8 => Operation::MaxU8,
            MaxU16 => Operation::MaxU16,
            MaxU32 => Operation::MaxU32,
            MaxU64 => Operation::MaxU64,
            MaxU128 => Operation::MaxU128,
            MaxU256 => Operation::MaxU256,
            Bv2Int => Operation::Bv2Int,
            Int2Bv => Operation::Int2Bv,
            AbortFlag => Operation::AbortFlag,
            AbortCode => Operation::AbortCode,
            WellFormed => Operation::WellFormed,
            BoxValue => Operation::BoxValue,
            UnboxValue => Operation::UnboxValue,
            EmptyEventStore => Operation::EmptyEventStore,
            ExtendEventStore => Operation::ExtendEventStore,
            EventStoreIncludes => Operation::EventStoreIncludes,
            EventStoreIncludedIn => Operation::EventStoreIncludedIn,
            NoOp => Operation::NoOp,
        })
    }

    // -------------------------------------------------------------------------
    // Value deserialization

    fn deser_value(&self, v: &SerValue) -> anyhow::Result<Value> {
        use SerValue::*;
        Ok(match v {
            Address(bytes) => Value::Address(deser_address_bytes(bytes, self.env.symbol_pool())?),
            Number(bytes) => Value::Number(deser_bigint(bytes)?),
            Bool(b) => Value::Bool(*b),
            ByteArray(bs) => Value::ByteArray(bs.clone()),
            AddressArray(addrs) => Value::AddressArray(
                addrs
                    .iter()
                    .map(|a| deser_address_bytes(a, self.env.symbol_pool()))
                    .collect::<anyhow::Result<_>>()?,
            ),
            Vector(vs) => Value::Vector(
                vs.iter()
                    .map(|v| self.deser_value(v))
                    .collect::<anyhow::Result<_>>()?,
            ),
            Tuple(vs) => Value::Tuple(
                vs.iter()
                    .map(|v| self.deser_value(v))
                    .collect::<anyhow::Result<_>>()?,
            ),
        })
    }

    // -------------------------------------------------------------------------
    // Spec deserialization

    fn deser_spec(&mut self, spec: &SerSpec) -> anyhow::Result<Spec> {
        use crate::ast::{Condition, ConditionKind};
        fn parse_condition_kind(s: &str) -> ConditionKind {
            match s {
                "assert" => ConditionKind::Assert,
                "assume" => ConditionKind::Assume,
                "ensures" => ConditionKind::Ensures,
                "requires" => ConditionKind::Requires,
                "aborts_if" => ConditionKind::AbortsIf,
                "aborts_with" => ConditionKind::AbortsWith,
                "succeeds_if" => ConditionKind::SucceedsIf,
                "modifies" => ConditionKind::Modifies,
                "emits" => ConditionKind::Emits,
                "decreases" => ConditionKind::Decreases,
                "invariant" => ConditionKind::LoopInvariant,
                _ => ConditionKind::Assert,
            }
        }
        let conditions = spec
            .conditions
            .iter()
            .map(|c| {
                let kind = parse_condition_kind(&c.kind);
                let loc = self.env.unknown_loc();
                Ok(Condition {
                    loc,
                    kind,
                    properties: BTreeMap::new(),
                    exp: self.deser_exp(&c.exp)?,
                    additional_exps: c
                        .additional_exps
                        .iter()
                        .map(|e| self.deser_exp(e))
                        .collect::<anyhow::Result<_>>()?,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Spec {
            conditions,
            ..Default::default()
        })
    }

    // -------------------------------------------------------------------------
    // Name resolution helpers

    /// Resolve `"0x<hex>::<module_name>"` or `"@<sym>::<module_name>"` → `ModuleId`.
    fn resolve_module(&self, module_str: &str) -> anyhow::Result<ModuleId> {
        let (addr_part, name_part) = split_first_component(module_str)?;
        let addr = parse_address(&addr_part, self.env.symbol_pool())?;
        let name_sym = self.env.symbol_pool().make(name_part);
        let mod_name = ModuleName::new(addr, name_sym);
        self.env
            .find_module(&mod_name)
            .map(|m| m.get_id())
            .ok_or_else(|| anyhow::anyhow!("module not found in GlobalEnv: {}", module_str))
    }

    /// Resolve `"<module_str>::<StructName>"` → `(ModuleId, StructId)`.
    fn resolve_struct(&self, qualified: &str) -> anyhow::Result<(ModuleId, StructId)> {
        let (module_part, struct_name) = split_last_component(qualified)?;
        let mid = self.resolve_module(module_part)?;
        let sym = self.env.symbol_pool().make(struct_name);
        self.env
            .get_module(mid)
            .find_struct(sym)
            .map(|s| (mid, s.get_id()))
            .ok_or_else(|| anyhow::anyhow!("struct not found: {}", qualified))
    }

    /// Resolve `"<module_str>::<fun_name>"` → `(ModuleId, FunId)`.
    fn resolve_function(&self, qualified: &str) -> anyhow::Result<(ModuleId, FunId)> {
        let (module_part, fun_name) = split_last_component(qualified)?;
        let mid = self.resolve_module(module_part)?;
        let sym = self.env.symbol_pool().make(fun_name);
        self.env
            .get_module(mid)
            .find_function(sym)
            .map(|f| (mid, f.get_id()))
            .ok_or_else(|| anyhow::anyhow!("function not found: {}", qualified))
    }

    /// Resolve `"<module_str>::<spec_fun_name>"` → `(ModuleId, SpecFunId)`.
    fn resolve_spec_fun(&self, qualified: &str) -> anyhow::Result<(ModuleId, SpecFunId)> {
        let (module_part, fun_name) = split_last_component(qualified)?;
        let mid = self.resolve_module(module_part)?;
        let target_sym = self.env.symbol_pool().make(fun_name);
        let module = self.env.get_module(mid);
        for (idx, decl) in module.get_spec_funs() {
            if decl.name == target_sym {
                return Ok((mid, *idx));
            }
        }
        anyhow::bail!("spec function not found: {}", qualified)
    }
}

// =============================================================================
// Free helper functions

/// Split `"0x<hex>::<module>"` into `("0x<hex>", "<module>")`.
/// Handles both numerical `"0x..."` and symbolic `"@..."` addresses.
fn split_first_component(s: &str) -> anyhow::Result<(String, &str)> {
    // The address part ends at the first `::`.
    let sep = s
        .find("::")
        .ok_or_else(|| anyhow::anyhow!("missing '::' in module string: {}", s))?;
    Ok((s[..sep].to_string(), &s[sep + 2..]))
}

/// Split `"<prefix>::<last>"` into `("<prefix>", "<last>")` using the last `::`.
fn split_last_component(s: &str) -> anyhow::Result<(&str, &str)> {
    let sep = s
        .rfind("::")
        .ok_or_else(|| anyhow::anyhow!("missing '::' in qualified name: {}", s))?;
    Ok((&s[..sep], &s[sep + 2..]))
}

/// Parse a canonical address string back to an [`Address`].
/// Accepts `"0x<64_hex_chars>"` (numerical) or `"@<name>"` (symbolic).
fn parse_address(s: &str, pool: &crate::symbol::SymbolPool) -> anyhow::Result<Address> {
    if let Some(hex) = s.strip_prefix("0x") {
        let account_addr = AccountAddress::from_hex(hex)
            .map_err(|e| anyhow::anyhow!("invalid account address '{}': {}", s, e))?;
        Ok(Address::Numerical(account_addr))
    } else if let Some(sym_name) = s.strip_prefix('@') {
        Ok(Address::Symbolic(pool.make(sym_name)))
    } else {
        anyhow::bail!("unrecognized address format: {}", s)
    }
}

/// Deserialize a serialized address (32-byte numerical or `\xff`+UTF-8 symbolic).
fn deser_address_bytes(bytes: &[u8], pool: &crate::symbol::SymbolPool) -> anyhow::Result<Address> {
    if bytes.first() == Some(&0xFF) {
        let name = std::str::from_utf8(&bytes[1..])
            .map_err(|e| anyhow::anyhow!("invalid UTF-8 in symbolic address: {}", e))?;
        Ok(Address::Symbolic(pool.make(name)))
    } else {
        let arr: [u8; AccountAddress::LENGTH] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 32-byte address, got {} bytes", bytes.len()))?;
        Ok(Address::Numerical(AccountAddress::new(arr)))
    }
}

// =============================================================================
// BigInt serialization / deserialization

/// Serialize a [`BigInt`] as `[sign_byte, be_magnitude...]`.
/// `sign_byte` is `0` for non-negative, `1` for negative.
fn ser_bigint(n: &BigInt) -> Vec<u8> {
    let is_negative = n.is_negative();
    let (_, magnitude) = n.to_bytes_be();
    let mut result = Vec::with_capacity(1 + magnitude.len());
    result.push(if is_negative { 1u8 } else { 0u8 });
    result.extend_from_slice(&magnitude);
    result
}

fn deser_bigint(bytes: &[u8]) -> anyhow::Result<BigInt> {
    if bytes.is_empty() {
        anyhow::bail!("empty bigint bytes");
    }
    let is_negative = bytes[0] != 0;
    let magnitude = &bytes[1..];
    let abs = BigInt::from_bytes_be(num::bigint::Sign::Plus, magnitude);
    Ok(if is_negative { -abs } else { abs })
}

// =============================================================================
// Misc free deserialization helpers

fn deser_capture_kind(k: &SerLambdaCaptureKind) -> LambdaCaptureKind {
    match k {
        SerLambdaCaptureKind::Default => LambdaCaptureKind::Default,
        SerLambdaCaptureKind::Copy => LambdaCaptureKind::Copy,
        SerLambdaCaptureKind::Move => LambdaCaptureKind::Move,
    }
}

fn deser_quant_kind(k: &SerQuantKind) -> QuantKind {
    match k {
        SerQuantKind::Forall => QuantKind::Forall,
        SerQuantKind::Exists => QuantKind::Exists,
        SerQuantKind::Choose => QuantKind::Choose,
        SerQuantKind::ChooseMin => QuantKind::ChooseMin,
    }
}

fn deser_behavior_kind(k: &SerBehaviorKind) -> BehaviorKind {
    match k {
        SerBehaviorKind::RequiresOf => BehaviorKind::RequiresOf,
        SerBehaviorKind::AbortsOf => BehaviorKind::AbortsOf,
        SerBehaviorKind::EnsuresOf => BehaviorKind::EnsuresOf,
        SerBehaviorKind::ModifiesOf => BehaviorKind::ModifiesOf,
        SerBehaviorKind::ResultOf => BehaviorKind::ResultOf,
    }
}

// =============================================================================
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser_bigint_zero() {
        let n = BigInt::from(0);
        let bytes = ser_bigint(&n);
        assert_eq!(bytes[0], 0);
        let back = deser_bigint(&bytes).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn test_ser_bigint_positive() {
        let n = BigInt::from(255u64);
        let bytes = ser_bigint(&n);
        assert_eq!(bytes[0], 0);
        let back = deser_bigint(&bytes).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn test_ser_bigint_negative() {
        let n = BigInt::from(-1i64);
        let bytes = ser_bigint(&n);
        assert_eq!(bytes[0], 1);
        let back = deser_bigint(&bytes).unwrap();
        assert_eq!(back, n);
    }

    #[test]
    fn test_inline_bodies_roundtrip() {
        let bodies = InlineFunctionBodies {
            functions: BTreeMap::new(),
        };
        let bytes = bodies.to_bytes().unwrap();
        let decoded = InlineFunctionBodies::from_bytes(&bytes).unwrap();
        assert_eq!(bodies, decoded);
    }

    #[test]
    fn test_ser_exp_determinism() {
        let exp1 = SerExp {
            ty: SerType::Bool,
            kind: SerExpKind::Value(SerValue::Bool(true)),
        };
        let exp2 = exp1.clone();
        let b1 = bcs::to_bytes(&exp1).unwrap();
        let b2 = bcs::to_bytes(&exp2).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn test_address_roundtrip_numerical() {
        let addr = AccountAddress::from_hex(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap();
        let ser = Address::Numerical(addr);
        // Serialize then deserialize via the byte encoding.
        let bytes = addr.into_bytes().to_vec();
        let pool = crate::symbol::SymbolPool::new();
        let back = deser_address_bytes(&bytes, &pool).unwrap();
        assert_eq!(ser, back);
    }

    #[test]
    fn test_split_last_component() {
        let (prefix, last) = split_last_component("0x1::module::Struct").unwrap();
        assert_eq!(prefix, "0x1::module");
        assert_eq!(last, "Struct");
    }
}
