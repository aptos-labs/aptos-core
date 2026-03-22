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
//! | `NodeId` (per-compilation opaque index) | Dropped entirely |
//! | `Symbol` (interned string index) | Resolved to the string content |
//! | `ModuleId` (per-compilation opaque index) | Resolved to `"0x<hex>::<name>"` |
//! | `StructId(Symbol)`, `FunId(Symbol)`, `FieldId(Symbol)` | Resolved to the symbol string |
//! | `SpecFunId(RawIndex)` | Resolved to the spec function's name string |
//! | `Address::Numerical` | 32-byte big-endian hex with `0x` prefix |
//! | `Address::Symbolic` | `@<sym>` |
//! | `BigInt` | `[sign, be_magnitude...]` bytes |

use crate::{
    ast::{
        AbortKind, BehaviorKind, ExpData, LambdaCaptureKind, Operation, Pattern, QuantKind, Spec,
        TraceKind, Value,
    },
    model::{FieldId, FunId, GlobalEnv, ModuleId, QualifiedInstId, SpecFunId, StructId},
    ty::{PrimitiveType, ReferenceKind, Type},
};
use move_core_types::metadata::Metadata;
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
    /// The function body expression.
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
}

// =============================================================================
// SerExp

/// Serializable expression — mirrors [`ExpData`] with `NodeId` dropped, `Symbol`
/// resolved to strings, and module/struct/function IDs resolved to canonical names.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SerExp {
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
    /// 32-byte big-endian account address.
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
// SerSpec (simplified — enough for assert!/assume! in inline function bodies)

/// Simplified serializable spec.  Only the pieces that can appear inside
/// inline function bodies (assert!/assume! expanding to SpecBlock) are needed.
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
// AstSerializer — conversion from live AST to serialized form

/// Converts a live [`GlobalEnv`] AST into the deterministically serializable
/// stripped form.
pub struct AstSerializer<'a> {
    env: &'a GlobalEnv,
}

impl<'a> AstSerializer<'a> {
    pub fn new(env: &'a GlobalEnv) -> Self {
        Self { env }
    }

    // -------------------------------------------------------------------------
    // Public entry points

    /// Serialize all `public inline` functions in the given module into an
    /// [`InlineFunctionBodies`] map.  Returns `None` if the module has no
    /// inline functions with bodies.
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
            Error | Var(_) => SerType::Bool, // shouldn't appear after type-checking
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
    // Expression conversion

    pub fn ser_exp(&self, exp: &ExpData) -> SerExp {
        use ExpData::*;
        match exp {
            Invalid(_) => SerExp::Invalid,
            Value(_, v) => SerExp::Value(self.ser_value(v)),
            LocalVar(_, sym) => SerExp::LocalVar(self.env.symbol_pool().string(*sym).to_string()),
            Temporary(_, idx) => SerExp::Temporary(*idx),
            Call(_, op, args) => SerExp::Call(
                self.ser_operation(op),
                args.iter().map(|a| self.ser_exp(a.as_ref())).collect(),
            ),
            Invoke(_, f, args) => SerExp::Invoke(
                Box::new(self.ser_exp(f.as_ref())),
                args.iter().map(|a| self.ser_exp(a.as_ref())).collect(),
            ),
            Lambda(_, pat, body, cap, spec) => SerExp::Lambda(
                self.ser_pattern_with_env(pat),
                Box::new(self.ser_exp(body.as_ref())),
                self.ser_capture_kind(cap),
                spec.as_ref().map(|s| Box::new(self.ser_exp(s.as_ref()))),
            ),
            Quant(_, kind, ranges, triggers, cond, body) => SerExp::Quant(
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
            Block(_, pat, opt_init, body) => SerExp::Block(
                self.ser_pattern_with_env(pat),
                opt_init
                    .as_ref()
                    .map(|e| Box::new(self.ser_exp(e.as_ref()))),
                Box::new(self.ser_exp(body.as_ref())),
            ),
            IfElse(_, cond, then_, else_) => SerExp::IfElse(
                Box::new(self.ser_exp(cond.as_ref())),
                Box::new(self.ser_exp(then_.as_ref())),
                Box::new(self.ser_exp(else_.as_ref())),
            ),
            Match(_, scrutinee, arms) => SerExp::Match(
                Box::new(self.ser_exp(scrutinee.as_ref())),
                arms.iter()
                    .map(|arm| SerMatchArm {
                        pattern: self.ser_pattern_with_env(&arm.pattern),
                        condition: arm.condition.as_ref().map(|c| self.ser_exp(c.as_ref())),
                        body: self.ser_exp(arm.body.as_ref()),
                    })
                    .collect(),
            ),
            Return(_, val) => SerExp::Return(Box::new(self.ser_exp(val.as_ref()))),
            Sequence(_, items) => {
                SerExp::Sequence(items.iter().map(|e| self.ser_exp(e.as_ref())).collect())
            },
            Loop(_, body) => SerExp::Loop(Box::new(self.ser_exp(body.as_ref()))),
            LoopCont(_, nest, is_continue) => SerExp::LoopCont(*nest, *is_continue),
            Assign(_, pat, rhs) => SerExp::Assign(
                self.ser_pattern_with_env(pat),
                Box::new(self.ser_exp(rhs.as_ref())),
            ),
            Mutate(_, lhs, rhs) => SerExp::Mutate(
                Box::new(self.ser_exp(lhs.as_ref())),
                Box::new(self.ser_exp(rhs.as_ref())),
            ),
            SpecBlock(_, spec) => SerExp::SpecBlock(self.ser_spec(spec)),
        }
    }

    // -------------------------------------------------------------------------
    // Pattern conversion
    //
    // Patterns carry a NodeId from which we read the node's type via the GlobalEnv.

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
            Address(addr) => SerValue::Address(self.ser_address(addr)),
            Number(n) => SerValue::Number(ser_bigint(n)),
            Bool(b) => SerValue::Bool(*b),
            ByteArray(bs) => SerValue::ByteArray(bs.clone()),
            AddressArray(addrs) => {
                SerValue::AddressArray(addrs.iter().map(|a| self.ser_address(a)).collect())
            },
            Vector(vs) => SerValue::Vector(vs.iter().map(|v| self.ser_value(v)).collect()),
            Tuple(vs) => SerValue::Tuple(vs.iter().map(|v| self.ser_value(v)).collect()),
        }
    }

    fn ser_address(&self, addr: &crate::ast::Address) -> Vec<u8> {
        use crate::ast::Address;
        match addr {
            Address::Numerical(account_addr) => account_addr.into_bytes().to_vec(),
            Address::Symbolic(sym) => {
                // Encode as `\xff` prefix (invalid account address byte) followed by UTF-8 name.
                // This distinguishes symbolic from numerical addresses.
                let mut bytes = vec![0xFF];
                bytes.extend_from_slice(self.env.symbol_pool().string(*sym).as_bytes());
                bytes
            },
        }
    }

    // -------------------------------------------------------------------------
    // Spec conversion

    pub fn ser_spec(&self, spec: &Spec) -> SerSpec {
        let conditions = spec
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
            .collect();
        SerSpec { conditions }
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

    /// Returns `"0x<hex32>::<module_name>"` or `"@<symbolic>::<module_name>"`.
    fn module_name_str(&self, mid: ModuleId) -> String {
        let module = self.env.get_module(mid);
        let name = module.get_name();
        let addr_str = self.ser_address_to_str(name.addr());
        let mod_str = self.env.symbol_pool().string(name.name()).to_string();
        format!("{}::{}", addr_str, mod_str)
    }

    fn ser_address_to_str(&self, addr: &crate::ast::Address) -> String {
        use crate::ast::Address;
        match addr {
            Address::Numerical(account_addr) => {
                format!("0x{}", account_addr.to_canonical_string())
            },
            Address::Symbolic(sym) => {
                format!("@{}", self.env.symbol_pool().string(*sym))
            },
        }
    }

    /// Returns `"<module_name_str>::<fun_name>"`.
    fn fun_name_str(&self, mid: ModuleId, fid: FunId) -> String {
        let fun_name = self.env.symbol_pool().string(fid.symbol()).to_string();
        format!("{}::{}", self.module_name_str(mid), fun_name)
    }

    /// Returns `"<module_name_str>::<struct_name>"`.
    fn struct_name_str(&self, mid: ModuleId, sid: StructId) -> String {
        let struct_name = self.env.symbol_pool().string(sid.symbol()).to_string();
        format!("{}::{}", self.module_name_str(mid), struct_name)
    }

    /// Returns `"<struct_name_str>"` possibly with type instantiation.
    fn struct_inst_name_str(&self, qinst: &QualifiedInstId<StructId>) -> String {
        let base = self.struct_name_str(qinst.module_id, qinst.id);
        if qinst.inst.is_empty() {
            base
        } else {
            let args: Vec<String> = qinst.inst.iter().map(|t| self.ser_type_to_str(t)).collect();
            format!("{}<{}>", base, args.join(", "))
        }
    }

    /// Returns `"<module_name_str>::<spec_fun_name>"`.
    fn spec_fun_name_str(&self, mid: ModuleId, sfid: SpecFunId) -> String {
        let qid = mid.qualified(sfid);
        let decl = self.env.get_spec_fun(qid);
        let fun_name = self.env.symbol_pool().string(decl.name).to_string();
        format!("{}::{}", self.module_name_str(mid), fun_name)
    }

    /// Returns the field name string.
    fn field_name_str(&self, fid: FieldId) -> String {
        self.env.symbol_pool().string(fid.symbol()).to_string()
    }

    /// Returns a human-readable type string (used in struct instantiation names).
    fn ser_type_to_str(&self, ty: &Type) -> String {
        // Reuse the serialized form's structure to produce a canonical string.
        match self.ser_type(ty) {
            SerType::Struct(name, args) if args.is_empty() => name,
            SerType::Struct(name, args) => {
                let arg_strs: Vec<String> = args.iter().map(|a| format!("{:?}", a)).collect();
                format!("{}<{}>", name, arg_strs.join(", "))
            },
            other => format!("{:?}", other),
        }
    }
}

// =============================================================================
// BigInt serialization

/// Serialize a `BigInt` as `[sign_byte, be_magnitude_bytes...]`.
/// `sign_byte` is 0 for non-negative, 1 for negative.
fn ser_bigint(n: &BigInt) -> Vec<u8> {
    let is_negative = n.is_negative();
    let (_, magnitude) = n.to_bytes_be();
    let mut result = Vec::with_capacity(1 + magnitude.len());
    result.push(if is_negative { 1u8 } else { 0u8 });
    result.extend_from_slice(&magnitude);
    result
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
        assert_eq!(bytes[0], 0); // non-negative
    }

    #[test]
    fn test_ser_bigint_positive() {
        let n = BigInt::from(255u64);
        let bytes = ser_bigint(&n);
        assert_eq!(bytes[0], 0); // non-negative
        assert_eq!(bytes[1], 255);
    }

    #[test]
    fn test_ser_bigint_negative() {
        let n = BigInt::from(-1i64);
        let bytes = ser_bigint(&n);
        assert_eq!(bytes[0], 1); // negative
        assert_eq!(bytes[1], 1); // magnitude 1
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
        // Two structurally identical SerExp values must serialize to the same bytes.
        let exp1 = SerExp::Value(SerValue::Bool(true));
        let exp2 = SerExp::Value(SerValue::Bool(true));
        let b1 = bcs::to_bytes(&exp1).unwrap();
        let b2 = bcs::to_bytes(&exp2).unwrap();
        assert_eq!(b1, b2);
    }
}
