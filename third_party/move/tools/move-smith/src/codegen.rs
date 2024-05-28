// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ast::*, names::Identifier, types::Type};
use std::vec;

pub trait CodeGenerator {
    fn emit_code(&self) -> String {
        self.emit_code_lines().join("\n")
    }
    fn emit_code_lines(&self) -> Vec<String>;
}

const INDENTATION_SIZE: usize = 4;

fn append_code_lines_with_indentation(
    program: &mut Vec<String>,
    lines: Vec<String>,
    indentation: usize,
) {
    for line in lines {
        program.push(format!("{:indent$}{}", "", line, indent = indentation));
    }
}

impl CodeGenerator for Identifier {
    fn emit_code_lines(&self) -> Vec<String> {
        vec![self.clone()]
    }
}

impl CodeGenerator for Module {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = vec![format!("module 0xCAFE::{} {{", self.name.emit_code())];
        for member in &self.members {
            // Prepend 4 spaces to each line of the member's code
            append_code_lines_with_indentation(
                &mut code,
                member.emit_code_lines(),
                INDENTATION_SIZE,
            )
        }
        code.push("}\n".to_string());
        code
    }
}

impl CodeGenerator for ModuleMember {
    fn emit_code_lines(&self) -> Vec<String> {
        match self {
            ModuleMember::Function(f) => f.emit_code_lines(),
        }
    }
}

impl CodeGenerator for Function {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = vec![format!("fun {}() {{", self.name.emit_code())];
        append_code_lines_with_indentation(
            &mut code,
            self.body.emit_code_lines(),
            INDENTATION_SIZE,
        );
        code.push("}".to_string());
        code
    }
}

impl CodeGenerator for FunctionBody {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = Vec::new();
        for stmt in &self.stmts {
            code.extend(stmt.emit_code_lines());
        }
        code
    }
}

impl CodeGenerator for Statement {
    fn emit_code_lines(&self) -> Vec<String> {
        match self {
            Statement::Decl(decl) => decl.emit_code_lines(),
            Statement::Expr(expr) => vec![format!("{};", expr.emit_code())],
        }
    }
}

impl CodeGenerator for Declaration {
    fn emit_code_lines(&self) -> Vec<String> {
        let rhs = match self.value {
            Some(ref expr) => format!(" = {}", expr.emit_code()),
            None => "".to_string(),
        };
        vec![format!(
            "let {}: {}{};",
            self.name.emit_code(),
            self.typ.emit_code(),
            rhs
        )]
    }
}

impl CodeGenerator for Expression {
    fn emit_code_lines(&self) -> Vec<String> {
        match self {
            Expression::NumberLiteral(n) => n.emit_code_lines(),
            Expression::Variable(ident) => ident.emit_code_lines(),
        }
    }
}

impl CodeGenerator for NumberLiteral {
    fn emit_code_lines(&self) -> Vec<String> {
        vec![format!("{}{}", self.value, self.typ.emit_code())]
    }
}

impl CodeGenerator for Type {
    fn emit_code_lines(&self) -> Vec<String> {
        use Type as T;
        vec![match self {
            T::U8 => "u8".to_string(),
            T::U16 => "u16".to_string(),
            T::U32 => "u32".to_string(),
            T::U64 => "u64".to_string(),
            T::U128 => "u128".to_string(),
            T::U256 => "u256".to_string(),
            _ => unimplemented!(),
        }]
    }
}
