// Revela decompiler. Copyright (c) Verichains, 2023-2024

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Display,
    rc::Rc,
};

use anyhow::Ok;
use move_model::{
    model::{FunctionEnv, ModuleId},
    ty::Type,
};
use move_stackless_bytecode::stackless_bytecode::{AssignKind, Bytecode, Constant};

pub type ExprNodeRef = Rc<RefCell<ExprNode>>;
#[derive(Debug, PartialEq)]
pub enum ExprNodeOperation {
    Ignored,
    #[allow(dead_code)]
    Deleted,
    NonTrivial,
    Raw(String),
    Const(Constant),
    LocalVariable(usize),
    Field(ExprNodeRef, String),
    Unary(String, ExprNodeRef),
    Cast(String, ExprNodeRef),
    Binary(String, ExprNodeRef, ExprNodeRef),
    Func(String, Vec<ExprNodeRef>, Vec<Type>),

    Destroy(ExprNodeRef),
    FreezeRef(ExprNodeRef),
    ReadRef(ExprNodeRef),
    BorrowLocal(ExprNodeRef, /* mut */ bool),
    WriteRef(ExprNodeRef /* dst */, ExprNodeRef /* src */),
    StructPack(
        String, /* struct name */
        Vec<(
            String,      /* field name */
            ExprNodeRef, /* field value */
        )>,
        Vec<Type>,
    ),
    StructUnpack(
        String,      /* struct name */
        Vec<String>, /* field names */
        ExprNodeRef,
        Vec<Type>,
    ),

    VariableSnapshot {
        variable: usize,
        assigment_id: usize,
        value: ExprNodeRef,
    },
}

#[derive(Clone, Debug)]
struct ToSourceCtx {
    in_borrow: bool,
}

impl ToSourceCtx {
    fn default() -> Self {
        Self { in_borrow: false }
    }
}

pub fn effective_operation<R, const N: usize>(
    nodes: &[&ExprNodeRef; N],
    cb: &mut dyn FnMut(&[&ExprNodeRef; N]) -> R,
) -> R {
    fn resolve_node<R, const N: usize>(
        nodes: &[&ExprNodeRef; N],
        arr: &mut Vec<&ExprNodeRef>,
        idx: usize,
        cb: &mut dyn FnMut(&[&ExprNodeRef; N]) -> R,
    ) -> R {
        if idx == N {
            let mut fixed_arr = [nodes[0]; N];
            for i in 0..N {
                fixed_arr[i] = arr[i];
            }
            return cb(&fixed_arr);
        }
        let node = if idx < arr.len() {
            arr[idx]
        } else {
            nodes[idx]
        };
        let mut arr = arr.clone();
        if arr.len() <= idx {
            arr.push(&node);
        } else {
            arr[idx] = &node;
        }
        let node = node.borrow();
        match &node.operation {
            ExprNodeOperation::VariableSnapshot { value, .. } => {
                arr[idx] = value;
                resolve_node(nodes, &mut arr, idx, cb)
            }
            _ => resolve_node(nodes, &mut arr, idx + 1, cb),
        }
    }

    let mut effective_nodes = Vec::new();

    resolve_node(nodes, &mut effective_nodes, 0, cb)
}

impl ExprNodeOperation {
    pub fn copy(&self) -> Self {
        match self {
            ExprNodeOperation::Unary(op, arg) => {
                ExprNodeOperation::Unary(op.clone(), arg.borrow().copy_as_ref())
            }
            ExprNodeOperation::Cast(op, arg) => {
                ExprNodeOperation::Cast(op.clone(), arg.borrow().copy_as_ref())
            }
            ExprNodeOperation::Binary(op, lhs, rhs) => ExprNodeOperation::Binary(
                op.clone(),
                lhs.borrow().copy_as_ref(),
                rhs.borrow().copy_as_ref(),
            ),
            ExprNodeOperation::Func(name, args, types) => ExprNodeOperation::Func(
                name.clone(),
                args.iter().map(|x| x.borrow().copy_as_ref()).collect(),
                types.clone(),
            ),
            ExprNodeOperation::StructPack(name, args, types) => ExprNodeOperation::StructPack(
                name.clone(),
                args.iter()
                    .map(|x| ((x.0.clone(), x.1.borrow().copy_as_ref())))
                    .collect(),
                types.clone(),
            ),
            ExprNodeOperation::StructUnpack(name, keys, val, types) => {
                ExprNodeOperation::StructUnpack(
                    name.clone(),
                    keys.clone(),
                    val.borrow().copy_as_ref(),
                    types.clone(),
                )
            }
            ExprNodeOperation::Field(expr, name) => {
                ExprNodeOperation::Field(expr.borrow().copy_as_ref(), name.clone())
            }
            ExprNodeOperation::ReadRef(expr) => {
                ExprNodeOperation::ReadRef(expr.borrow().copy_as_ref())
            }
            ExprNodeOperation::BorrowLocal(expr, mutable) => {
                ExprNodeOperation::BorrowLocal(expr.borrow().copy_as_ref(), *mutable)
            }
            ExprNodeOperation::FreezeRef(expr) => {
                ExprNodeOperation::FreezeRef(expr.borrow().copy_as_ref())
            }
            ExprNodeOperation::Destroy(expr) => {
                ExprNodeOperation::Destroy(expr.borrow().copy_as_ref())
            }
            ExprNodeOperation::WriteRef(lhs, rhs) => {
                ExprNodeOperation::WriteRef(lhs.borrow().copy_as_ref(), rhs.borrow().copy_as_ref())
            }
            ExprNodeOperation::Raw(name) => ExprNodeOperation::Raw(name.clone()),
            ExprNodeOperation::Const(c) => ExprNodeOperation::Const(c.clone()),
            ExprNodeOperation::Ignored => ExprNodeOperation::Ignored,
            ExprNodeOperation::Deleted => ExprNodeOperation::Deleted,
            ExprNodeOperation::NonTrivial => ExprNodeOperation::NonTrivial,
            ExprNodeOperation::LocalVariable(idx) => ExprNodeOperation::LocalVariable(idx.clone()),
            ExprNodeOperation::VariableSnapshot {
                variable,
                assigment_id,
                value,
            } => ExprNodeOperation::VariableSnapshot {
                variable: variable.clone(),
                assigment_id: assigment_id.clone(),
                value: value.borrow().copy_as_ref(),
            },
        }
    }
    pub fn to_node(&self) -> ExprNodeRef {
        Rc::new(RefCell::new(ExprNode {
            operation: self.copy(),
        }))
    }
    pub fn to_expr(&self) -> Expr {
        Expr::new(self.to_node())
    }
    fn typeparams_to_source(types: &Vec<Type>, naming: &Naming) -> String {
        if types.is_empty() {
            String::new()
        } else {
            format!(
                "<{}>",
                types
                    .iter()
                    .map(|x| naming.ty(x))
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        }
    }
    fn const_to_source(val: &Constant) -> Result<String, anyhow::Error> {
        match val {
            Constant::Bool(v) => Ok(format!("{}", v)),
            Constant::U8(x) => Ok(format!("{}", x)),
            Constant::U16(x) => Ok(format!("{}", x)),
            Constant::U32(x) => Ok(format!("{}", x)),
            Constant::U64(x) => Ok(format!("{}", x)),
            Constant::U128(x) => Ok(format!("{}", x)),
            Constant::U256(x) => Ok(format!("{}", x)),
            Constant::Address(x) => match x {
                move_model::ast::Address::Numerical(val) => {
                    Ok(format!("@{}", val.to_hex_literal()))
                }
                move_model::ast::Address::Symbolic(_val) => {
                    unreachable!("There must be no symbolic address in compiled binary")
                }
            },
            Constant::ByteArray(v) => {
                let is_safe = v.iter().all(|x| *x >= 0x20 && *x <= 0x7e);
                if is_safe {
                    Ok(format!(
                        "b\"{}\"",
                        v.iter()
                            .map(|x| *x as char)
                            .collect::<String>()
                            .replace("\\", "\\\\")
                            .replace("\"", "\\\"")
                    ))
                } else {
                    Ok(format!(
                        "x\"{}\"",
                        v.iter()
                            .map(|x| format!("{:02x}", x))
                            .collect::<Vec<_>>()
                            .join(""),
                    ))
                }
            }
            Constant::AddressArray(v) => Ok(format!(
                "vector[{}]",
                v.iter()
                    .map(|x| Self::const_to_source(&Constant::Address(x.clone())))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", "),
            )),
            Constant::Vector(v) => Ok(format!(
                "vector[{}]",
                v.iter()
                    .map(|x| Self::const_to_source(x))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", "),
            )),
        }
    }
    pub fn to_source_decl(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        match self {
            ExprNodeOperation::StructPack(name, args, types) => {
                if args.len() < 2 {
                    return self.to_source(naming);
                }
                let k_width = args.iter().map(|x| x.0.len()).max().unwrap();
                Ok(format!(
                    "{}{}{{\n{},\n}}",
                    name,
                    Self::typeparams_to_source(types, naming),
                    args.iter()
                        .map(|x| x.1.borrow().to_source(naming).and_then(|v| Ok(format!(
                            "{:width$} : {}",
                            x.0,
                            v,
                            width = k_width
                        ))))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", \n")
                ))
            }
            _ => self.to_source(naming),
        }
    }
    pub fn to_source(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        let ctx = ToSourceCtx::default();
        self.to_source_with_ctx(naming, &ctx)
    }
    fn to_source_with_ctx(
        &self,
        naming: &Naming,
        ctx: &ToSourceCtx,
    ) -> Result<String, anyhow::Error> {
        let mut ctx = ctx.clone();
        if ctx.in_borrow {
            match self {
                ExprNodeOperation::BorrowLocal(..) => {}
                ExprNodeOperation::Field(..) => {}
                _ => {
                    ctx.in_borrow = false;
                }
            }
        }

        match self {
            ExprNodeOperation::LocalVariable(idx) => Ok(naming.variable(*idx)),
            ExprNodeOperation::Ignored => Ok("_".to_string()),
            ExprNodeOperation::Deleted => Ok("<<< !!! deleted !!! >>>".to_string()),
            ExprNodeOperation::NonTrivial => Ok("!!non-trivial!!".to_string()),
            ExprNodeOperation::Raw(x) => Ok(format!("((/*raw:*/{}))", x)),
            ExprNodeOperation::Const(c) => Self::const_to_source(c),
            ExprNodeOperation::Field(expr, name) => {
                // &(&object).field -> & object.field
                if ctx.in_borrow {
                    if let Some(r) = effective_operation(&[expr], &mut |[e]| -> Option<
                        Result<String, anyhow::Error>,
                    > {
                        let e = e.borrow();
                        if let ExprNodeOperation::BorrowLocal(inner_expr, _) = &e.operation {
                            let r = bracket_if_binary_with_ctx(inner_expr, Some(naming), &ctx);
                            match r {
                                std::result::Result::Ok(v) => {
                                    return Some(Ok(format!("{}.{}", v, name)))
                                }
                                Err(_) => return Some(r),
                            }
                        }
                        None
                    }) {
                        return r;
                    }
                }
                Ok(format!(
                    "{}.{}",
                    bracket_if_binary_with_ctx(expr, Some(naming), &ctx)?,
                    name
                ))
            }
            ExprNodeOperation::Unary(op, expr) => Ok(format!(
                "{}{}",
                op,
                bracket_if_binary_with_ctx(expr, Some(naming), &ctx)?
            )),
            ExprNodeOperation::Cast(ty, expr) => Ok(format!(
                "{} as {}",
                bracket_if_binary_with_ctx(expr, Some(naming), &ctx)?,
                ty
            )),
            ExprNodeOperation::Binary(op, a, b) => {
                let a_str = check_bracket_for_binary(a, get_precedence(op), Some(naming), &ctx)?;
                let b_str = check_bracket_for_binary(b, get_precedence(op), Some(naming), &ctx)?;
                Ok(format!("{} {} {}", a_str, op, b_str))
            }
            ExprNodeOperation::Func(name, args, types) => Ok(format!(
                "{}{}({})",
                name,
                Self::typeparams_to_source(types, naming),
                args.iter()
                    .map(|x| x.borrow().to_source_with_ctx(naming, &ctx))
                    .collect::<Result<Vec<String>, anyhow::Error>>()?
                    .join(", ")
            )),
            ExprNodeOperation::Destroy(expr) => Ok(format!(
                "/*destroyed:{}*/",
                expr.borrow().to_source_with_ctx(naming, &ctx)?
            )),
            ExprNodeOperation::FreezeRef(expr) => expr.borrow().to_source_with_ctx(naming, &ctx),
            ExprNodeOperation::ReadRef(expr) => {
                effective_operation(&[expr], &mut |[expr]| match &expr.borrow().operation {
                    ExprNodeOperation::BorrowLocal(inner_expr, _) => {
                        // cleanup *&, *&mut
                        ctx.in_borrow = true;
                        Ok(format!(
                            "{}",
                            inner_expr.borrow().to_source_with_ctx(naming, &ctx)?
                        ))
                    }
                    _ => Ok(format!(
                        "*{}",
                        bracket_if_binary_with_ctx(expr, Some(naming), &ctx)?
                    )),
                })
            }
            ExprNodeOperation::BorrowLocal(expr, mutable) => {
                ctx.in_borrow = true;
                if *mutable {
                    Ok(format!(
                        "&mut {}",
                        expr.borrow().to_source_with_ctx(naming, &ctx)?
                    ))
                } else {
                    Ok(format!(
                        "&{}",
                        expr.borrow().to_source_with_ctx(naming, &ctx)?
                    ))
                }
            }
            ExprNodeOperation::WriteRef(lhs, rhs) => Ok(format!(
                "{} = {}",
                ExprNodeOperation::ReadRef(lhs.clone()).to_source_with_ctx(naming, &ctx)?,
                rhs.borrow().to_source_with_ctx(naming, &ctx)?
            )),
            ExprNodeOperation::StructPack(name, args, types) => Ok(format!(
                "{}{}{{{}}}",
                name,
                Self::typeparams_to_source(types, naming),
                args.iter()
                    .map(|x| x
                        .1
                        .borrow()
                        .to_source_with_ctx(naming, &ctx)
                        .and_then(|v| Ok(format!("{}: {}", x.0, v))))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
            ExprNodeOperation::StructUnpack(name, keys, val, types) => Ok(format!(
                "{}{}{{{}}} = {}",
                name,
                Self::typeparams_to_source(types, naming),
                keys.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                val.borrow().to_source_with_ctx(naming, &ctx)?
            )),
            ExprNodeOperation::VariableSnapshot { value, .. } => {
                value.borrow().to_source_with_ctx(naming, &ctx)
            }
        }
    }

    fn collect_variables(
        &self,
        result_variables: &mut HashSet<usize>,
        implicit_variables: &mut HashSet<usize>,
        in_implicit_expr: bool,
    ) {
        match self {
            ExprNodeOperation::LocalVariable(idx) => {
                if in_implicit_expr {
                    implicit_variables.insert(*idx);
                } else {
                    result_variables.insert(*idx);
                }
            }
            ExprNodeOperation::Ignored
            | ExprNodeOperation::Deleted
            | ExprNodeOperation::NonTrivial
            | ExprNodeOperation::Raw(..)
            | ExprNodeOperation::Const(..) => {}
            ExprNodeOperation::Field(expr, _) => expr.borrow().collect_variables(
                result_variables,
                implicit_variables,
                in_implicit_expr,
            ),
            ExprNodeOperation::Unary(_, expr) => expr.borrow().collect_variables(
                result_variables,
                implicit_variables,
                in_implicit_expr,
            ),
            ExprNodeOperation::Cast(_, expr) => expr.borrow().collect_variables(
                result_variables,
                implicit_variables,
                in_implicit_expr,
            ),
            ExprNodeOperation::Binary(_, a, b) => {
                a.borrow().collect_variables(
                    result_variables,
                    implicit_variables,
                    in_implicit_expr,
                );
                b.borrow().collect_variables(
                    result_variables,
                    implicit_variables,
                    in_implicit_expr,
                );
            }
            ExprNodeOperation::Func(_, args, _) => {
                for arg in args {
                    arg.borrow().collect_variables(
                        result_variables,
                        implicit_variables,
                        in_implicit_expr,
                    );
                }
            }
            ExprNodeOperation::Destroy(expr)
            | ExprNodeOperation::FreezeRef(expr)
            | ExprNodeOperation::ReadRef(expr)
            | ExprNodeOperation::BorrowLocal(expr, _) => expr.borrow().collect_variables(
                result_variables,
                implicit_variables,
                in_implicit_expr,
            ),
            ExprNodeOperation::WriteRef(lhs, rhs) => {
                lhs.borrow().collect_variables(
                    result_variables,
                    implicit_variables,
                    in_implicit_expr,
                );
                rhs.borrow().collect_variables(
                    result_variables,
                    implicit_variables,
                    in_implicit_expr,
                );
            }
            ExprNodeOperation::StructPack(_, args, _) => {
                for arg in args {
                    arg.1.borrow().collect_variables(
                        result_variables,
                        implicit_variables,
                        in_implicit_expr,
                    );
                }
            }
            ExprNodeOperation::StructUnpack(_, _, val, _) => val.borrow().collect_variables(
                result_variables,
                implicit_variables,
                in_implicit_expr,
            ),
            ExprNodeOperation::VariableSnapshot {
                variable, value, ..
            } => {
                implicit_variables.insert(*variable);
                value.borrow().collect_variables(
                    result_variables,
                    implicit_variables,
                    in_implicit_expr,
                );
            }
        }
    }

    pub fn has_reference_to_any_variable(&self, variables: &HashSet<usize>) -> bool {
        match self {
            ExprNodeOperation::LocalVariable(idx) => variables.contains(idx),
            ExprNodeOperation::Ignored
            | ExprNodeOperation::Deleted
            | ExprNodeOperation::NonTrivial
            | ExprNodeOperation::Raw(..)
            | ExprNodeOperation::Const(..) => false,
            ExprNodeOperation::Field(expr, _) => expr
                .borrow()
                .operation
                .has_reference_to_any_variable(variables),
            ExprNodeOperation::Unary(_, expr) => expr
                .borrow()
                .operation
                .has_reference_to_any_variable(variables),
            ExprNodeOperation::Cast(_, expr) => expr
                .borrow()
                .operation
                .has_reference_to_any_variable(variables),
            ExprNodeOperation::Binary(_, a, b) => {
                a.borrow()
                    .operation
                    .has_reference_to_any_variable(variables)
                    || b.borrow()
                        .operation
                        .has_reference_to_any_variable(variables)
            }
            ExprNodeOperation::Func(_, args, _) => args.iter().any(|arg| {
                arg.borrow()
                    .operation
                    .has_reference_to_any_variable(variables)
            }),
            ExprNodeOperation::Destroy(expr)
            | ExprNodeOperation::FreezeRef(expr)
            | ExprNodeOperation::ReadRef(expr)
            | ExprNodeOperation::BorrowLocal(expr, _) => expr
                .borrow()
                .operation
                .has_reference_to_any_variable(variables),
            ExprNodeOperation::WriteRef(lhs, rhs) => {
                lhs.borrow()
                    .operation
                    .has_reference_to_any_variable(variables)
                    || rhs
                        .borrow()
                        .operation
                        .has_reference_to_any_variable(variables)
            }
            ExprNodeOperation::StructPack(_, args, _) => args.iter().any(|arg| {
                arg.1
                    .borrow()
                    .operation
                    .has_reference_to_any_variable(variables)
            }),
            ExprNodeOperation::StructUnpack(_, _, val, _) => val
                .borrow()
                .operation
                .has_reference_to_any_variable(variables),
            ExprNodeOperation::VariableSnapshot {
                variable,
                assigment_id: _,
                value,
            } => {
                variables.contains(variable)
                    || value
                        .borrow()
                        .operation
                        .has_reference_to_any_variable(variables)
            }
        }
    }

    pub fn rename_variables(&mut self, renamed_variables: &HashMap<usize, usize>) {
        match self {
            ExprNodeOperation::LocalVariable(idx) => {
                if renamed_variables.get(idx).is_none() {
                    panic!("Variable {} not found {:?}", idx, renamed_variables);
                }
                *idx = *renamed_variables.get(idx).unwrap();
            }
            ExprNodeOperation::NonTrivial => {
                panic!("NonTrivial should not be renamed");
            }
            ExprNodeOperation::Ignored
            | ExprNodeOperation::Deleted
            | ExprNodeOperation::Raw(..)
            | ExprNodeOperation::Const(..) => {}
            ExprNodeOperation::Binary(_, a, b) | ExprNodeOperation::WriteRef(a, b) => {
                a.borrow_mut().rename_variables(renamed_variables);
                b.borrow_mut().rename_variables(renamed_variables);
            }
            ExprNodeOperation::Func(_, args, _) => {
                for arg in args {
                    arg.borrow_mut().rename_variables(renamed_variables);
                }
            }
            ExprNodeOperation::StructPack(_, args, _) => {
                for arg in args {
                    arg.1.borrow_mut().rename_variables(renamed_variables);
                }
            }
            ExprNodeOperation::StructUnpack(_, _, val, _) => {
                val.borrow_mut().rename_variables(renamed_variables)
            }
            ExprNodeOperation::Field(expr, _)
            | ExprNodeOperation::Unary(_, expr)
            | ExprNodeOperation::Cast(_, expr)
            | ExprNodeOperation::Destroy(expr)
            | ExprNodeOperation::FreezeRef(expr)
            | ExprNodeOperation::ReadRef(expr)
            | ExprNodeOperation::BorrowLocal(expr, _) => {
                expr.borrow_mut().rename_variables(renamed_variables)
            }
            ExprNodeOperation::VariableSnapshot {
                variable, value, ..
            } => {
                if renamed_variables.get(variable).is_none() {
                    panic!("Variable {} not found {:?}", variable, renamed_variables);
                }
                *variable = *renamed_variables.get(variable).unwrap();
                value.borrow_mut().rename_variables(renamed_variables);
            }
        }
    }

    fn commit_pending_variables(&self, variables: &HashSet<usize>) -> ExprNodeRef {
        match self {
            ExprNodeOperation::Ignored => self.to_node(),
            ExprNodeOperation::Deleted => self.to_node(),
            ExprNodeOperation::NonTrivial => self.to_node(),
            ExprNodeOperation::Raw(_) => self.to_node(),
            ExprNodeOperation::Const(_) => self.to_node(),
            ExprNodeOperation::LocalVariable(_) => self.to_node(),
            ExprNodeOperation::Field(expr, name) => ExprNodeOperation::Field(
                expr.borrow().commit_pending_variables(variables),
                name.clone(),
            )
            .to_node(),
            ExprNodeOperation::Unary(op, expr) => ExprNodeOperation::Unary(
                op.clone(),
                expr.borrow().commit_pending_variables(variables),
            )
            .to_node(),
            ExprNodeOperation::Cast(typ, expr) => ExprNodeOperation::Cast(
                typ.clone(),
                expr.borrow().commit_pending_variables(variables),
            )
            .to_node(),
            ExprNodeOperation::Binary(op, left, right) => ExprNodeOperation::Binary(
                op.clone(),
                left.borrow().commit_pending_variables(variables),
                right.borrow().commit_pending_variables(variables),
            )
            .to_node(),
            ExprNodeOperation::Func(name, args, typs) => ExprNodeOperation::Func(
                name.clone(),
                args.iter()
                    .map(|x| x.borrow().commit_pending_variables(variables))
                    .collect(),
                typs.clone(),
            )
            .to_node(),
            ExprNodeOperation::Destroy(expr) => {
                ExprNodeOperation::Destroy(expr.borrow().commit_pending_variables(variables))
                    .to_node()
            }
            ExprNodeOperation::FreezeRef(expr) => {
                ExprNodeOperation::FreezeRef(expr.borrow().commit_pending_variables(variables))
                    .to_node()
            }
            ExprNodeOperation::ReadRef(expr) => {
                ExprNodeOperation::ReadRef(expr.borrow().commit_pending_variables(variables))
                    .to_node()
            }
            ExprNodeOperation::BorrowLocal(expr, mutable) => ExprNodeOperation::BorrowLocal(
                expr.borrow().commit_pending_variables(variables),
                *mutable,
            )
            .to_node(),
            ExprNodeOperation::WriteRef(expr, expr2) => ExprNodeOperation::WriteRef(
                expr.borrow().commit_pending_variables(variables),
                expr2.borrow().commit_pending_variables(variables),
            )
            .to_node(),
            ExprNodeOperation::StructPack(name, fields, typs) => ExprNodeOperation::StructPack(
                name.clone(),
                fields
                    .iter()
                    .map(|x| {
                        (
                            x.0.clone(),
                            x.1.borrow().commit_pending_variables(variables),
                        )
                    })
                    .collect(),
                typs.clone(),
            )
            .to_node(),
            ExprNodeOperation::StructUnpack(name, fields_names, expr, typs) => {
                ExprNodeOperation::StructUnpack(
                    name.clone(),
                    fields_names.clone(),
                    expr.borrow().commit_pending_variables(variables),
                    typs.clone(),
                )
                .to_node()
            }
            ExprNodeOperation::VariableSnapshot {
                variable,
                assigment_id,
                value,
            } => {
                if variables.contains(variable) {
                    ExprNodeOperation::LocalVariable(*variable).to_node()
                } else {
                    ExprNodeOperation::VariableSnapshot {
                        variable: *variable,
                        assigment_id: *assigment_id,
                        value: value.borrow().commit_pending_variables(variables),
                    }
                    .to_node()
                }
            }
        }
    }
}

fn get_precedence(operator: &str) -> u32 {
    match operator {
        // spec
        // "==>" => 1,
        // ":=" => 3,
        "||" => 5,
        "&&" => 10,

        "==" | "!=" => 15,

        "<" | ">" | "<=" | ">=" => 15,
        ".." => 20,
        "|" => 25,
        "^" => 30,
        "&" => 35,
        "<<" | ">>" => 40,
        "+" | "-" => 45,
        "*" | "/" | "%" => 50,

        _ => 0, // anything else is not a binary operator
    }
}

fn check_bracket_for_binary(
    expr: &ExprNodeRef,
    parent_precedence: u32,
    naming: Option<&Naming>,
    ctx: &ToSourceCtx,
) -> Result<String, anyhow::Error> {
    effective_operation(&[expr], &mut |&[expr]| {
        let expr_str = if let Some(naming) = naming {
            expr.borrow().to_source_with_ctx(naming, ctx)?
        } else {
            expr.borrow().to_string()
        };
        let inner_precedence = match &expr.borrow().operation {
            ExprNodeOperation::Binary(op, _, _) => get_precedence(op),
            ExprNodeOperation::Cast(..) => 3,
            _ => 1000,
        };
        Ok(if inner_precedence < parent_precedence {
            format!("({})", expr_str)
        } else {
            expr_str
        })
    })
}

fn bracket_if_binary_with_ctx(
    expr: &ExprNodeRef,
    naming: Option<&Naming>,
    ctx: &ToSourceCtx,
) -> Result<String, anyhow::Error> {
    effective_operation(&[expr], &mut |&[expr]| {
        let expr_str = if let Some(naming) = naming {
            expr.borrow().to_source_with_ctx(naming, ctx)?
        } else {
            expr.borrow().to_string()
        };
        Ok(match &expr.borrow().operation {
            ExprNodeOperation::Binary(..) => format!("({})", expr_str),
            ExprNodeOperation::Cast(..) => format!("({})", expr_str),
            _ => expr_str,
        })
    })
}

impl Display for ExprNodeOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExprNodeOperation::Deleted => write!(f, "<<< !!! deleted !!! >>>"),
            ExprNodeOperation::Ignored => write!(f, "_"),
            ExprNodeOperation::NonTrivial => write!(f, "!!non-trivial!!"),
            ExprNodeOperation::Raw(s) => write!(f, "((/*raw:*/{}))", s),
            ExprNodeOperation::Const(c) => write!(f, "{}", c),
            ExprNodeOperation::LocalVariable(idx) => write!(f, "_$local$_{}", idx),
            ExprNodeOperation::Unary(op, expr) => {
                write!(
                    f,
                    "{}{}",
                    op,
                    bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap()
                )
            }
            ExprNodeOperation::Cast(op, expr) => {
                write!(
                    f,
                    "{} as {}",
                    bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap(),
                    op
                )
            }
            ExprNodeOperation::BorrowLocal(expr, mutable) => {
                if *mutable {
                    write!(
                        f,
                        "&mut {}",
                        bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap()
                    )
                } else {
                    write!(
                        f,
                        "&{}",
                        bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap()
                    )
                }
            }
            ExprNodeOperation::Binary(op, a, b) => {
                let a_str =
                    check_bracket_for_binary(a, get_precedence(op), None, &ToSourceCtx::default())
                        .unwrap();
                let b_str =
                    check_bracket_for_binary(b, get_precedence(op), None, &ToSourceCtx::default())
                        .unwrap();
                write!(f, "{} {} {}", a_str, op, b_str)
            }
            // freezeref convert &mut to &, that typing is at variable declaration level so just ignore
            ExprNodeOperation::FreezeRef(expr) => write!(f, "{}", expr.borrow()),
            ExprNodeOperation::ReadRef(expr) => {
                write!(
                    f,
                    "*{}",
                    bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap()
                )
            }
            ExprNodeOperation::WriteRef(lhs, rhs) => {
                write!(
                    f,
                    "*{} = {}",
                    bracket_if_binary_with_ctx(lhs, None, &ToSourceCtx::default()).unwrap(),
                    rhs.borrow()
                )
            }
            ExprNodeOperation::Destroy(expr) => write!(f, "/*destroyed:{}*/", expr.borrow()),
            ExprNodeOperation::Field(expr, name) => {
                write!(
                    f,
                    "{}.{}",
                    bracket_if_binary_with_ctx(expr, None, &ToSourceCtx::default()).unwrap(),
                    name
                )
            }
            ExprNodeOperation::Func(name, args, typs) => {
                write!(
                    f,
                    "{}{}({})",
                    name,
                    if typs.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "<{}>",
                            typs.iter()
                                .map(|x| format!("{:?}", x))
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    },
                    args.iter()
                        .map(|x| x.borrow().to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            ExprNodeOperation::StructPack(name, args, types) => {
                write!(
                    f,
                    "{}{}{{{}}}",
                    name,
                    if types.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "<{}>",
                            types
                                .iter()
                                .map(|x| format!("{:?}", x))
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    },
                    args.iter()
                        .map(|x| format!("{}: {}", x.0, x.1.borrow().to_string()))
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            ExprNodeOperation::StructUnpack(name, keys, val, types) => {
                write!(
                    f,
                    "{}{}{{{}}} = {}",
                    name,
                    if types.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "<{}>",
                            types
                                .iter()
                                .map(|x| format!("{:?}", x))
                                .collect::<Vec<String>>()
                                .join(", ")
                        )
                    },
                    keys.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>()
                        .join(", "),
                    val.borrow().to_string()
                )
            }
            ExprNodeOperation::VariableSnapshot {
                variable,
                assigment_id,
                value,
            } => {
                write!(
                    f,
                    "/*snapshot:{}:{}*/{}",
                    variable,
                    assigment_id,
                    value.borrow().to_string()
                )
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ExprNode {
    pub(crate) operation: ExprNodeOperation,
}

impl ExprNode {
    pub fn rename_variables(&mut self, renamed_variables: &HashMap<usize, usize>) {
        self.operation.rename_variables(renamed_variables);
    }
    pub fn copy_as_ref(&self) -> ExprNodeRef {
        Rc::new(RefCell::new(Self {
            operation: self.operation.copy(),
        }))
    }

    pub fn to_source(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        self.operation.to_source(naming)
    }

    fn to_source_with_ctx(
        &self,
        naming: &Naming,
        ctx: &ToSourceCtx,
    ) -> Result<String, anyhow::Error> {
        self.operation.to_source_with_ctx(naming, ctx)
    }

    pub fn to_source_decl(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        self.operation.to_source_decl(naming)
    }

    pub fn collect_variables(
        &self,
        result_variables: &mut HashSet<usize>,
        implicit_variables: &mut HashSet<usize>,
        in_implicit_expr: bool,
    ) {
        self.operation
            .collect_variables(result_variables, implicit_variables, in_implicit_expr);
    }

    pub fn commit_pending_variables(&self, variables: &HashSet<usize>) -> ExprNodeRef {
        self.operation.commit_pending_variables(variables)
    }
}

impl Display for ExprNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.operation.fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    node: ExprNodeRef,
}

pub struct VariablesInfo {
    pub variables: HashSet<usize>,
    pub implicit_variables: HashSet<usize>,
}
impl VariablesInfo {
    fn any_variables(&self) -> HashSet<usize> {
        self.variables
            .union(&self.implicit_variables)
            .map(|x| *x)
            .collect()
    }
}

impl Expr {
    pub fn new(node: ExprNodeRef) -> Self {
        Self { node }
    }

    pub fn rename_variables(&mut self, renamed_variables: &HashMap<usize, usize>) {
        self.node.borrow_mut().rename_variables(renamed_variables);
    }

    pub fn non_trivial() -> Self {
        Self {
            node: ExprNodeOperation::NonTrivial.to_node(),
        }
    }

    pub fn is_non_trivial(&self) -> bool {
        match &self.node.borrow().operation {
            ExprNodeOperation::NonTrivial => true,
            _ => false,
        }
    }

    pub fn is_flushed(&self) -> bool {
        match &self.node.borrow().operation {
            ExprNodeOperation::Raw(..) => true,
            ExprNodeOperation::LocalVariable(..) => true,
            _ => false,
        }
    }

    pub fn copy(&self) -> Self {
        Self {
            node: self.value_copied(),
        }
    }

    pub fn value(&self) -> &ExprNodeRef {
        &self.node
    }

    pub fn value_copied(&self) -> ExprNodeRef {
        self.node.borrow().copy_as_ref()
    }

    fn ignored() -> Expr {
        Expr::new(ExprNodeOperation::Ignored.to_node())
    }

    #[allow(dead_code)]
    fn deleted() -> Expr {
        Expr::new(ExprNodeOperation::Deleted.to_node())
    }

    pub fn to_source(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        self.node.borrow().to_source(naming)
    }

    pub fn to_source_decl(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        self.node.borrow().to_source_decl(naming)
    }

    pub fn commit_pending_variables(&self, variables: &HashSet<usize>) -> Expr {
        Expr::new(self.node.borrow().commit_pending_variables(variables))
    }

    pub(crate) fn should_ignore(&self) -> bool {
        match &self.node.borrow().operation {
            ExprNodeOperation::Destroy(..) => true,
            _ => false,
        }
    }

    pub(crate) fn collect_variables(&self, in_implicit_expr: bool) -> VariablesInfo {
        let mut result_variables = HashSet::new();
        let mut implicit_variables = HashSet::new();
        self.node.borrow().collect_variables(
            &mut result_variables,
            &mut implicit_variables,
            in_implicit_expr,
        );
        VariablesInfo {
            variables: result_variables,
            implicit_variables,
        }
    }

    pub(crate) fn is_single_variable(&self) -> Option<usize> {
        match &self.node.borrow().operation {
            ExprNodeOperation::LocalVariable(idx) => Some(*idx),
            _ => None,
        }
    }

    pub(crate) fn has_reference_to_any_variable(&self, variables: &HashSet<usize>) -> bool {
        self.node
            .borrow()
            .operation
            .has_reference_to_any_variable(variables)
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.node.borrow().fmt(f)
    }
}

#[derive(Clone, Debug)]
struct VariableValueSnapshot {
    value: Expr,
    defined_in_context: Option<usize>,
    assigment_idx: usize,
}

#[derive(Debug)]
pub struct StacklessEvaluationContext<'a> {
    context_id: usize,
    assignment_id_provider: Rc<RefCell<usize>>,
    variables: HashMap<usize, VariableValueSnapshot>,
    pending_variables: HashMap<usize, VariableValueSnapshot>,
    finalized_pending_variables: HashSet<usize>,
    func_env: &'a FunctionEnv<'a>,
    last_branch_expr: Option<Expr>,
    loop_entry: bool,
}

impl<'a> Clone for StacklessEvaluationContext<'a> {
    fn clone(&self) -> Self {
        let next_id = self.next_assignment_id();
        Self {
            context_id: next_id,
            assignment_id_provider: self.assignment_id_provider.clone(),
            variables: self.variables.clone(),
            pending_variables: self.pending_variables.clone(),
            finalized_pending_variables: self.finalized_pending_variables.clone(),
            func_env: self.func_env,
            last_branch_expr: self.last_branch_expr.clone(),
            // this property is not cloned
            loop_entry: false,
        }
    }
}

pub mod operations;

use operations::*;

use super::super::naming::Naming;

#[derive(Clone, Debug)]
pub struct ReturnValueHint {
    pub ty: move_model::ty::Type,
}

pub struct StacklessEvaluationRunResult {
    pub results: Expr,
    pub new_variables: HashSet<usize>,
    pub flushed_variables: HashSet<usize>,
    pub cannot_keep_as_expr: bool,
}

impl<'a> StacklessEvaluationContext<'a> {
    pub fn new(func_env: &'a FunctionEnv<'a>) -> Self {
        Self {
            context_id: 1,
            variables: HashMap::new(),
            pending_variables: HashMap::new(),
            finalized_pending_variables: HashSet::new(),
            assignment_id_provider: Rc::new(RefCell::new(0)),
            func_env,
            last_branch_expr: None,
            loop_entry: false,
        }
    }

    pub fn shortest_prefix(&self, mod_id: &ModuleId) -> String {
        super::super::utils::shortest_prefix(&self.func_env.module_env, mod_id)
    }

    pub fn defined(&self, idx: usize) -> bool {
        self.variables.contains_key(&idx)
    }

    pub fn defined_or_pending(&self, idx: usize) -> bool {
        self.variables.contains_key(&idx) || self.pending_variables.contains_key(&idx)
    }

    fn get_pending_var(&self, idx: usize) -> Option<&VariableValueSnapshot> {
        if let Some(pending) = self.pending_variables.get(&idx) {
            Some(pending)
        } else {
            None
        }
    }

    pub fn get_var_with_allow_undefined(&self, idx: usize, allow_undefined: bool) -> Expr {
        if !self.is_flushed(idx) {
            if let Some(VariableValueSnapshot {
                value,
                assigment_idx,
                ..
            }) = self.get_pending_var(idx)
            {
                return ExprNodeOperation::VariableSnapshot {
                    variable: idx,
                    assigment_id: *assigment_idx,
                    value: value.copy().node,
                }
                .to_expr();
            }
        }

        if let Some(value) = self.variables.get(&idx) {
            value.value.copy()
        } else {
            if allow_undefined {
                ExprNodeOperation::LocalVariable(idx).to_expr()
            } else {
                panic!("Variable {} not defined", idx)
            }
        }
    }
    pub fn get_var(&self, idx: usize) -> Expr {
        self.get_var_with_allow_undefined(idx, false)
    }

    fn next_assignment_id(&self) -> usize {
        let mut id = self.assignment_id_provider.borrow_mut();
        *id = id.wrapping_add(1).max(1);
        *id
    }

    fn run_assignment(&mut self, idx: usize, value: Expr) -> bool {
        let id = self.next_assignment_id();
        let is_new_variable = !self.variables.contains_key(&idx);
        self.variables.insert(
            idx,
            VariableValueSnapshot {
                value,
                defined_in_context: Some(self.context_id),
                assigment_idx: id,
            },
        );
        is_new_variable
    }

    pub fn push_branch_condition(&mut self, e: Expr) -> Result<(), anyhow::Error> {
        if self.last_branch_expr.is_some() {
            return Err(anyhow::anyhow!("Branch condition already pushed"));
        }
        self.last_branch_expr = Some(e);
        Ok(())
    }

    pub fn pop_branch_condition(&mut self) -> Option<Expr> {
        let expr = self.last_branch_expr.clone();
        self.last_branch_expr = None;
        expr
    }

    pub fn run(
        &mut self,
        bytecode: &Bytecode,
        dst_types: &Vec<Option<ReturnValueHint>>,
    ) -> Result<StacklessEvaluationRunResult, anyhow::Error> {
        if self.last_branch_expr.is_some() {
            return Err(anyhow::anyhow!(
                "Branch should be handled before running next bytecode"
            ));
        }
        let mut flushed_variables = HashSet::new();
        let mut new_variables = HashSet::new();
        match bytecode {
            Bytecode::Call(_, dsts, oper, srcs, _abort_action) => {
                for &dst in dsts {
                    if self.defined(dst) && self.get_var(dst).is_flushed() {
                        flushed_variables.insert(dst);
                    }
                }
                let allow_undefined = matches!(
                    oper,
                    move_stackless_bytecode::stackless_bytecode::Operation::Drop
                    | move_stackless_bytecode::stackless_bytecode::Operation::Release
                );
                let OperationEvaluatorResult {
                    expr: results,
                    mut cannot_keep,
                } = oper.evaluate(
                    self,
                    &srcs
                        .iter()
                        .map(|x| {
                            if !self.defined(*x) {
                                ExprNodeOperation::Raw(format!("/*undefined:{}*/undefined", x))
                                    .to_expr()
                            } else {
                                self.get_var_with_allow_undefined(*x, allow_undefined)
                            }
                        })
                        .collect(),
                    dst_types,
                )?;

                let mut handled = false;
                match &results.node.borrow().operation {
                    ExprNodeOperation::Destroy(..) => {
                        if dsts.len() != 0 {
                            return Err(anyhow::anyhow!("Expected zero return value for destroy"));
                        }
                        handled = true;
                    }
                    ExprNodeOperation::ReadRef(..) => {}
                    ExprNodeOperation::WriteRef(_wdst, _wsrc) => {
                        if dsts.len() != 0 {
                            return Err(anyhow::anyhow!(
                                "Expected zero return value for write_ref"
                            ));
                        }
                        // FIXME: should we inc write for wdst?
                        handled = true;
                    }
                    ExprNodeOperation::StructUnpack(_name, keys, _value, _types) => {
                        // special case - unpack to no variable
                        if dsts.len() == 0 {
                            handled = true;
                        } else {
                            if dsts.len() != keys.len() {
                                return Err(anyhow::anyhow!("Unmatched struct unpack"));
                            };
                            for dst in dsts {
                                if self.run_assignment(*dst, Expr::non_trivial()) {
                                    new_variables.insert(*dst);
                                }
                            }
                            handled = true;
                        }
                    }
                    _ => {}
                }

                if !handled {
                    if dsts.len() == 1 {
                        if self.run_assignment(dsts[0], results.copy()) {
                            new_variables.insert(dsts[0]);
                        }
                    } else {
                        for dst in dsts {
                            if self.run_assignment(*dst, Expr::non_trivial()) {
                                new_variables.insert(*dst);
                            }
                        }
                    }
                }

                if cannot_keep == false
                    && results
                        .collect_variables(false)
                        .any_variables()
                        .intersection(&HashSet::from_iter(dsts.iter().cloned()))
                        .next()
                        .is_some()
                {
                    cannot_keep = true;
                }

                Ok(StacklessEvaluationRunResult {
                    results,
                    new_variables,
                    flushed_variables,
                    cannot_keep_as_expr: cannot_keep,
                })
            }
            Bytecode::Assign(_, dst, src, kind) => {
                let dst = *dst;
                if self.defined(dst) && self.get_var(dst).is_flushed() {
                    flushed_variables.insert(dst);
                }
                let result = self.get_var(*src);
                if self.run_assignment(dst, result.copy()) {
                    new_variables.insert(dst);
                }
                match kind {
                    AssignKind::Copy => {}
                    AssignKind::Move => {
                        new_variables.insert(dst);
                        // value of src may be still referenced by other variables, the ownership already checked at compiler time so just ignore
                        // self.run_assignment(*src, Expr::deleted());
                    }
                    AssignKind::Store => {
                        // TODO: this is still a TODO in stackless bytecode too
                        // this assign is due to a COPY/MOVE pushed to the stack and poped
                        // it's seems that we dont need to do anything here
                    }
                    AssignKind::Inferred => {}
                };

                let cannot_keep = result
                    .collect_variables(false)
                    .any_variables()
                    .contains(&dst);

                Ok(StacklessEvaluationRunResult {
                    results: result,
                    new_variables,
                    flushed_variables,
                    cannot_keep_as_expr: cannot_keep,
                })
            }
            Bytecode::Load(_, dst, value) => {
                let dst = *dst;
                if self.defined(dst) && self.get_var(dst).is_flushed() {
                    flushed_variables.insert(dst);
                }
                let expr = ExprNodeOperation::Const(value.clone()).to_expr();
                if self.run_assignment(dst, expr.copy()) {
                    new_variables.insert(dst);
                }
                Ok(StacklessEvaluationRunResult {
                    results: expr,
                    new_variables,
                    flushed_variables,
                    cannot_keep_as_expr: false,
                })
            }
            Bytecode::Nop(..)
            | Bytecode::Ret(..)
            | Bytecode::Branch(..)
            | Bytecode::Jump(..)
            | Bytecode::Label(..)
            | Bytecode::Abort(..) => Ok(StacklessEvaluationRunResult {
                results: Expr::ignored(),
                new_variables,
                flushed_variables,
                cannot_keep_as_expr: false,
            }),
            Bytecode::Prop(..) | Bytecode::SaveMem(..) | Bytecode::SaveSpecVar(..) => {
                unreachable!()
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn flush_value(&mut self, dst: usize, name: String, is_new: bool) {
        self.variables.insert(
            dst,
            VariableValueSnapshot {
                value: ExprNodeOperation::Raw(name.clone()).to_expr(),
                defined_in_context: if is_new { Some(self.context_id) } else { None },
                assigment_idx: usize::MAX,
            },
        );
    }

    pub(crate) fn flush_local_value(&mut self, dst: usize, is_new: Option<bool>) {
        let is_new = is_new.unwrap_or(!self.variables.contains_key(&dst));
        self.variables.insert(
            dst,
            VariableValueSnapshot {
                value: ExprNodeOperation::LocalVariable(dst).to_expr(),
                defined_in_context: if is_new { Some(self.context_id) } else { None },
                assigment_idx: usize::MAX,
            },
        );
    }

    fn is_flushed(&self, dst: usize) -> bool {
        if let Some(VariableValueSnapshot {
            assigment_idx: aid, ..
        }) = self.variables.get(&dst)
        {
            *aid == usize::MAX
        } else {
            false
        }
    }

    pub(crate) fn flush_pending_local_value(
        &mut self,
        dst: usize,
        is_new: Option<bool>,
        value: Expr,
    ) -> usize {
        if !self.defined(dst) {
            panic!("Invariant Exception: Variable {} not defined", dst);
        }
        if self.is_flushed(dst) {
            panic!("Invariant Exception: Variable {} already defined", dst);
        }
        let is_new = is_new.unwrap_or(!self.pending_variables.contains_key(&dst));
        let id = self.next_assignment_id();
        self.pending_variables.insert(
            dst,
            VariableValueSnapshot {
                value,
                defined_in_context: if is_new { Some(self.context_id) } else { None },
                assigment_idx: id,
            },
        );
        id
    }

    /// Assume that branches are starting from current context, merge them and return the variables that need to be flushed
    /// Currently not handling ignored and deleted variables, just consider these actions as an assignment
    pub(crate) fn merge_branches(
        &mut self,
        branches: &Vec<&StacklessEvaluationContext<'_>>,
        _self_not_in_tail: bool,
    ) -> Vec<usize> {
        let mut need_flushes = HashSet::new();

        for branch in branches {
            for (
                var_id,
                VariableValueSnapshot {
                    defined_in_context: br_context_definition_id,
                    assigment_idx: br_aid,
                    ..
                },
            ) in branch.variables.iter()
            {
                if let Some(VariableValueSnapshot {
                    assigment_idx: aid,
                    defined_in_context: current_context_definition_id,
                    ..
                }) = self.variables.get(var_id)
                {
                    if *aid != *br_aid
                        || *current_context_definition_id != *br_context_definition_id
                    {
                        need_flushes.insert(*var_id);
                    }
                } else {
                    need_flushes.insert(*var_id);
                }
            }
        }

        // for pending variables, we just ignore any that has conflict
        let mut pending_variables_to_remove = HashSet::new();
        for branch in branches {
            for (var_id, var_value) in branch.pending_variables.iter() {
                if let Some(self_var_value) = self.pending_variables.get(var_id) {
                    let conflict = var_value.assigment_idx != self_var_value.assigment_idx;
                    if conflict {
                        pending_variables_to_remove.insert(*var_id);
                    }
                }
            }
            self.finalized_pending_variables
                .extend(branch.finalized_pending_variables.iter());
        }

        for var_id in pending_variables_to_remove {
            self.pending_variables.remove(&var_id);
            self.finalized_pending_variables.insert(var_id);
        }

        need_flushes.into_iter().collect()
    }

    #[allow(dead_code)]
    pub(crate) fn get_vars(&self) -> HashSet<usize> {
        self.variables.keys().cloned().collect()
    }

    pub(crate) fn enter_loop(&mut self) {
        self.loop_entry = true;
    }
}
