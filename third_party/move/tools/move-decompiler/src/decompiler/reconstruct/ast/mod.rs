// Copyright (c) Verichains
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use crate::decompiler::evaluator::stackless::{ExprNodeOperation, ExprNodeRef};

use super::super::naming::Naming;

use super::{super::evaluator::stackless::Expr, code_unit::SourceCodeUnit};

pub mod optimizers;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DecompiledExpr {
    Undefined,
    EvaluationExpr(Expr),
    #[allow(dead_code)]
    Variable(usize),
    Tuple(Vec<DecompiledExprRef>),
}

pub(crate) type DecompiledExprRef = Box<DecompiledExpr>;

impl DecompiledExpr {
    pub fn boxed(self: Self) -> DecompiledExprRef {
        Box::new(self)
    }

    pub fn copy_as_ref(&self) -> DecompiledExprRef {
        match self {
            DecompiledExpr::Undefined => DecompiledExpr::Undefined.boxed(),

            DecompiledExpr::EvaluationExpr(expr) => {
                DecompiledExpr::EvaluationExpr(expr.copy()).boxed()
            }

            DecompiledExpr::Variable(var) => DecompiledExpr::Variable(*var).boxed(),

            DecompiledExpr::Tuple(exprs) => {
                DecompiledExpr::Tuple(exprs.iter().map(|e| e.copy_as_ref()).collect()).boxed()
            }
        }
    }

    pub fn commit_pending_variables(
        &self,
        selected_variables: &HashSet<usize>,
    ) -> DecompiledExprRef {
        match self {
            DecompiledExpr::Undefined => DecompiledExpr::Undefined.boxed(),

            DecompiledExpr::EvaluationExpr(expr) => {
                DecompiledExpr::EvaluationExpr(expr.commit_pending_variables(selected_variables))
                    .boxed()
            }

            DecompiledExpr::Variable(var) => DecompiledExpr::Variable(*var).boxed(),

            DecompiledExpr::Tuple(exprs) => DecompiledExpr::Tuple(
                exprs
                    .iter()
                    .map(|e| e.commit_pending_variables(selected_variables))
                    .collect(),
            )
            .boxed(),
        }
    }

    pub fn is_single_or_tuple_variable_expr(&self) -> Option<Vec<usize>> {
        match self {
            DecompiledExpr::Tuple(exprs) => {
                exprs.iter().map(|e| e.is_single_variable_expr()).collect()
            }

            DecompiledExpr::EvaluationExpr(e) => e.is_single_variable().map(|v| vec![v]),

            DecompiledExpr::Variable(var) => Some(vec![*var]),

            _ => None,
        }
    }

    pub fn is_single_variable_expr(&self) -> Option<usize> {
        let vars = self.is_single_or_tuple_variable_expr()?;

        if vars.len() == 1 {
            Some(vars[0])
        } else {
            None
        }
    }

    pub fn has_reference_to_any_variable(&self, variables: &HashSet<usize>) -> bool {
        match self {
            DecompiledExpr::Undefined => false,

            DecompiledExpr::EvaluationExpr(expr) => expr.has_reference_to_any_variable(variables),

            DecompiledExpr::Variable(var) => variables.contains(var),

            DecompiledExpr::Tuple(exprs) => exprs
                .iter()
                .any(|e| e.has_reference_to_any_variable(variables)),
        }
    }

    pub fn rename_variables(&mut self, renamed_variables: &HashMap<usize, usize>) {
        match self {
            DecompiledExpr::Undefined => {}

            DecompiledExpr::EvaluationExpr(expr) => {
                expr.rename_variables(renamed_variables);
            }

            DecompiledExpr::Variable(var) => {
                *var = renamed_variables[var];
            }

            DecompiledExpr::Tuple(exprs) => {
                for expr in exprs {
                    expr.rename_variables(renamed_variables);
                }
            }
        }
    }

    pub fn collect_variables(
        &self,
        result_variables: &mut HashSet<usize>,
        implicit_variables: &mut HashSet<usize>,
        in_implicit_expr: bool,
    ) {
        match &self {
            DecompiledExpr::Undefined => {}

            DecompiledExpr::EvaluationExpr(expr) => {
                let v = expr.collect_variables(in_implicit_expr);
                result_variables.extend(v.variables);
                implicit_variables.extend(v.implicit_variables);
            }

            DecompiledExpr::Variable(var) => {
                if in_implicit_expr {
                    implicit_variables.insert(*var);
                } else {
                    result_variables.insert(*var);
                }
            }

            DecompiledExpr::Tuple(exprs) => {
                exprs.iter().for_each(|expr| {
                    expr.collect_variables(result_variables, implicit_variables, in_implicit_expr)
                });
            }
        }
    }

    pub fn is_empty_tuple(&self) -> bool {
        match self {
            DecompiledExpr::Tuple(exprs) => exprs.is_empty(),

            _ => false,
        }
    }

    pub fn to_expr(&self) -> Result<ExprNodeRef, anyhow::Error> {
        match self {
            DecompiledExpr::Undefined => {
                Ok(ExprNodeOperation::Raw("undefined".to_string()).to_node())
            }

            DecompiledExpr::EvaluationExpr(expr) => Ok(expr.value_copied()),

            DecompiledExpr::Variable(var) => Ok(ExprNodeOperation::LocalVariable(*var).to_node()),

            DecompiledExpr::Tuple(exprs) => {
                if exprs.len() == 1 {
                    exprs[0].to_expr()
                } else {
                    Err(anyhow::anyhow!("Cannot convert tuple to expr"))
                }
            }
        }
    }

    pub fn to_source_decl(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        match self {
            DecompiledExpr::EvaluationExpr(expr) => Ok(expr.to_source_decl(naming)?),

            _ => self.to_source(naming),
        }
    }

    pub fn to_source(&self, naming: &Naming) -> Result<String, anyhow::Error> {
        match self {
            DecompiledExpr::Undefined => Ok("undefined".to_string()),

            DecompiledExpr::EvaluationExpr(expr) => Ok(expr.to_source(naming)?),

            DecompiledExpr::Variable(var) => Ok(naming.variable(*var)),

            DecompiledExpr::Tuple(exprs) => {
                if exprs.len() == 1 {
                    exprs[0].to_source(naming)
                } else {
                    Ok(format!(
                        "({})",
                        exprs
                            .iter()
                            .map(|e| e.to_source(naming))
                            .collect::<Result<Vec<_>, _>>()?
                            .join(", ")
                    ))
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ResultUsageType {
    None,
    Return,
    Abort,
    BlockResult,
}

#[derive(Debug, Clone)]
pub(crate) enum DecompiledCodeItem {
    ReturnStatement(DecompiledExprRef),
    AbortStatement(DecompiledExprRef),
    BreakStatement,
    ContinueStatement,
    CommentStatement(String),
    PossibleAssignStatement {
        #[allow(dead_code)]
        assigment_id: usize,
        variable: usize,
        value: DecompiledExprRef,
        is_decl: bool,
    },
    PreDeclareStatement {
        variable: usize,
    },
    AssignStatement {
        variable: usize,
        value: DecompiledExprRef,
        is_decl: bool,
    },
    AssignTupleStatement {
        variables: Vec<usize>,
        value: DecompiledExprRef,
        is_decl: bool,
    },
    AssignStructureStatement {
        structure_visible_name: String,
        variables: Vec<(String, usize)>,
        value: DecompiledExprRef,
    },
    Statement {
        expr: DecompiledExprRef,
    },
    IfElseStatement {
        cond: DecompiledExprRef,
        if_unit: DecompiledCodeUnitRef,
        else_unit: DecompiledCodeUnitRef,
        result_variables: Vec<usize>,
        /* this if statement is used as return value */
        use_as_result: ResultUsageType,
    },
    WhileStatement {
        cond: Option<DecompiledExprRef>,
        body: DecompiledCodeUnitRef,
    },
}

pub(crate) type DecompiledCodeUnitRef = Box<DecompiledCodeUnit>;

#[derive(Debug, Clone)]
pub(crate) struct DecompiledCodeUnit {
    blocks: Vec<DecompiledCodeItem>,
    exit: Option<DecompiledExprRef>,
    // sorted by variable index
    result_variables: Vec<usize>,
}

impl DecompiledCodeUnit {
    pub fn new() -> DecompiledCodeUnitRef {
        Box::new(DecompiledCodeUnit {
            blocks: Vec::new(),
            exit: None,
            result_variables: Vec::new(),
        })
    }

    pub fn extends(&mut self, other: DecompiledCodeUnitRef) -> Result<(), anyhow::Error> {
        self.extends_main(other, true)
    }

    pub fn extends_main(
        &mut self,
        other: DecompiledCodeUnitRef,
        copy_result_variables: bool,
    ) -> Result<(), anyhow::Error> {
        if self.exit.is_some() {
            return Err(anyhow::anyhow!("Cannot extend terminated code unit"));
        }

        self.blocks.extend(other.blocks);

        if other.exit.is_some() {
            self.exit = other.exit;
            if copy_result_variables {
                self.result_variables = other.result_variables;
            }
        }

        Ok(())
    }

    pub fn add(&mut self, item: DecompiledCodeItem) {
        self.blocks.push(item);
    }

    pub fn exit(
        &mut self,
        variables: Vec<usize>,
        expr: DecompiledExprRef,
        must_be_preempty: bool,
    ) -> Result<(), anyhow::Error> {
        if let Some(current) = &self.exit {
            if must_be_preempty && current != &expr {
                return Err(anyhow::anyhow!("Cannot set exit expr twice"));
            }
        }

        self.result_variables = variables;
        self.exit = Some(expr);

        Ok(())
    }

    pub fn has_reference_to_any_variable(&self, variables: &HashSet<usize>) -> bool {
        for block in &self.blocks {
            match block {
                DecompiledCodeItem::PossibleAssignStatement {
                    variable, value, ..
                } => {
                    if variables.contains(variable)
                        || value.has_reference_to_any_variable(variables)
                    {
                        return true;
                    }
                }

                DecompiledCodeItem::PreDeclareStatement { variable } => {
                    if variables.contains(variable) {
                        return true;
                    }
                }

                DecompiledCodeItem::ReturnStatement(expr)
                | DecompiledCodeItem::AbortStatement(expr)
                | DecompiledCodeItem::Statement { expr } => {
                    if expr.has_reference_to_any_variable(variables) {
                        return true;
                    }
                }

                DecompiledCodeItem::BreakStatement
                | DecompiledCodeItem::ContinueStatement
                | DecompiledCodeItem::CommentStatement(_) => {}
                DecompiledCodeItem::AssignStatement {
                    variable, value, ..
                } => {
                    if variables.contains(variable)
                        || value.has_reference_to_any_variable(variables) {
                        return true;
                    }
                }

                DecompiledCodeItem::AssignTupleStatement {
                    variables: vs,
                    value,
                    ..
                } => {
                    if vs.iter().any(|v| variables.contains(v))
                        || value.has_reference_to_any_variable(variables)
                    {
                        return true;
                    }
                }

                DecompiledCodeItem::AssignStructureStatement {
                    variables: vs,
                    value,
                    ..
                } => {
                    if vs.iter().any(|(_, v)| variables.contains(v))
                        || value.has_reference_to_any_variable(variables)
                    {
                        return true;
                    }
                }

                DecompiledCodeItem::IfElseStatement {
                    cond,
                    if_unit,
                    else_unit,
                    ..
                } => {
                    if cond.has_reference_to_any_variable(variables)
                        || if_unit.has_reference_to_any_variable(variables)
                        || else_unit.has_reference_to_any_variable(variables)
                    {
                        return true;
                    }
                }

                DecompiledCodeItem::WhileStatement { cond, body } => {
                    if cond
                        .as_ref()
                        .map(|x| x.has_reference_to_any_variable(variables))
                        .unwrap_or(false)
                        || body.has_reference_to_any_variable(variables) {
                        return true;
                    }
                }
            }
        }

        false
    }

    pub fn to_source(
        &self,
        naming: &Naming,
        root_block: bool,
    ) -> Result<SourceCodeUnit, anyhow::Error> {
        let mut source = SourceCodeUnit::new(0);
        let mut iter = self.blocks.iter().peekable();

        while let Some(item) = iter.next() {
            let can_obmit_return = root_block && iter.peek().is_none() && self.exit.is_none();
            match item {
                DecompiledCodeItem::PreDeclareStatement { variable } => {
                    source.add_line(format!("let {};", naming.variable(*variable)));
                }

                DecompiledCodeItem::PossibleAssignStatement {
                    variable,
                    value,
                    is_decl,
                    ..
                } => {
                    // if debug
                    if !cfg!(debug_assertions) {
                        panic!("Invariant Exception: PossibleAssignStatement is not meant to be used in final source code generation")
                    }
                    if *is_decl {
                        to_decl_source(
                            &mut source,
                            format!("// possible: let {} = ", naming.variable(*variable)).as_str(),
                            ";",
                            value,
                            naming,
                        )?;
                    } else {
                        source.add_line(format!(
                            "// possible: {} = {};",
                            naming.variable(*variable),
                            value.to_source(naming)?
                        ));
                    }
                }

                DecompiledCodeItem::ReturnStatement(expr) => {
                    if root_block && can_obmit_return {
                        if !expr.is_empty_tuple() {
                            to_decl_source(&mut source, "", "", expr, naming)?;
                        }
                    } else {
                        if expr.is_empty_tuple() {
                            source.add_line(format!("return"));
                        } else {
                            to_decl_source(&mut source, "return ", "", expr, naming)?;
                        }
                    }
                }

                DecompiledCodeItem::AbortStatement(expr) => {
                    to_decl_source(
                        &mut source,
                        "abort ",
                        if iter.peek().is_none() { "" } else { ";" },
                        expr,
                        naming,
                    )?;
                }

                DecompiledCodeItem::BreakStatement => {
                    if iter.peek().is_none() {
                        source.add_line(format!("break"));
                    } else {
                        source.add_line(format!("break;"));
                    }
                }

                DecompiledCodeItem::ContinueStatement => {
                    if iter.peek().is_none() {
                        source.add_line(format!("continue"));
                    } else {
                        source.add_line(format!("continue;"));
                    }
                }

                DecompiledCodeItem::CommentStatement(comment) => {
                    source.add_line(format!("/* {} */", comment));
                }

                DecompiledCodeItem::AssignStatement {
                    variable,
                    value,
                    is_decl,
                } => {
                    if *is_decl {
                        to_decl_source(
                            &mut source,
                            format!("let {} = ", naming.variable(*variable)).as_str(),
                            ";",
                            value,
                            naming,
                        )?;
                    } else {
                        source.add_line(format!(
                            "{} = {};",
                            naming.variable(*variable),
                            value.to_source(naming)?
                        ));
                    }
                }

                DecompiledCodeItem::AssignTupleStatement {
                    variables,
                    value,
                    is_decl,
                } => {
                    source.add_line(format!(
                        "{}({}) = {};",
                        if *is_decl { "let " } else { "" },
                        variables
                            .iter()
                            .map(|v| naming.variable(*v))
                            .collect::<Vec<_>>()
                            .join(", "),
                        value.to_source(naming)?
                    ));
                }

                DecompiledCodeItem::AssignStructureStatement {
                    structure_visible_name,
                    variables,
                    value,
                } => {
                    if variables.len() >= 2 {
                        source.add_line(format!("let {} {{", structure_visible_name));
                        let mut inner_unit = SourceCodeUnit::new(1);
                        let k_max_width = variables.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

                        for (k, v) in variables {
                            inner_unit.add_line(format!(
                                "{:width$} : {},",
                                k,
                                naming.variable(*v),
                                width = k_max_width
                            ));
                        }

                        source.add_block(inner_unit);
                        source.add_line(format!("}} = {};", value.to_source(naming)?));
                    } else {
                        source.add_line(format!(
                            "let {} {{ {} }} = {};",
                            structure_visible_name,
                            variables
                                .iter()
                                .map(|(k, v)| format!("{}: {}", k, naming.variable(*v)))
                                .collect::<Vec<_>>()
                                .join(", "),
                            value.to_source(naming)?,
                        ));
                    }
                }

                DecompiledCodeItem::Statement { expr } => {
                    source.add_line(format!("{};", expr.to_source(naming)?));
                }

                DecompiledCodeItem::IfElseStatement {
                    cond,
                    if_unit,
                    else_unit,
                    result_variables,
                    use_as_result,
                } => {
                    let prefix = match use_as_result {
                        ResultUsageType::None => let_assigment_or_empty(result_variables, naming),
                        ResultUsageType::Return => {
                            if can_obmit_return {
                                String::new()
                            } else {
                                "return ".to_string()
                            }
                        }
                        ResultUsageType::Abort => "abort ".to_string(),
                        ResultUsageType::BlockResult => String::new(),
                    };

                    source.add_line(format!("{}if ({}) {{", prefix, cond.to_source(naming)?,));

                    let mut if_b = if_unit.to_source(naming, false)?;
                    if_b.add_indent(1);
                    source.add_block(if_b);

                    let mut else_b = else_unit.to_source(naming, false)?;
                    else_b.add_indent(1);

                    if !else_b.is_empty() {
                        source.add_line(format!("}} else {{"));
                        source.add_block(else_b);
                    }

                    if use_as_result != &ResultUsageType::None {
                        source.add_line(format!("}}"));
                    } else {
                        source.add_line(format!("}};"));
                    }
                }

                DecompiledCodeItem::WhileStatement { cond, body } => {
                    if cond.is_none() {
                        source.add_line(format!("loop {{"));
                    } else {
                        source.add_line(format!(
                            "while ({}) {{",
                            cond.as_ref().unwrap().to_source(naming)?
                        ));
                    }

                    let mut b = body.to_source(naming, false)?;
                    b.add_indent(1);
                    source.add_block(b);
                    source.add_line(format!("}};"));
                }
            }
        }

        if let Some(value) = &self.exit {
            source.add_line(format!("{}", value.to_source(naming)?));
        }

        Ok(source)
    }
}

fn to_decl_source(
    source: &mut SourceCodeUnit,
    prefix: &str,
    suffix: &str,
    value: &DecompiledExpr,
    naming: &Naming<'_>,
) -> Result<(), anyhow::Error> {
    let value = value.to_source_decl(naming)?;
    let value = prefix.to_string() + &value + suffix;
    let lines = value.split("\n").collect::<Vec<_>>();

    if lines.len() > 1 {
        source.add_line(lines[0].to_string());

        let mut inner_unit = SourceCodeUnit::new(1);

        for line in &lines[1..lines.len() - 1] {
            inner_unit.add_line(line.to_string());
        }

        source.add_block(inner_unit);
        source.add_line(lines[lines.len() - 1].to_string());
    } else {
        source.add_line(value);
    }

    Ok(())
}

fn let_assigment_or_empty(result_variables: &Vec<usize>, naming: &Naming) -> String {
    if result_variables.is_empty() {
        String::new()
    } else {
        let vars = format!(
            "{}",
            result_variables
                .iter()
                .map(|v| naming.variable(*v))
                .collect::<Vec<_>>()
                .join(", ")
        );

        if result_variables.len() > 1 {
            format!("let ({}) = ", vars)
        } else {
            format!("let {} = ", vars)
        }
    }
}
