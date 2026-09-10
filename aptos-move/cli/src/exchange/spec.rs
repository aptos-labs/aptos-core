// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Translation of masm specification clauses (`move_asm::syntax::SpecClause`)
//! into the exchange spec-expression form (`move_model_exchange::SpecExp`).
//!
//! Name resolution: identifiers denote parameters/locals (their stackless
//! temp index) or quantifier binders (de Bruijn indices); struct names
//! denote dense resource ids; field names resolve type-directedly against
//! the struct sort of their operand.  Number literals in address positions
//! (`global`/`exists`/`modifies` addresses) denote addresses — masm has no
//! separate address-literal token.  `old(..)` marks inner
//! `global`/`exists` accesses with
//! the pre-state snapshot label `0` — matching the Lean side's `preLabel` —
//! and is the identity on parameters (contract environments bind parameter
//! temporaries to the entry arguments).  Quantifier binders carry their
//! domain type (`address`, `u64`) into the typed `quant` node; the
//! consumer's semantics bounds the range by the domain.
//! Loop invariants containing `old(..)` of a local are rejected until
//! loop-entry local snapshots are represented explicitly; treating `old(x)`
//! as current `x` would be unsound.  Old global memory remains supported.
//!
//! `>`/`>=` translate to `<`/`<=` with swapped operands, `!=` to negated
//! equality, mirroring the instruction mapping.

use anyhow::{anyhow, bail, Result};
use move_asm::syntax::{Fun, SpecBinOp, SpecClause, SpecExp, SpecQuantKind};
use move_model_exchange::{self as exchange, check};
use std::collections::BTreeMap;

/// Name-resolution context for spec expressions.
pub struct SpecCtx<'a> {
    /// Parameters and locals by name, to their local index.
    pub temps: BTreeMap<String, usize>,
    /// Struct names to dense resource ids.
    pub structs: BTreeMap<String, usize>,
    /// The dumped struct declarations, for type-directed field resolution.
    pub struct_decls: &'a [exchange::Struct],
    /// Declared types of all locals (parameters first).
    pub local_types: &'a [exchange::Type],
    pub num_params: usize,
    pub returns: &'a [exchange::Type],
}

impl SpecCtx<'_> {
    fn resource(&self, name: &str) -> Result<usize> {
        self.structs
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("unknown struct `{}`", name))
    }

    /// The spec sort of an already-translated operand (under the most
    /// permissive scope; the position discipline is enforced by the final
    /// `check_function` pass).
    fn sort_of(
        &self,
        binders: &[(String, exchange::Type)],
        e: &exchange::SpecExp,
    ) -> Result<check::SpecSort> {
        let mut sorts: Vec<check::SpecSort> = binders
            .iter()
            .map(|(_, ty)| check::SpecSort::of(ty))
            .collect();
        check::SpecCheckCtx {
            structs: self.struct_decls,
            locals: self.local_types,
            num_params: self.num_params,
            returns: self.returns,
        }
        .sort(check::ClausePos::Any, &mut sorts, e)
        .map_err(|e| anyhow!("{}", e))
    }
}

fn u64_of(n: &move_core_types::int256::U256) -> Result<u64> {
    u64::try_from(*n).map_err(|_| anyhow!("number does not fit in u64"))
}

/// A number literal as a `0x`-hex address literal (minimal digits).
fn address_literal(n: &move_core_types::int256::U256) -> String {
    let hex: String = n
        .to_le_bytes()
        .iter()
        .rev()
        .map(|b| format!("{:02x}", b))
        .collect();
    let trimmed = hex.trim_start_matches('0');
    format!("0x{}", if trimmed.is_empty() { "0" } else { trimmed })
}

/// Translates one spec expression.  `binders` is the stack of quantifier
/// binders (name and domain type), innermost last.
fn translate_exp(
    ctx: &SpecCtx,
    binders: &mut Vec<(String, exchange::Type)>,
    in_old: bool,
    e: &SpecExp,
) -> Result<exchange::SpecExp> {
    let label = in_old.then_some(0);
    match e {
        SpecExp::Number(n) => Ok(exchange::SpecExp::Value(exchange::Value::Num(
            u64_of(n)?.to_string(),
        ))),
        SpecExp::Bool(b) => Ok(exchange::SpecExp::Value(exchange::Value::Bool(*b))),
        SpecExp::Ident(id) => {
            let name = id.as_str();
            if let Some(pos) = binders.iter().rposition(|(b, _)| b == name) {
                return Ok(exchange::SpecExp::Bvar(binders.len() - 1 - pos));
            }
            if let Some(idx) = ctx.temps.get(name) {
                return Ok(exchange::SpecExp::Local(*idx));
            }
            // Result pseudo-names are intentionally resolved only after
            // lexical binders and declared locals so legal masm identifiers
            // named `result`/`result_N` shadow them normally.
            if name == "result" {
                return Ok(exchange::SpecExp::Result(0));
            }
            if let Some(rest) = name.strip_prefix("result_") {
                if let Ok(i) = rest.parse::<usize>() {
                    if i == 0 {
                        bail!("result positions are 1-based");
                    }
                    return Ok(exchange::SpecExp::Result(i - 1));
                }
            }
            bail!("unknown identifier `{}` in spec expression", name)
        },
        SpecExp::Result(i) => Ok(exchange::SpecExp::Result(*i)),
        SpecExp::Old(inner) => translate_exp(ctx, binders, true, inner),
        SpecExp::Global { resource, addr } => {
            let r = ctx.resource(resource.as_str())?;
            let a = translate_addr_exp(ctx, binders, in_old, addr)?;
            Ok(exchange::SpecExp::Global(r, label, Box::new(a)))
        },
        SpecExp::ResourceExists { resource, addr } => {
            let r = ctx.resource(resource.as_str())?;
            let a = translate_addr_exp(ctx, binders, in_old, addr)?;
            Ok(exchange::SpecExp::Exists(r, label, Box::new(a)))
        },
        SpecExp::Select { exp, field } => {
            let e = translate_exp(ctx, binders, in_old, exp)?;
            // Type-directed: resolve the field within the operand's struct.
            let offset = match ctx.sort_of(binders, &e)? {
                check::SpecSort::Struct(r) => {
                    let decl = ctx
                        .struct_decls
                        .get(r)
                        .ok_or_else(|| anyhow!("undeclared resource {}", r))?;
                    decl.fields
                        .iter()
                        .position(|f| f.name == field.as_str())
                        .ok_or_else(|| anyhow!("struct `{}` has no field `{}`", decl.name, field))?
                },
                s => bail!("field selection applies to a struct, found {}", s),
            };
            Ok(exchange::SpecExp::Select(offset, Box::new(e)))
        },
        SpecExp::Not(inner) => {
            let e = translate_exp(ctx, binders, in_old, inner)?;
            Ok(exchange::SpecExp::Not(Box::new(e)))
        },
        SpecExp::Binary(op, lhs, rhs) => {
            let l = translate_exp(ctx, binders, in_old, lhs)?;
            let r = translate_exp(ctx, binders, in_old, rhs)?;
            let op = match op {
                SpecBinOp::Add => exchange::SpecBinOp::Add,
                SpecBinOp::Sub => exchange::SpecBinOp::Sub,
                SpecBinOp::Mul => exchange::SpecBinOp::Mul,
                SpecBinOp::Div => exchange::SpecBinOp::Div,
                SpecBinOp::Mod => exchange::SpecBinOp::Mod,
                SpecBinOp::Lt => exchange::SpecBinOp::Lt,
                SpecBinOp::Le => exchange::SpecBinOp::Le,
                SpecBinOp::Eq => exchange::SpecBinOp::Eq,
                SpecBinOp::And => exchange::SpecBinOp::And,
                SpecBinOp::Or => exchange::SpecBinOp::Or,
                SpecBinOp::Implies => exchange::SpecBinOp::Implies,
                SpecBinOp::Gt => return Ok(exchange::SpecExp::gt(l, r)),
                SpecBinOp::Ge => return Ok(exchange::SpecExp::ge(l, r)),
                SpecBinOp::Neq => return Ok(exchange::SpecExp::neq(l, r)),
            };
            Ok(exchange::SpecExp::binop(op, l, r))
        },
        SpecExp::Quant {
            kind,
            var,
            ty,
            body,
        } => {
            let dom = match ty.as_str() {
                "address" => exchange::Type::Address,
                "u64" => exchange::Type::U64,
                other => bail!("unsupported quantifier domain `{}`", other),
            };
            binders.push((var.as_str().to_string(), dom.clone()));
            let b = translate_exp(ctx, binders, in_old, body);
            binders.pop();
            let b = b?;
            let quant_kind = match kind {
                SpecQuantKind::Forall => exchange::QuantKind::All,
                SpecQuantKind::Exists => exchange::QuantKind::Ex,
            };
            Ok(exchange::SpecExp::quant(quant_kind, dom, b))
        },
    }
}

/// Whether an expression reads a function local beneath `old(..)`.  Old
/// global memory is represented by a label and remains supported; loop-entry
/// local snapshots are not represented by the exchange format.
fn contains_old_local(ctx: &SpecCtx, e: &SpecExp) -> bool {
    fn go(ctx: &SpecCtx, binders: &mut Vec<String>, in_old: bool, e: &SpecExp) -> bool {
        match e {
            SpecExp::Old(inner) => go(ctx, binders, true, inner),
            SpecExp::Ident(id) => {
                in_old
                    && !binders.iter().rev().any(|b| b == id.as_str())
                    && ctx.temps.contains_key(id.as_str())
            },
            SpecExp::Select { exp, .. } | SpecExp::Not(exp) => go(ctx, binders, in_old, exp),
            SpecExp::Global { addr, .. } | SpecExp::ResourceExists { addr, .. } => {
                go(ctx, binders, in_old, addr)
            },
            SpecExp::Binary(_, lhs, rhs) => {
                go(ctx, binders, in_old, lhs) || go(ctx, binders, in_old, rhs)
            },
            SpecExp::Quant { var, body, .. } => {
                binders.push(var.as_str().to_string());
                let found = go(ctx, binders, in_old, body);
                binders.pop();
                found
            },
            SpecExp::Number(_) | SpecExp::Bool(_) | SpecExp::Result(_) => false,
        }
    }
    go(ctx, &mut vec![], false, e)
}

/// Translates an expression in address position: a number literal denotes
/// an address there (masm has no separate address-literal token).
fn translate_addr_exp(
    ctx: &SpecCtx,
    binders: &mut Vec<(String, exchange::Type)>,
    in_old: bool,
    e: &SpecExp,
) -> Result<exchange::SpecExp> {
    if let SpecExp::Number(n) = e {
        return Ok(exchange::SpecExp::Value(exchange::Value::Address(
            address_literal(n),
        )));
    }
    translate_exp(ctx, binders, in_old, e)
}

/// The spec-relevant data of one masm function, extracted from the parsed
/// AST before assembly consumes it (the assembler ignores spec clauses, so
/// they are taken rather than cloned).
pub struct FunSpecInput {
    pub name: String,
    /// Parameter and local names, in local-index order.
    pub decls: Vec<String>,
    /// The label of each instruction (code order), for resolving
    /// `invariant <label>:` clauses.
    pub labels: Vec<Option<String>>,
    pub clauses: Vec<SpecClause>,
}

impl FunSpecInput {
    /// Extracts the spec inputs of `fun`, taking its spec clauses.
    pub fn take(fun: &mut Fun) -> FunSpecInput {
        let labels: Vec<Option<String>> = fun
            .instrs
            .iter()
            .map(|i| i.label.as_ref().map(|l| l.as_str().to_string()))
            .collect();
        FunSpecInput {
            name: fun.name.as_str().to_string(),
            decls: fun
                .params
                .iter()
                .chain(fun.locals.iter())
                .map(|d| d.name.as_str().to_string())
                .collect(),
            labels,
            clauses: std::mem::take(&mut fun.spec_clauses),
        }
    }
}

/// The translated spec clauses of one function: the contract plus the loop
/// invariants, keyed by the masm instruction index their label is attached
/// to.
pub struct TranslatedSpec {
    pub contract: exchange::Contract,
    pub invariants: Vec<(usize, exchange::SpecExp)>,
}

/// Translates the spec clauses of `input`.
pub fn translate_clauses(ctx: &SpecCtx, input: &FunSpecInput) -> Result<TranslatedSpec> {
    let mut requires = vec![];
    let mut aborts_if = vec![];
    let mut ensures = vec![];
    let mut modifies = vec![];
    let mut invariants = vec![];
    for clause in &input.clauses {
        let mut binders = vec![];
        match clause {
            SpecClause::Requires(e) => requires.push(translate_exp(ctx, &mut binders, false, e)?),
            SpecClause::AbortsIf(e) => aborts_if.push(translate_exp(ctx, &mut binders, false, e)?),
            SpecClause::Ensures(e) => ensures.push(translate_exp(ctx, &mut binders, false, e)?),
            SpecClause::Modifies { resource, addr } => {
                let r = ctx.resource(resource.as_str())?;
                let a = translate_addr_exp(ctx, &mut binders, false, addr)?;
                modifies.push(exchange::ModifiesTarget {
                    resource: r,
                    addr: a,
                });
            },
            SpecClause::Invariant { label, exp } => {
                if contains_old_local(ctx, exp) {
                    bail!(
                        "`old(..)` of a local in loop invariants is unsupported because local loop-entry snapshots are not represented"
                    );
                }
                let pos = input
                    .labels
                    .iter()
                    .position(|l| l.as_deref() == Some(label.as_str()))
                    .ok_or_else(|| anyhow!("invariant refers to unknown label `{}`", label))?;
                invariants.push((pos, translate_exp(ctx, &mut binders, false, exp)?));
            },
        }
    }
    invariants.sort_by_key(|(pos, _)| *pos);
    Ok(TranslatedSpec {
        contract: exchange::Contract {
            requires,
            aborts_if,
            ensures,
            modifies,
        },
        invariants,
    })
}
