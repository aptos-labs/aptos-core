// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Well-sortedness of specification expressions.
//!
//! Consumers *assume* `requires` at function entry, so an ill-scoped or
//! ill-sorted clause would not fail verification — it would make it
//! vacuous (a stuck assumption can never hold).  Producers therefore check
//! every clause before export.  This module defines the spec-level sorts —
//! integers are the unbounded `num`, signers are addresses, references are
//! transparent — and the position discipline: which locals, and whether
//! `result`, a clause may reference.

use crate::{Function, SpecBinOp, SpecExp, Struct, Type, Value};
use std::{collections::BTreeSet, fmt};

/// Spec-level sorts: `u64` values embed into the unbounded `num`, `signer`
/// values are addresses, and references are transparent (spec expressions
/// operate on the dereferenced value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecSort {
    Bool,
    Num,
    Address,
    TypeParameter(usize),
    Struct(usize),
    StructInst(usize, Vec<SpecSort>),
    Enum(usize),
    EnumInst(usize, Vec<SpecSort>),
    Vector(Box<SpecSort>),
}

impl fmt::Display for SpecSort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecSort::Bool => write!(f, "bool"),
            SpecSort::Num => write!(f, "num"),
            SpecSort::Address => write!(f, "address"),
            SpecSort::TypeParameter(i) => write!(f, "type parameter {}", i),
            SpecSort::Struct(r) => write!(f, "struct {}", r),
            SpecSort::StructInst(r, args) => write!(f, "struct {}<{:?}>", r, args),
            SpecSort::Enum(r) => write!(f, "enum {}", r),
            SpecSort::EnumInst(r, args) => write!(f, "enum {}<{:?}>", r, args),
            SpecSort::Vector(t) => write!(f, "vector<{}>", t),
        }
    }
}

impl SpecSort {
    /// The spec-level sort of a declared type.
    pub fn of(ty: &Type) -> SpecSort {
        match ty {
            Type::Bool => SpecSort::Bool,
            Type::U8
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
            | Type::I256 => SpecSort::Num,
            Type::Address | Type::Signer => SpecSort::Address,
            Type::TypeParameter(i) => SpecSort::TypeParameter(*i),
            Type::Struct(r) => SpecSort::Struct(*r),
            Type::StructInst(r, args) => {
                SpecSort::StructInst(*r, args.iter().map(SpecSort::of).collect())
            },
            Type::Enum(r) => SpecSort::Enum(*r),
            Type::EnumInst(r, args) => {
                SpecSort::EnumInst(*r, args.iter().map(SpecSort::of).collect())
            },
            Type::Vector(t) => SpecSort::Vector(Box::new(SpecSort::of(t))),
            Type::Ref(t) | Type::MutRef(t) => SpecSort::of(t),
        }
    }

    fn instantiate(self, args: &[SpecSort]) -> SpecSort {
        match self {
            SpecSort::TypeParameter(i) => {
                args.get(i).cloned().unwrap_or(SpecSort::TypeParameter(i))
            },
            SpecSort::StructInst(r, inner) => SpecSort::StructInst(
                r,
                inner.into_iter().map(|ty| ty.instantiate(args)).collect(),
            ),
            SpecSort::EnumInst(r, inner) => SpecSort::EnumInst(
                r,
                inner.into_iter().map(|ty| ty.instantiate(args)).collect(),
            ),
            SpecSort::Vector(inner) => SpecSort::Vector(Box::new(inner.instantiate(args))),
            sort => sort,
        }
    }
}

/// The position a clause occurs in, determining its scope.
#[derive(Debug, Clone, Copy)]
pub enum ClausePos {
    /// `requires`/`aborts_if`/`modifies` addresses: evaluated at the
    /// boundary in the pre-state — parameters only, no `result`.
    Pre,
    /// `ensures`: parameters and `result` (function locals beyond the
    /// parameters are not visible at the boundary).
    Post,
    /// Loop invariants: all function locals, no `result`.
    Invariant,
    /// The most permissive scope (all locals and `result` visible), for
    /// sort queries during translation; the position discipline is
    /// enforced separately by `check_function`.
    Any,
}

/// The typing context of one function's specification.
pub struct SpecCheckCtx<'a> {
    pub structs: &'a [Struct],
    /// Declared types of all locals (parameters first).
    pub locals: &'a [Type],
    pub num_params: usize,
    pub returns: &'a [Type],
}

impl SpecCheckCtx<'_> {
    fn value_sort(value: &Value) -> Result<SpecSort, String> {
        match value {
            Value::Bool(_) => Ok(SpecSort::Bool),
            Value::Num(_) => Ok(SpecSort::Num),
            Value::Address(_) => Ok(SpecSort::Address),
            Value::Vector(values) => {
                let Some(first) = values.first() else {
                    return Err("an empty vector constant has no inferable spec sort".to_string());
                };
                let elem = Self::value_sort(first)?;
                for value in &values[1..] {
                    let found = Self::value_sort(value)?;
                    if found != elem {
                        return Err(format!(
                            "vector constant has elements of different sorts {} and {}",
                            elem, found
                        ));
                    }
                }
                Ok(SpecSort::Vector(Box::new(elem)))
            },
        }
    }

    /// Mutable references are represented by `Value.mut` in Lean, and
    /// specifications have no implicit operation which extracts its payload.
    /// Immutable references are represented transparently and remain valid
    /// specification operands (notably `&signer` parameters).
    fn local_sort(&self, i: usize) -> Result<SpecSort, String> {
        let local = self
            .locals
            .get(i)
            .ok_or_else(|| format!("undeclared local {}", i))?;
        match local {
            Type::MutRef(_) => Err(format!(
                "mutable-reference local {} cannot be used in a specification",
                i
            )),
            _ => Ok(SpecSort::of(local)),
        }
    }

    fn field_sort(&self, r: usize, offset: usize) -> Result<SpecSort, String> {
        let s = self
            .structs
            .get(r)
            .ok_or_else(|| format!("undeclared resource {}", r))?;
        let field = s
            .fields
            .get(offset)
            .ok_or_else(|| format!("struct `{}` has no field at offset {}", s.name, offset))?;
        Ok(SpecSort::of(&field.ty))
    }

    /// The sort of `e`, or an error for ill-scoped or ill-sorted
    /// expressions.  `binders` is the stack of quantifier binder sorts,
    /// innermost last.
    pub fn sort(
        &self,
        pos: ClausePos,
        binders: &mut Vec<SpecSort>,
        e: &SpecExp,
    ) -> Result<SpecSort, String> {
        match e {
            SpecExp::Value(value) => Self::value_sort(value),
            SpecExp::Local(i) => {
                let bound = match pos {
                    ClausePos::Invariant | ClausePos::Any => self.locals.len(),
                    ClausePos::Pre | ClausePos::Post => self.num_params,
                };
                if *i >= bound {
                    return Err(if *i < self.locals.len() {
                        format!(
                            "local {} is not a parameter (only parameters are \
                             visible at the function boundary)",
                            i
                        )
                    } else {
                        format!("undeclared local {}", i)
                    });
                }
                self.local_sort(*i)
            },
            SpecExp::Bvar(k) => {
                if *k >= binders.len() {
                    return Err(format!("unbound quantifier variable {}", k));
                }
                Ok(binders[binders.len() - 1 - k].clone())
            },
            SpecExp::Result(i) => match pos {
                ClausePos::Post | ClausePos::Any => {
                    let result = self.returns.get(*i).ok_or_else(|| {
                        format!(
                            "result {} out of range ({} return value(s))",
                            i,
                            self.returns.len()
                        )
                    })?;
                    if matches!(result, Type::MutRef(_)) {
                        return Err(format!(
                            "mutable-reference result {} cannot be used in a specification",
                            i
                        ));
                    }
                    Ok(SpecSort::of(result))
                },
                ClausePos::Pre | ClausePos::Invariant => {
                    Err("`result` is only available in `ensures`".to_string())
                },
            },
            SpecExp::Binop(op, l, r) => {
                let sl = self.sort(pos, binders, l)?;
                let sr = self.sort(pos, binders, r)?;
                let expect = |s: SpecSort, want: SpecSort, side: &str| {
                    if s == want {
                        Ok(())
                    } else {
                        Err(format!(
                            "{} operand of `{:?}` must be {}, found {}",
                            side, op, want, s
                        ))
                    }
                };
                match op {
                    SpecBinOp::Add | SpecBinOp::Sub | SpecBinOp::Mul => {
                        expect(sl, SpecSort::Num, "left")?;
                        expect(sr, SpecSort::Num, "right")?;
                        Ok(SpecSort::Num)
                    },
                    SpecBinOp::Div | SpecBinOp::Mod => {
                        expect(sl, SpecSort::Num, "left")?;
                        expect(sr, SpecSort::Num, "right")?;
                        if !matches!(
                            r.as_ref(),
                            SpecExp::Value(Value::Num(n)) if n != "0"
                        ) {
                            return Err(format!(
                                "right operand of `{:?}` must be a statically nonzero literal",
                                op
                            ));
                        }
                        Ok(SpecSort::Num)
                    },
                    SpecBinOp::Lt | SpecBinOp::Le => {
                        expect(sl, SpecSort::Num, "left")?;
                        expect(sr, SpecSort::Num, "right")?;
                        Ok(SpecSort::Bool)
                    },
                    SpecBinOp::Eq => {
                        if sl != sr {
                            return Err(format!(
                                "`==` compares values of different sorts {} and {}",
                                sl, sr
                            ));
                        }
                        Ok(SpecSort::Bool)
                    },
                    SpecBinOp::Index => match sl {
                        SpecSort::Vector(elem) => {
                            expect(sr, SpecSort::Num, "right")?;
                            Ok(*elem)
                        },
                        s => Err(format!(
                            "left operand of `index` must be a vector, found {}",
                            s
                        )),
                    },
                    SpecBinOp::And | SpecBinOp::Or | SpecBinOp::Implies | SpecBinOp::Iff => {
                        expect(sl, SpecSort::Bool, "left")?;
                        expect(sr, SpecSort::Bool, "right")?;
                        Ok(SpecSort::Bool)
                    },
                }
            },
            SpecExp::Not(e) => match self.sort(pos, binders, e)? {
                SpecSort::Bool => Ok(SpecSort::Bool),
                s => Err(format!("`!` applies to bool, found {}", s)),
            },
            SpecExp::Select(offset, e) => match self.sort(pos, binders, e)? {
                SpecSort::Struct(r) => self.field_sort(r, *offset),
                SpecSort::StructInst(r, args) => {
                    Ok(self.field_sort(r, *offset)?.instantiate(&args))
                },
                s => Err(format!("field selection applies to a struct, found {}", s)),
            },
            SpecExp::Len(e) => match self.sort(pos, binders, e)? {
                SpecSort::Vector(_) => Ok(SpecSort::Num),
                s => Err(format!("`len` applies to a vector, found {}", s)),
            },
            SpecExp::Global(r, lbl, addr) => {
                if *r >= self.structs.len() {
                    return Err(format!("undeclared resource {}", r));
                }
                if lbl.is_some_and(|label| label != 0) {
                    return Err(format!("undeclared memory label {:?}", lbl));
                }
                match self.sort(pos, binders, addr)? {
                    SpecSort::Address => Ok(SpecSort::Struct(*r)),
                    s => Err(format!("`global` address must be an address, found {}", s)),
                }
            },
            SpecExp::Exists(r, lbl, addr) => {
                if *r >= self.structs.len() {
                    return Err(format!("undeclared resource {}", r));
                }
                if lbl.is_some_and(|label| label != 0) {
                    return Err(format!("undeclared memory label {:?}", lbl));
                }
                match self.sort(pos, binders, addr)? {
                    SpecSort::Address => Ok(SpecSort::Bool),
                    s => Err(format!("`exists` address must be an address, found {}", s)),
                }
            },
            SpecExp::Ite(c, t, e) => {
                match self.sort(pos, binders, c)? {
                    SpecSort::Bool => {},
                    s => return Err(format!("`if` condition must be bool, found {}", s)),
                }
                let st = self.sort(pos, binders, t)?;
                let se = self.sort(pos, binders, e)?;
                if st != se {
                    return Err(format!(
                        "`if` branches have different sorts {} and {}",
                        st, se
                    ));
                }
                Ok(st)
            },
            SpecExp::Quant(_kind, dom, body) => {
                binders.push(SpecSort::of(dom));
                let s = self.sort(pos, binders, body);
                binders.pop();
                match s? {
                    SpecSort::Bool => Ok(SpecSort::Bool),
                    s => Err(format!("quantifier body must be bool, found {}", s)),
                }
            },
        }
    }

    fn check_bool(&self, pos: ClausePos, what: &str, e: &SpecExp) -> Result<(), String> {
        match self.sort(pos, &mut vec![], e) {
            Ok(SpecSort::Bool) => Ok(()),
            Ok(s) => Err(format!("{} must be a boolean condition, found {}", what, s)),
            Err(err) => Err(format!("in {}: {}", what, err)),
        }
    }

    /// Checks the contract and loop invariants of `fun` under the position
    /// discipline (see module docs).
    pub fn check_function(&self, fun: &Function) -> Result<(), String> {
        if fun.params > fun.locals.len() {
            return Err(format!(
                "function declares {} parameters but only {} locals",
                fun.params,
                fun.locals.len()
            ));
        }
        for e in &fun.spec.requires {
            self.check_bool(ClausePos::Pre, "`requires` clause", e)?;
        }
        for e in &fun.spec.aborts_if {
            self.check_bool(ClausePos::Pre, "`aborts_if` clause", e)?;
        }
        for e in &fun.spec.ensures {
            self.check_bool(ClausePos::Post, "`ensures` clause", e)?;
        }
        for target in &fun.spec.modifies {
            if target.resource >= self.structs.len() {
                return Err(format!(
                    "in `modifies` clause: undeclared resource {}",
                    target.resource
                ));
            }
            match self.sort(ClausePos::Pre, &mut vec![], &target.addr) {
                Ok(SpecSort::Address) => {},
                Ok(s) => {
                    return Err(format!(
                        "`modifies` address must be an address, found {}",
                        s
                    ))
                },
                Err(err) => return Err(format!("in `modifies` clause: {}", err)),
            }
        }
        let mut loop_headers = BTreeSet::new();
        for (loop_index, lp) in fun.loops.iter().enumerate() {
            if lp.header >= fun.blocks.len() {
                return Err(format!(
                    "loop {} header block {} out of range ({} block(s))",
                    loop_index,
                    lp.header,
                    fun.blocks.len()
                ));
            }
            if !loop_headers.insert(lp.header) {
                return Err(format!("duplicate loop header block {}", lp.header));
            }
            let members: BTreeSet<_> = lp.members.iter().copied().collect();
            if members.len() != lp.members.len() {
                return Err(format!(
                    "loop {} contains duplicate member blocks",
                    loop_index
                ));
            }
            if !lp.members.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(format!(
                    "loop {} member blocks are not strictly ascending",
                    loop_index
                ));
            }
            if !members.contains(&lp.header) {
                return Err(format!(
                    "loop {} members do not contain its header {}",
                    loop_index, lp.header
                ));
            }
            if let Some(member) = lp
                .members
                .iter()
                .find(|member| **member >= fun.blocks.len())
            {
                return Err(format!(
                    "loop {} member block {} out of range ({} block(s))",
                    loop_index,
                    member,
                    fun.blocks.len()
                ));
            }
            let val_targets: BTreeSet<_> = lp.val_targets.iter().copied().collect();
            if val_targets.len() != lp.val_targets.len() {
                return Err(format!(
                    "loop {} contains duplicate local targets",
                    loop_index
                ));
            }
            if !lp.val_targets.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(format!(
                    "loop {} local targets are not strictly ascending",
                    loop_index
                ));
            }
            if let Some(local) = lp
                .val_targets
                .iter()
                .find(|local| **local >= fun.locals.len())
            {
                return Err(format!(
                    "loop {} local target {} out of range ({} local(s))",
                    loop_index,
                    local,
                    fun.locals.len()
                ));
            }
            let mem_targets: BTreeSet<_> = lp.mem_targets.iter().copied().collect();
            if mem_targets.len() != lp.mem_targets.len() {
                return Err(format!(
                    "loop {} contains duplicate memory targets",
                    loop_index
                ));
            }
            if !lp.mem_targets.windows(2).all(|pair| pair[0] < pair[1]) {
                return Err(format!(
                    "loop {} memory targets are not strictly ascending",
                    loop_index
                ));
            }
            if let Some(resource) = lp
                .mem_targets
                .iter()
                .find(|resource| **resource >= self.structs.len())
            {
                return Err(format!(
                    "loop {} memory target {} is an undeclared resource",
                    loop_index, resource
                ));
            }
            for e in &lp.invariants {
                self.check_bool(ClausePos::Invariant, "loop invariant", e)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Contract, Field};

    fn empty_fun(locals: &[Type], returns: &[Type]) -> Function {
        Function {
            name: "f".to_string(),
            type_parameters: vec![],
            params: 1,
            locals: locals.to_vec(),
            returns: returns.to_vec(),
            blocks: vec![],
            loops: vec![],
            spec: Contract {
                requires: vec![],
                aborts_if: vec![],
                ensures: vec![],
                modifies: vec![],
            },
        }
    }

    fn check(
        structs: &[Struct],
        locals: &[Type],
        returns: &[Type],
        fun: &Function,
    ) -> Result<(), String> {
        SpecCheckCtx {
            structs,
            locals,
            num_params: fun.params,
            returns,
        }
        .check_function(fun)
    }

    #[test]
    fn rejects_non_parameter_in_requires() {
        let locals = vec![Type::U64, Type::U64];
        let mut fun = empty_fun(&locals, &[]);
        // `requires local1 < 1` — local 1 is not a parameter.
        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Lt,
            SpecExp::Local(1),
            SpecExp::Value(Value::Num("1".to_string())),
        )];
        let err = check(&[], &locals, &[], &fun).unwrap_err();
        assert!(err.contains("not a parameter"), "{}", err);
    }

    #[test]
    fn rejects_result_in_requires_and_non_bool_clause() {
        let locals = vec![Type::U64];
        let returns = vec![Type::U64];
        let mut fun = empty_fun(&locals, &returns);
        fun.spec.requires = vec![SpecExp::Result(0)];
        let err = check(&[], &locals, &returns, &fun).unwrap_err();
        assert!(err.contains("only available in `ensures`"), "{}", err);

        // `requires x + 1` — not a boolean condition.
        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Add,
            SpecExp::Local(0),
            SpecExp::Value(Value::Num("1".to_string())),
        )];
        let err = check(&[], &locals, &returns, &fun).unwrap_err();
        assert!(err.contains("must be a boolean condition"), "{}", err);
    }

    #[test]
    fn accepts_well_sorted_contract_and_invariant() {
        let structs = vec![Struct {
            name: "R".to_string(),
            type_parameters: vec![],
            fields: vec![Field {
                name: "v".to_string(),
                ty: Type::U64,
            }],
            variants: None,
        }];
        let locals = vec![Type::Address, Type::U64];
        let returns = vec![Type::U64];
        let mut fun = empty_fun(&locals, &returns);
        fun.spec.aborts_if = vec![SpecExp::Not(Box::new(SpecExp::Exists(
            0,
            None,
            Box::new(SpecExp::Local(0)),
        )))];
        fun.spec.ensures = vec![
            SpecExp::binop(
                SpecBinOp::Eq,
                SpecExp::Result(0),
                SpecExp::Select(
                    0,
                    Box::new(SpecExp::Global(0, Some(0), Box::new(SpecExp::Local(0)))),
                ),
            ),
            SpecExp::quant(
                crate::QuantKind::All,
                Type::U64,
                SpecExp::binop(
                    SpecBinOp::Le,
                    SpecExp::Value(Value::Num("0".to_string())),
                    SpecExp::Bvar(0),
                ),
            ),
        ];
        check(&structs, &locals, &returns, &fun).unwrap();

        // An invariant may reference non-parameter locals.
        fun.spec.ensures = vec![];
        fun.blocks = vec![crate::Block {
            instrs: vec![],
            term: crate::Term::Ret(vec![]),
        }];
        fun.loops = vec![crate::Loop {
            header: 0,
            members: vec![0],
            val_targets: vec![],
            mem_targets: vec![],
            invariants: vec![SpecExp::binop(
                SpecBinOp::Lt,
                SpecExp::Local(1),
                SpecExp::Value(Value::Num("10".to_string())),
            )],
        }];
        check(&structs, &locals, &returns, &fun).unwrap();
    }

    #[test]
    fn rejects_unknown_memory_labels() {
        let structs = vec![Struct {
            name: "R".to_string(),
            type_parameters: vec![],
            fields: vec![],
            variants: None,
        }];
        let locals = vec![Type::Address];
        let mut fun = empty_fun(&locals, &[]);
        fun.spec.requires = vec![SpecExp::Exists(0, Some(1), Box::new(SpecExp::Local(0)))];
        let err = check(&structs, &locals, &[], &fun).unwrap_err();
        assert!(err.contains("undeclared memory label"), "{}", err);
    }

    #[test]
    fn rejects_mutable_reference_locals_in_specs() {
        let locals = vec![Type::MutRef(Box::new(Type::U64))];
        let mut fun = empty_fun(&locals, &[]);
        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Eq,
            SpecExp::Local(0),
            SpecExp::Value(Value::Num("0".to_string())),
        )];
        let err = check(&[], &locals, &[], &fun).unwrap_err();
        assert!(err.contains("mutable-reference local"), "{}", err);

        let locals = vec![Type::Ref(Box::new(Type::Signer))];
        let mut fun = empty_fun(&locals, &[]);
        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Eq,
            SpecExp::Local(0),
            SpecExp::Value(Value::Address("0x0".to_string())),
        )];
        check(&[], &locals, &[], &fun).unwrap();

        let returns = vec![Type::MutRef(Box::new(Type::U64))];
        let mut fun = empty_fun(&[], &returns);
        fun.params = 0;
        fun.spec.ensures = vec![SpecExp::binop(
            SpecBinOp::Eq,
            SpecExp::Result(0),
            SpecExp::Value(Value::Num("0".to_string())),
        )];
        let err = check(&[], &[], &returns, &fun).unwrap_err();
        assert!(err.contains("mutable-reference result"), "{}", err);
    }

    #[test]
    fn rejects_possibly_zero_spec_divisors() {
        let locals = vec![Type::U64, Type::U64];
        let mut fun = empty_fun(&locals, &[]);
        fun.params = 2;
        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Eq,
            SpecExp::binop(SpecBinOp::Div, SpecExp::Local(0), SpecExp::Local(1)),
            SpecExp::Value(Value::Num("0".to_string())),
        )];
        let err = check(&[], &locals, &[], &fun).unwrap_err();
        assert!(err.contains("statically nonzero literal"), "{}", err);

        fun.spec.requires = vec![SpecExp::binop(
            SpecBinOp::Eq,
            SpecExp::binop(
                SpecBinOp::Div,
                SpecExp::Local(0),
                SpecExp::Value(Value::Num("1".to_string())),
            ),
            SpecExp::Value(Value::Num("0".to_string())),
        )];
        check(&[], &locals, &[], &fun).unwrap();
    }

    #[test]
    fn rejects_out_of_range_loop_metadata() {
        let locals = vec![Type::U64];
        let mut fun = empty_fun(&locals, &[]);
        fun.blocks = vec![crate::Block {
            instrs: vec![],
            term: crate::Term::Ret(vec![]),
        }];
        fun.loops = vec![crate::Loop {
            header: 0,
            members: vec![0],
            val_targets: vec![1],
            mem_targets: vec![],
            invariants: vec![],
        }];
        let err = check(&[], &locals, &[], &fun).unwrap_err();
        assert!(err.contains("local target 1 out of range"), "{}", err);

        fun.loops[0].val_targets.clear();
        fun.loops[0].members = vec![1];
        let err = check(&[], &locals, &[], &fun).unwrap_err();
        assert!(err.contains("do not contain its header"), "{}", err);
    }
}
