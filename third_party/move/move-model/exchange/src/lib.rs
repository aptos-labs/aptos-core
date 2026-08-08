// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! External exchange format for Move stackless bytecode and specifications.
//!
//! The data structures in this crate describe a Move module — its structs,
//! functions as basic-block CFGs of stackless bytecode, loop metadata, and
//! specification contracts — for exchange with external tools.  The primary
//! consumer today is the Lean formalization of the Move Prover
//! (`third_party/move/lean`), fed through the Aptos CLI's `aptos move exchange`
//! command.
//!
//! **These types are the normative schema.**  The wire format is JSON,
//! exactly as serde derives it from the definitions below:
//!
//! - structs serialize as objects with their snake_case field names,
//!   e.g. `{"header": 0, "members": [0, 1], ...}`;
//! - enums are externally tagged with snake_case variant names:
//!   a unit variant is a bare string (`"add"`, `"nop"`), a newtype variant
//!   is a single-key object whose payload is the value itself
//!   (`{"move_from": 0}`, `{"ret": [7]}`), and a tuple variant is a
//!   single-key object with the argument list
//!   (`{"load": [1, {"u64": "5"}]}`, `{"call": [[2], "eq", [0, 1]]}`).
//!   Note that the variant *arity* determines array-wrapping: changing a
//!   newtype variant to a tuple variant changes the wire shape, which is
//!   why the tests below pin exact wire fragments.
//!
//! Conventions:
//!
//! - All identifiers (locals, blocks, resources, functions, fields) are
//!   dense per-module indices in declaration resp. code-layout order; names
//!   in [`Struct`] are documentation only.
//! - `u64` constants travel as decimal strings so that consumers without
//!   exact 64-bit JSON numbers (e.g. JavaScript) stay precise; addresses
//!   travel as `0x`-prefixed hex-literal strings.
//! - Block `0` is the entry block; block ids follow code layout, so a
//!   control-flow edge to a lower-or-equal id is a loop back edge.
//! - Typing assumptions are not part of contracts: producers do not
//!   synthesize them; consumers derive well-formedness from the declared
//!   types ([`Function::locals`], [`Function::returns`], [`Field::ty`]).
//!
//! Version history: version 1 was an ad-hoc array encoding predating this
//! crate; version 2 introduced this format with the spec-expression tag
//! `temp` for locals; version 3 renames that tag to `local`; version 4
//! types quantifier binders with their domain, removes the
//! `is_u64`/`is_address`/`is_struct` well-formedness tests (consumers
//! derive well-formedness from declared types), and gives an empty
//! `aborts_if` list the prover's partial semantics (no claim); version 5
//! adds vectors (the `vector` type, the `vec_*` value operations, the
//! `borrow_vec_elem` reference operation, and the spec-level
//! `len`/`index`).

pub mod check;

use serde::{Deserialize, Serialize};

/// The current version of the exchange format.
pub const FORMAT_VERSION: u64 = 5;

/// Index of a local of a function (a `LocalIndex` in move-model terms).
/// Parameters come first.
pub type LocalIndex = usize;
/// Index of a basic block of a function, in code-layout order.
pub type BlockId = usize;
/// Index of a struct declaration of the module.
pub type ResourceId = usize;
/// Index of a function declaration of the module.
pub type FunId = usize;
/// Offset of a field within its struct declaration.
pub type FieldOffset = usize;
/// A memory snapshot label; label `0` is the pre-state of the function
/// (the state `old(..)` refers to).
pub type MemLabel = u64;

/// A Move module.  The top-level object of an exchange document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    /// The exchange format version; see [`FORMAT_VERSION`].
    pub version: u64,
    pub structs: Vec<Struct>,
    pub funs: Vec<Function>,
}

impl Module {
    /// Creates a module stamped with the current [`FORMAT_VERSION`].
    pub fn new(structs: Vec<Struct>, funs: Vec<Function>) -> Self {
        Self {
            version: FORMAT_VERSION,
            structs,
            funs,
        }
    }

    /// Checks that a deserialized module carries the supported version.
    pub fn check_version(&self) -> Result<(), String> {
        if self.version == FORMAT_VERSION {
            Ok(())
        } else {
            Err(format!(
                "unsupported exchange format version {}, expected {}",
                self.version, FORMAT_VERSION
            ))
        }
    }

    /// Pretty-prints the module as JSON with a fill layout: any subtree
    /// whose compact rendering fits on the line is inlined (e.g.
    /// `{"call": [[2], {"move_from": 0}, [1]]}`); only larger ones break
    /// per element.  No trailing newline.
    pub fn to_pretty_json(&self) -> String {
        let value = serde_json::to_value(self).expect("serialization succeeds");
        let mut out = String::new();
        pretty_json(&Measured::of(&value), 0, &mut out);
        out
    }
}

/// Line width for [`Module::to_pretty_json`]'s inlining decision.
const PRETTY_WIDTH: usize = 100;

/// A JSON subtree with the length of its one-line rendering (spaces after
/// `:` and `,`) measured once per node, so the fill layout is linear in
/// the document size.
enum Measured {
    /// A scalar, pre-rendered.
    Scalar(String),
    Array(Vec<Measured>, usize),
    /// Entries with pre-rendered (quoted) keys.
    Object(Vec<(String, Measured)>, usize),
}

impl Measured {
    fn of(v: &serde_json::Value) -> Measured {
        match v {
            serde_json::Value::Array(items) => {
                let items: Vec<Measured> = items.iter().map(Measured::of).collect();
                let sep = 2 * items.len().saturating_sub(1);
                let len = 2 + items.iter().map(Measured::len).sum::<usize>() + sep;
                Measured::Array(items, len)
            },
            serde_json::Value::Object(map) => {
                let entries: Vec<(String, Measured)> = map
                    .iter()
                    .map(|(k, val)| {
                        (
                            serde_json::Value::String(k.clone()).to_string(),
                            Measured::of(val),
                        )
                    })
                    .collect();
                let sep = 2 * entries.len().saturating_sub(1);
                let len = 2
                    + entries
                        .iter()
                        .map(|(k, m)| k.len() + 2 + m.len())
                        .sum::<usize>()
                    + sep;
                Measured::Object(entries, len)
            },
            v => Measured::Scalar(v.to_string()),
        }
    }

    /// The length of the one-line rendering.
    fn len(&self) -> usize {
        match self {
            Measured::Scalar(s) => s.len(),
            Measured::Array(_, len) | Measured::Object(_, len) => *len,
        }
    }

    /// Writes the one-line rendering.
    fn write_compact(&self, out: &mut String) {
        match self {
            Measured::Scalar(s) => out.push_str(s),
            Measured::Array(items, _) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    item.write_compact(out);
                }
                out.push(']');
            },
            Measured::Object(entries, _) => {
                out.push('{');
                for (i, (k, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(k);
                    out.push_str(": ");
                    val.write_compact(out);
                }
                out.push('}');
            },
        }
    }
}

fn pretty_json(m: &Measured, indent: usize, out: &mut String) {
    if indent + m.len() <= PRETTY_WIDTH {
        m.write_compact(out);
        return;
    }
    let pad = " ".repeat(indent + 2);
    match m {
        Measured::Array(items, _) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                out.push_str(&pad);
                pretty_json(item, indent + 2, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&" ".repeat(indent));
            out.push(']');
        },
        Measured::Object(entries, _) => {
            out.push_str("{\n");
            for (i, (k, val)) in entries.iter().enumerate() {
                out.push_str(&pad);
                out.push_str(k);
                out.push_str(": ");
                pretty_json(val, indent + 2, out);
                if i + 1 < entries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&" ".repeat(indent));
            out.push('}');
        },
        // A scalar that exceeds the width (e.g. a long string) is emitted
        // as is.
        Measured::Scalar(s) => out.push_str(s),
    }
}

/// A Move type of the supported fragment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    /// `"bool"`
    Bool,
    /// `"u64"`
    U64,
    /// `"address"`
    Address,
    /// `"signer"` — signer values are addresses.
    Signer,
    /// `{"struct": resource}`
    Struct(ResourceId),
    /// `{"vector": type}` — a vector with the given element type.
    Vector(Box<Type>),
    /// `{"ref": type}` — immutable reference.  Transported for
    /// completeness, like the reference operations.
    Ref(Box<Type>),
    /// `{"mut_ref": type}` — mutable reference.
    MutRef(Box<Type>),
}

/// A struct field: name (documentation; consumers address fields by their
/// [`FieldOffset`]) and type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

/// A struct declaration.  The name is documentation; consumers address
/// structs by their [`ResourceId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    pub name: String,
    /// Fields in offset order.
    pub fields: Vec<Field>,
}

/// A function as a control-flow graph of basic blocks, together with loop
/// metadata and its specification contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    /// Number of parameters; locals `0..params` are the parameters.
    pub params: usize,
    /// Declared types of *all* locals: parameters first, then the remaining
    /// locals, including those synthesized by the producer.
    pub locals: Vec<Type>,
    /// The return types, in order of the `ret` operands.
    pub returns: Vec<Type>,
    /// Basic blocks in code-layout order; block `0` is the entry.
    pub blocks: Vec<Block>,
    /// The natural loops of the CFG.
    pub loops: Vec<Loop>,
    pub spec: Contract,
}

/// A basic block: straight-line instructions followed by one terminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub instrs: Vec<Instr>,
    pub term: Term,
}

/// A three-address instruction over locals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Instr {
    /// `{"load": [dst, value]}` — load a constant.
    Load(LocalIndex, Value),
    /// `{"assign": [dst, src]}` — copy a local.
    Assign(LocalIndex, LocalIndex),
    /// `{"call": [[dsts], oper, [srcs]]}` — apply an operation.
    Call(Vec<LocalIndex>, Oper, Vec<LocalIndex>),
    /// `"nop"` — no operation.
    Nop,
}

/// The operation of a [`Instr::Call`].
///
/// There are deliberately no `gt`/`ge`/`neq` operations: producers
/// normalize `gt`/`ge` to [`Oper::Lt`]/[`Oper::Le`] with swapped operands
/// and lower `neq` to [`Oper::Eq`] followed by [`Oper::Not`] through a
/// fresh local.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Oper {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Eq,
    And,
    Or,
    Not,
    /// `"pack"` — build a struct value from field operands.
    Pack,
    /// `"unpack"` — decompose a struct value into its fields.
    Unpack,
    /// `{"get_field": offset}` — select a field of a struct value.
    GetField(FieldOffset),
    /// `{"update_field": offset}` — functionally replace a field of a
    /// struct value.  Not emitted by the current producers; it is the
    /// residue of reference elimination (write through a field borrow).
    UpdateField(FieldOffset),
    /// `"vec_pack"` — build a vector value from the operands.  Like the
    /// other `vec_*` operations, not emitted by the current producers
    /// (vector natives are not translated yet).
    VecPack,
    /// `"vec_len"` — the length of a vector value, as `u64`.
    VecLen,
    /// `"vec_get"` — read the element at a dynamic index
    /// (vector, index); aborts out of range, like `vector::borrow`.
    VecGet,
    /// `"vec_set"` — functionally replace the element at a dynamic index
    /// (vector, index, value); aborts out of range.  Like `update_field`,
    /// a residue of reference elimination.
    VecSet,
    /// `"vec_push"` — append a value at the back (vector, value).
    VecPush,
    /// `"vec_pop"` — remove the last element, returning
    /// (vector, element); aborts on the empty vector.
    VecPop,
    /// `{"get_global": resource}` — read a resource from global memory.
    GetGlobal(ResourceId),
    /// `{"write_global": resource}` — overwrite a resource in global
    /// memory.  Not emitted by the current producers; it is the residue of
    /// reference elimination (`borrow_global` + write-back).
    WriteGlobal(ResourceId),
    /// `{"move_to": resource}` — publish a resource (signer, value).
    MoveTo(ResourceId),
    /// `{"move_from": resource}` — remove and return a resource.
    MoveFrom(ResourceId),
    /// `{"exists": resource}` — test resource existence.
    Exists(ResourceId),
    /// `{"function": fun}` — call a function of the same module.
    Function(FunId),
    /// The reference operations below are transported for completeness;
    /// consumers may reject them (the Lean model executes them; verifying
    /// borrow-based code goes through its reference elimination).
    BorrowLoc,
    /// `{"borrow_field": offset}`
    BorrowField(FieldOffset),
    /// `{"borrow_global": resource}`
    BorrowGlobal(ResourceId),
    /// `"borrow_vec_elem"` — borrow the element at a dynamic index
    /// (reference to a vector, index).
    BorrowVecElem,
    ReadRef,
    WriteRef,
    FreezeRef,
}

/// A block terminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Term {
    /// `{"jump": block}`
    Jump(BlockId),
    /// `{"branch": [cond, then_block, else_block]}` — branch on a boolean
    /// local.
    Branch(LocalIndex, BlockId, BlockId),
    /// `{"ret": [srcs]}` — return the given locals.
    Ret(Vec<LocalIndex>),
    /// `{"abort": code}` — abort with the code in the given local.
    Abort(LocalIndex),
}

/// A constant value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Value {
    /// `{"bool": true}`
    Bool(bool),
    /// `{"u64": "5"}` — decimal string, see the crate docs.
    U64(#[serde(with = "u64_as_string")] u64),
    /// `{"address": "0x42"}` — `0x`-prefixed hex literal.
    Address(String),
}

/// A specification expression.  Quantifiers bind a single variable,
/// referenced by de Bruijn index ([`SpecExp::Bvar`], innermost = 0).
/// `old(..)` has no node of its own: it is expressed by the memory label
/// carried by [`SpecExp::Global`]/[`SpecExp::Exists`] (`None` = current
/// state, `Some(0)` = pre-state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecExp {
    /// `{"value": value}`
    Value(Value),
    /// `{"local": n}` — a parameter of the function, bound to its value
    /// at function entry in every clause (arguments are call-by-value at
    /// the boundary, so later writes to parameter locals are not visible;
    /// only loop invariants may reference non-parameter locals, in the
    /// current state).
    Local(LocalIndex),
    /// `{"bvar": k}` — a quantifier-bound variable, de Bruijn index.
    Bvar(usize),
    /// `{"result": i}` — the i-th return value (`ensures` only).
    Result(usize),
    /// `{"binop": [op, lhs, rhs]}`
    Binop(SpecBinOp, Box<SpecExp>, Box<SpecExp>),
    /// `{"not": e}`
    Not(Box<SpecExp>),
    /// `{"select": [offset, e]}` — field selection.
    Select(FieldOffset, Box<SpecExp>),
    /// `{"len": e}` — the length of a vector, as `num`.
    Len(Box<SpecExp>),
    /// `{"global": [resource, label|null, addr]}` — the resource value at
    /// an address, in the labeled memory snapshot (or the current state).
    Global(ResourceId, Option<MemLabel>, Box<SpecExp>),
    /// `{"exists": [resource, label|null, addr]}` — resource existence.
    Exists(ResourceId, Option<MemLabel>, Box<SpecExp>),
    /// `{"ite": [cond, then, else]}`
    Ite(Box<SpecExp>, Box<SpecExp>, Box<SpecExp>),
    /// `{"quant": [kind, type, body]}` — quantifier with one bound
    /// variable of the given domain type; the semantics ranges over the
    /// well-formed values of that type.
    Quant(QuantKind, Type, Box<SpecExp>),
}

/// A binary operation of a [`SpecExp::Binop`].  Arithmetic is over
/// unbounded integers.  As with [`Oper`], there are no `gt`/`ge`/`neq`;
/// producers normalize them by constructing through [`SpecExp::gt`],
/// [`SpecExp::ge`], and [`SpecExp::neq`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Le,
    Eq,
    /// `"index"` — vector element selection (vector, index); undefined out
    /// of range or at a negative index.
    Index,
    And,
    Or,
    Implies,
    Iff,
}

impl SpecExp {
    /// `l op r`.
    pub fn binop(op: SpecBinOp, l: SpecExp, r: SpecExp) -> SpecExp {
        SpecExp::Binop(op, Box::new(l), Box::new(r))
    }

    /// `l > r`, normalized to `r < l` (the format has no `gt`).
    pub fn gt(l: SpecExp, r: SpecExp) -> SpecExp {
        Self::binop(SpecBinOp::Lt, r, l)
    }

    /// `l >= r`, normalized to `r <= l` (the format has no `ge`).
    pub fn ge(l: SpecExp, r: SpecExp) -> SpecExp {
        Self::binop(SpecBinOp::Le, r, l)
    }

    /// `l != r`, normalized to `!(l == r)` (the format has no `neq`).
    pub fn neq(l: SpecExp, r: SpecExp) -> SpecExp {
        SpecExp::Not(Box::new(Self::binop(SpecBinOp::Eq, l, r)))
    }

    /// A quantifier over one bound variable of domain type `dom`.
    pub fn quant(kind: QuantKind, dom: Type, body: SpecExp) -> SpecExp {
        SpecExp::Quant(kind, dom, Box::new(body))
    }
}

/// The kind of a [`SpecExp::Quant`]: `"all"` (universal) or `"ex"`
/// (existential).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantKind {
    All,
    Ex,
}

/// The specification contract of a function.  `requires` and `ensures`
/// are conjoined; `aborts_if` conditions are disjoined.  An **empty**
/// `aborts_if` list makes no claim about aborts (the prover's *partial*
/// default); an explicit `aborts_if false` forbids aborting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    pub requires: Vec<SpecExp>,
    pub aborts_if: Vec<SpecExp>,
    pub ensures: Vec<SpecExp>,
    pub modifies: Vec<ModifiesTarget>,
}

/// A `modifies global<R>(addr)` target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModifiesTarget {
    pub resource: ResourceId,
    pub addr: SpecExp,
}

/// Metadata of one natural loop of a function's CFG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Loop {
    /// The loop header block (target of the back edges).
    pub header: BlockId,
    /// All blocks of the loop, ascending; includes the header.
    pub members: Vec<BlockId>,
    /// Locals written inside the loop.
    pub val_targets: Vec<LocalIndex>,
    /// Resources whose global memory is written inside the loop.
    pub mem_targets: Vec<ResourceId>,
    /// The loop invariants attached to the header.
    pub invariants: Vec<SpecExp>,
}

/// `u64` as a decimal string on the wire; see the crate docs.
mod u64_as_string {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        s.parse()
            .map_err(|e| D::Error::custom(format!("invalid u64 `{}`: {}", s, e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A module exercising every variant of the format.
    fn full_module() -> Module {
        Module::new(
            vec![Struct {
                name: "Account".to_string(),
                fields: vec![Field {
                    name: "balance".to_string(),
                    ty: Type::U64,
                }],
            }],
            vec![Function {
                name: "f".to_string(),
                params: 1,
                locals: vec![
                    Type::Ref(Box::new(Type::Signer)),
                    Type::U64,
                    Type::Bool,
                    Type::Address,
                    Type::U64,
                    Type::U64,
                    Type::U64,
                    Type::U64,
                    Type::Bool,
                    Type::Struct(0),
                    Type::U64,
                    Type::U64,
                    Type::Struct(0),
                    Type::Struct(0),
                    Type::Bool,
                    Type::U64,
                    Type::MutRef(Box::new(Type::U64)),
                    Type::MutRef(Box::new(Type::U64)),
                    Type::MutRef(Box::new(Type::Struct(0))),
                    Type::U64,
                    Type::Ref(Box::new(Type::U64)),
                ],
                returns: vec![Type::U64],
                blocks: vec![
                    Block {
                        instrs: vec![
                            Instr::Load(1, Value::U64(u64::MAX)),
                            Instr::Load(2, Value::Bool(true)),
                            Instr::Load(3, Value::Address("0x42".to_string())),
                            Instr::Assign(4, 1),
                            Instr::Nop,
                            Instr::Call(vec![5], Oper::Add, vec![1, 4]),
                            Instr::Call(vec![6], Oper::Sub, vec![5, 1]),
                            Instr::Call(vec![7], Oper::Mul, vec![6, 6]),
                            Instr::Call(vec![7], Oper::Div, vec![7, 6]),
                            Instr::Call(vec![7], Oper::Mod, vec![7, 6]),
                            Instr::Call(vec![8], Oper::Lt, vec![6, 7]),
                            Instr::Call(vec![8], Oper::Le, vec![6, 7]),
                            Instr::Call(vec![8], Oper::Eq, vec![6, 7]),
                            Instr::Call(vec![8], Oper::And, vec![8, 2]),
                            Instr::Call(vec![8], Oper::Or, vec![8, 2]),
                            Instr::Call(vec![8], Oper::Not, vec![8]),
                            Instr::Call(vec![9], Oper::Pack, vec![1]),
                            Instr::Call(vec![10], Oper::Unpack, vec![9]),
                            Instr::Call(vec![11], Oper::GetField(0), vec![9]),
                            Instr::Call(vec![12], Oper::GetGlobal(0), vec![3]),
                            Instr::Call(vec![], Oper::WriteGlobal(0), vec![3, 9]),
                            Instr::Call(vec![], Oper::MoveTo(0), vec![0, 9]),
                            Instr::Call(vec![13], Oper::MoveFrom(0), vec![3]),
                            Instr::Call(vec![14], Oper::Exists(0), vec![3]),
                            Instr::Call(vec![15], Oper::Function(0), vec![1]),
                            Instr::Call(vec![16], Oper::BorrowLoc, vec![1]),
                            Instr::Call(vec![17], Oper::BorrowField(0), vec![16]),
                            Instr::Call(vec![18], Oper::BorrowGlobal(0), vec![3]),
                            Instr::Call(vec![19], Oper::ReadRef, vec![17]),
                            Instr::Call(vec![], Oper::WriteRef, vec![17, 1]),
                            Instr::Call(vec![20], Oper::FreezeRef, vec![18]),
                        ],
                        term: Term::Branch(8, 1, 2),
                    },
                    Block {
                        instrs: vec![],
                        term: Term::Jump(0),
                    },
                    Block {
                        instrs: vec![],
                        term: Term::Abort(1),
                    },
                    Block {
                        instrs: vec![],
                        term: Term::Ret(vec![7]),
                    },
                ],
                loops: vec![Loop {
                    header: 0,
                    members: vec![0, 1],
                    val_targets: vec![1, 4],
                    mem_targets: vec![0],
                    invariants: vec![SpecExp::binop(
                        SpecBinOp::Le,
                        SpecExp::Local(1),
                        SpecExp::Value(Value::U64(10)),
                    )],
                }],
                spec: Contract {
                    requires: vec![SpecExp::quant(
                        QuantKind::All,
                        Type::Address,
                        SpecExp::Exists(0, None, Box::new(SpecExp::Bvar(0))),
                    )],
                    aborts_if: vec![SpecExp::Not(Box::new(SpecExp::Exists(
                        0,
                        None,
                        Box::new(SpecExp::Local(0)),
                    )))],
                    ensures: vec![
                        SpecExp::Binop(
                            SpecBinOp::Eq,
                            Box::new(SpecExp::Result(0)),
                            Box::new(SpecExp::Select(
                                0,
                                Box::new(SpecExp::Global(0, Some(0), Box::new(SpecExp::Local(0)))),
                            )),
                        ),
                        SpecExp::Ite(
                            Box::new(SpecExp::Value(Value::Bool(false))),
                            Box::new(SpecExp::Select(
                                0,
                                Box::new(SpecExp::Global(0, None, Box::new(SpecExp::Local(0)))),
                            )),
                            Box::new(SpecExp::quant(
                                QuantKind::Ex,
                                Type::U64,
                                SpecExp::Binop(
                                    SpecBinOp::Iff,
                                    Box::new(SpecExp::Bvar(0)),
                                    Box::new(SpecExp::Bvar(0)),
                                ),
                            )),
                        ),
                    ],
                    modifies: vec![ModifiesTarget {
                        resource: 0,
                        addr: SpecExp::Local(0),
                    }],
                },
            }],
        )
    }

    #[test]
    fn round_trip() {
        let module = full_module();
        let json = serde_json::to_value(&module).unwrap();
        let back: Module = serde_json::from_value(json).unwrap();
        assert_eq!(module, back);
        assert!(back.check_version().is_ok());
    }

    #[test]
    fn wire_fragments() {
        // Pin the exact wire shapes; the variant arity determines
        // array-wrapping, so an innocent-looking refactor of a variant
        // changes the format.
        let json = serde_json::to_value(full_module()).unwrap();
        assert_eq!(json["version"], json!(5));
        assert_eq!(
            json["structs"][0]["fields"][0],
            json!({"name": "balance", "ty": "u64"})
        );
        let locals = &json["funs"][0]["locals"];
        assert_eq!(locals[0], json!({"ref": "signer"}));
        assert_eq!(locals[9], json!({"struct": 0}));
        assert_eq!(locals[16], json!({"mut_ref": "u64"}));
        assert_eq!(json["funs"][0]["returns"], json!(["u64"]));
        let instrs = &json["funs"][0]["blocks"][0]["instrs"];
        assert_eq!(
            instrs[0],
            json!({"load": [1, {"u64": "18446744073709551615"}]})
        );
        assert_eq!(instrs[2], json!({"load": [3, {"address": "0x42"}]}));
        assert_eq!(instrs[3], json!({"assign": [4, 1]}));
        assert_eq!(instrs[4], json!("nop"));
        assert_eq!(instrs[5], json!({"call": [[5], "add", [1, 4]]}));
        assert_eq!(instrs[18], json!({"call": [[11], {"get_field": 0}, [9]]}));
        assert_eq!(
            serde_json::to_value(Oper::UpdateField(1)).unwrap(),
            json!({"update_field": 1})
        );
        assert_eq!(
            serde_json::to_value(Type::Vector(Box::new(Type::U64))).unwrap(),
            json!({"vector": "u64"})
        );
        assert_eq!(
            serde_json::to_value(Oper::VecPack).unwrap(),
            json!("vec_pack")
        );
        assert_eq!(
            serde_json::to_value(Oper::VecLen).unwrap(),
            json!("vec_len")
        );
        assert_eq!(
            serde_json::to_value(Oper::VecGet).unwrap(),
            json!("vec_get")
        );
        assert_eq!(
            serde_json::to_value(Oper::VecSet).unwrap(),
            json!("vec_set")
        );
        assert_eq!(
            serde_json::to_value(Oper::VecPush).unwrap(),
            json!("vec_push")
        );
        assert_eq!(
            serde_json::to_value(Oper::VecPop).unwrap(),
            json!("vec_pop")
        );
        assert_eq!(
            serde_json::to_value(Oper::BorrowVecElem).unwrap(),
            json!("borrow_vec_elem")
        );
        assert_eq!(
            serde_json::to_value(SpecExp::Len(Box::new(SpecExp::Local(0)))).unwrap(),
            json!({"len": {"local": 0}})
        );
        assert_eq!(
            serde_json::to_value(SpecExp::binop(
                SpecBinOp::Index,
                SpecExp::Local(0),
                SpecExp::Bvar(0)
            ))
            .unwrap(),
            json!({"binop": ["index", {"local": 0}, {"bvar": 0}]})
        );
        assert_eq!(
            instrs[20],
            json!({"call": [[], {"write_global": 0}, [3, 9]]})
        );
        assert_eq!(
            json["funs"][0]["blocks"][0]["term"],
            json!({"branch": [8, 1, 2]})
        );
        assert_eq!(json["funs"][0]["blocks"][1]["term"], json!({"jump": 0}));
        assert_eq!(json["funs"][0]["blocks"][2]["term"], json!({"abort": 1}));
        assert_eq!(json["funs"][0]["blocks"][3]["term"], json!({"ret": [7]}));
        let spec = &json["funs"][0]["spec"];
        assert_eq!(
            spec["requires"][0]["quant"],
            json!(["all", "address", {"exists": [0, null, {"bvar": 0}]}])
        );
        assert_eq!(
            spec["ensures"][0],
            json!({"binop": ["eq", {"result": 0},
                             {"select": [0, {"global": [0, 0, {"local": 0}]}]}]})
        );
        assert_eq!(
            spec["modifies"][0],
            json!({"resource": 0, "addr": {"local": 0}})
        );
        let lp = &json["funs"][0]["loops"][0];
        assert_eq!(
            lp["invariants"][0],
            json!({"binop": ["le", {"local": 1}, {"value": {"u64": "10"}}]})
        );
    }

    #[test]
    fn pretty_json_fill_layout() {
        let pretty = full_module().to_pretty_json();
        // Small subtrees are inlined; the document as a whole is broken.
        assert!(pretty.contains(r#"{"call": [[5], "add", [1, 4]]}"#));
        assert!(pretty.contains("\"blocks\": [\n"));
        // The rendering is still valid JSON denoting the same module.
        let back: Module = serde_json::from_str(&pretty).unwrap();
        assert_eq!(full_module(), back);
    }

    #[test]
    fn version_check() {
        let mut module = full_module();
        module.version = 1;
        assert!(module.check_version().is_err());
        let err = serde_json::from_value::<Module>(json!({
            "version": 4, "structs": [],
            "funs": [{"name": "f", "params": 0,
                      "locals": ["u64"], "returns": [],
                      "blocks": [{"instrs": [{"load": [0, {"u64": "not-a-number"}]}],
                                  "term": {"ret": []}}],
                      "loops": [],
                      "spec": {"requires": [], "aborts_if": [], "ensures": [],
                               "modifies": []}}]
        }))
        .unwrap_err();
        assert!(err.to_string().contains("invalid u64"));
    }
}
