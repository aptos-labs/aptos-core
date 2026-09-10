// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Translation of move-model specifications (the genuine Move specification
//! language, as produced by compiler v2 from `spec` blocks) into the
//! exchange spec-expression form (`move_model_exchange::SpecExp`).
//!
//! This is the source-mode counterpart of `spec.rs` (which translates the
//! masm spec clauses): conditions come from `FunctionEnv::get_spec()` and
//! loop invariants from the `Prop` instructions the stackless generator
//! emits at loop headers.  `old(..)` marks inner `global`/`exists` accesses
//! with the pre-state snapshot label `0`; quantifier binders carry their
//! domain type into the typed `quant` node; `>`/`>=`/`!=` normalize as in
//! the instruction mapping.

use anyhow::{anyhow, bail, Result};
use move_model::{
    ast::{Address, Exp, ExpData, Operation, Pattern, QuantKind, Value as ModelValue},
    model::{GlobalEnv, QualifiedId, StructId},
    symbol::Symbol,
    ty::{PrimitiveType, Type},
};
use move_model_exchange as exchange;
use num::BigInt;
use std::collections::BTreeMap;

/// Name-resolution context for model spec expressions.
pub struct ModelSpecCtx<'a> {
    pub env: &'a GlobalEnv,
    /// Module-local struct ids to dense resource ids.
    pub structs: &'a BTreeMap<QualifiedId<StructId>, usize>,
}

impl ModelSpecCtx<'_> {
    /// The dense resource id of the (unique) struct type instantiating the
    /// given node (of a `Global`/`Exists` operation).
    fn resource_of_node(&self, id: move_model::model::NodeId) -> Result<usize> {
        let inst = self.env.get_node_instantiation(id);
        let [ty] = inst.as_slice() else {
            bail!("expected exactly one type instantiation");
        };
        let Type::Struct(mid, sid, targs) = ty else {
            bail!("expected a struct type");
        };
        if !targs.is_empty() {
            bail!("generic resources not supported");
        }
        self.structs
            .get(&mid.qualified(*sid))
            .copied()
            .ok_or_else(|| anyhow!("resource of foreign module not supported"))
    }
}

fn u64_of(n: &BigInt) -> Result<u64> {
    u64::try_from(n).map_err(|_| anyhow!("number does not fit in u64"))
}

/// Translates one move-model spec expression.
pub fn translate_exp(
    ctx: &ModelSpecCtx,
    binders: &mut Vec<Symbol>,
    in_old: bool,
    exp: &ExpData,
) -> Result<exchange::SpecExp> {
    let label = in_old.then_some(0);
    match exp {
        ExpData::Value(_, v) => match v {
            ModelValue::Number(n) => Ok(exchange::SpecExp::Value(exchange::Value::Num(
                u64_of(n)?.to_string(),
            ))),
            ModelValue::Bool(b) => Ok(exchange::SpecExp::Value(exchange::Value::Bool(*b))),
            ModelValue::Address(Address::Numerical(a)) => Ok(exchange::SpecExp::Value(
                exchange::Value::Address(a.to_hex_literal()),
            )),
            v => bail!("unsupported constant {:?} in spec", v),
        },
        ExpData::Temporary(_, i) => Ok(exchange::SpecExp::Local(*i)),
        ExpData::LocalVar(_, sym) => {
            if let Some(pos) = binders.iter().rposition(|b| b == sym) {
                Ok(exchange::SpecExp::Bvar(binders.len() - 1 - pos))
            } else {
                bail!(
                    "unsupported local `{}` in spec expression",
                    sym.display(ctx.env.symbol_pool())
                )
            }
        },
        ExpData::Call(id, op, args) => {
            let arg = |i: usize| -> &ExpData { args[i].as_ref() };
            let pair = |ctx: &ModelSpecCtx,
                        binders: &mut Vec<Symbol>|
             -> Result<(exchange::SpecExp, exchange::SpecExp)> {
                Ok((
                    translate_exp(ctx, binders, in_old, arg(0))?,
                    translate_exp(ctx, binders, in_old, arg(1))?,
                ))
            };
            let bin = |ctx: &ModelSpecCtx,
                       binders: &mut Vec<Symbol>,
                       op: exchange::SpecBinOp|
             -> Result<exchange::SpecExp> {
                let (l, r) = pair(ctx, binders)?;
                Ok(exchange::SpecExp::binop(op, l, r))
            };
            match op {
                Operation::Old => translate_exp(ctx, binders, true, arg(0)),
                Operation::Global(None) => {
                    let r = ctx.resource_of_node(*id)?;
                    let a = translate_exp(ctx, binders, in_old, arg(0))?;
                    Ok(exchange::SpecExp::Global(r, label, Box::new(a)))
                },
                Operation::Exists(None) => {
                    let r = ctx.resource_of_node(*id)?;
                    let a = translate_exp(ctx, binders, in_old, arg(0))?;
                    Ok(exchange::SpecExp::Exists(r, label, Box::new(a)))
                },
                Operation::Select(mid, sid, fid) => {
                    let module = ctx.env.get_module(*mid);
                    let struct_env = module.into_struct(*sid);
                    let field = struct_env.get_field(*fid);
                    if field.is_ghost() {
                        bail!(
                            "ghost field `{}` not supported",
                            field.get_name().display(ctx.env.symbol_pool())
                        );
                    }
                    let offset = field.get_offset();
                    let e = translate_exp(ctx, binders, in_old, arg(0))?;
                    Ok(exchange::SpecExp::Select(offset, Box::new(e)))
                },
                Operation::Result(i) => Ok(exchange::SpecExp::Result(*i)),
                Operation::Not => {
                    let e = translate_exp(ctx, binders, in_old, arg(0))?;
                    Ok(exchange::SpecExp::Not(Box::new(e)))
                },
                Operation::Add => bin(ctx, binders, exchange::SpecBinOp::Add),
                Operation::Sub => bin(ctx, binders, exchange::SpecBinOp::Sub),
                Operation::Mul => bin(ctx, binders, exchange::SpecBinOp::Mul),
                Operation::Div => bin(ctx, binders, exchange::SpecBinOp::Div),
                Operation::Mod => bin(ctx, binders, exchange::SpecBinOp::Mod),
                Operation::Lt => bin(ctx, binders, exchange::SpecBinOp::Lt),
                Operation::Le => bin(ctx, binders, exchange::SpecBinOp::Le),
                Operation::Gt => {
                    let (l, r) = pair(ctx, binders)?;
                    Ok(exchange::SpecExp::gt(l, r))
                },
                Operation::Ge => {
                    let (l, r) = pair(ctx, binders)?;
                    Ok(exchange::SpecExp::ge(l, r))
                },
                Operation::Eq => bin(ctx, binders, exchange::SpecBinOp::Eq),
                Operation::Neq => {
                    let (l, r) = pair(ctx, binders)?;
                    Ok(exchange::SpecExp::neq(l, r))
                },
                Operation::And => bin(ctx, binders, exchange::SpecBinOp::And),
                Operation::Or => bin(ctx, binders, exchange::SpecBinOp::Or),
                Operation::Implies => bin(ctx, binders, exchange::SpecBinOp::Implies),
                Operation::Iff => bin(ctx, binders, exchange::SpecBinOp::Iff),
                op => bail!("unsupported spec operation {:?}", op),
            }
        },
        ExpData::IfElse(_, c, t, e) => {
            let c = translate_exp(ctx, binders, in_old, c.as_ref())?;
            let t = translate_exp(ctx, binders, in_old, t.as_ref())?;
            let e = translate_exp(ctx, binders, in_old, e.as_ref())?;
            Ok(exchange::SpecExp::Ite(
                Box::new(c),
                Box::new(t),
                Box::new(e),
            ))
        },
        ExpData::Quant(_, kind, ranges, _triggers, where_opt, body) => {
            if where_opt.is_some() {
                bail!("quantifier `where` clauses not supported");
            }
            let [(pat, domain)] = ranges.as_slice() else {
                bail!("only single-variable quantifiers are supported");
            };
            let Pattern::Var(_, sym) = pat else {
                bail!("only simple quantifier binders are supported");
            };
            let ExpData::Call(dom_id, Operation::TypeDomain, _) = domain.as_ref() else {
                bail!("only type-domain quantifiers are supported");
            };
            let dom = match ctx.env.get_node_instantiation(*dom_id).as_slice() {
                [Type::Primitive(PrimitiveType::Address)] => exchange::Type::Address,
                [Type::Primitive(PrimitiveType::U64)] => exchange::Type::U64,
                ty => bail!("unsupported quantifier domain {:?}", ty),
            };
            binders.push(*sym);
            let b = translate_exp(ctx, binders, in_old, body.as_ref());
            binders.pop();
            let b = b?;
            let quant_kind = match kind {
                QuantKind::Forall => exchange::QuantKind::All,
                QuantKind::Exists => exchange::QuantKind::Ex,
                kind => bail!("unsupported quantifier kind {:?}", kind),
            };
            Ok(exchange::SpecExp::quant(quant_kind, dom, b))
        },
        exp => bail!(
            "unsupported spec expression {}",
            exp.display_verbose(ctx.env)
        ),
    }
}

/// Whether `exp` reads a bytecode temporary beneath `old(..)`.  Function
/// contracts bind parameter locals to entry values, but loop invariants do
/// not currently carry snapshots of local values, so such invariants must be
/// rejected instead of silently reading the current local.
pub fn contains_old_temporary(exp: &Exp) -> bool {
    let mut old_depth = 0usize;
    let mut found = false;
    exp.visit_pre_post(&mut |post, node| {
        if matches!(node, ExpData::Call(_, Operation::Old, _)) {
            if post {
                old_depth -= 1;
            } else {
                old_depth += 1;
            }
        } else if !post && old_depth > 0 && matches!(node, ExpData::Temporary(..)) {
            found = true;
        }
        !found
    });
    found
}

/// A modifies target `global<R>(addr)`: resource id and address expression.
pub fn translate_modifies_target(
    ctx: &ModelSpecCtx,
    exp: &Exp,
) -> Result<exchange::ModifiesTarget> {
    let ExpData::Call(id, Operation::Global(None), args) = exp.as_ref() else {
        bail!("modifies target must be a `global<R>(addr)` expression");
    };
    let r = ctx.resource_of_node(*id)?;
    let mut binders = vec![];
    let a = translate_exp(ctx, &mut binders, false, args[0].as_ref())?;
    Ok(exchange::ModifiesTarget {
        resource: r,
        addr: a,
    })
}
