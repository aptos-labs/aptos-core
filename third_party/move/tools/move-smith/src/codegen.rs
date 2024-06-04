// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ast::*, names::Identifier, types::Type};
use std::vec;

/// Generates Move source code from an AST.
/// `emit_code_lines` should be implemented for each AST node.
pub trait CodeGenerator {
    fn emit_code(&self) -> String {
        self.emit_code_lines().join("\n")
    }
    fn emit_code_lines(&self) -> Vec<String>;
}

/// The number of spaces to use for indentation.
const INDENTATION_SIZE: usize = 4;

/// Helper function add indentation to each line of code.
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
        vec![self.0.clone()]
    }
}

/// To follow the transactional test format, we output module code first, then script code.
impl CodeGenerator for CompileUnit {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = Vec::new();
        for m in &self.modules {
            code.extend(m.emit_code_lines());
        }

        for s in &self.scripts {
            code.extend(s.emit_code_lines());
        }

        code
    }
}

impl CodeGenerator for Script {
    fn emit_code_lines(&self) -> Vec<String> {
        // The `//# run` is for the transactional test
        let mut code = vec!["//# run".to_string(), "script {".to_string()];
        let main = Function {
            signature: FunctionSignature {
                parameters: Vec::new(),
                return_type: None,
            },
            visibility: Visibility { public: false },
            // Hardcode one function to simplify the output
            name: Identifier("main".to_string()),
            body: Some(FunctionBody {
                stmts: self
                    .main
                    .iter()
                    .map(|f| Statement::Expr(Expression::FunctionCall(f.clone())))
                    .collect(),
            }),
            return_stmt: None,
        };
        let main_code = main.emit_code_lines();
        append_code_lines_with_indentation(&mut code, main_code, INDENTATION_SIZE);
        code.push("}\n".to_string());
        code
    }
}

/// Output struct definitions and then function definitions in a module.
impl CodeGenerator for Module {
    fn emit_code_lines(&self) -> Vec<String> {
        // The `//# publish` is for the transactional test
        // TODO: remove the hardcoded address
        let mut code = vec![
            "//# publish".to_string(),
            format!("module 0xCAFE::{} {{", self.name.emit_code()),
        ];

        for s in &self.structs {
            append_code_lines_with_indentation(&mut code, s.emit_code_lines(), INDENTATION_SIZE)
        }

        for f in &self.functions {
            append_code_lines_with_indentation(&mut code, f.emit_code_lines(), INDENTATION_SIZE)
        }

        code.push("}\n".to_string());
        code
    }
}

impl CodeGenerator for StructDefinition {
    fn emit_code_lines(&self) -> Vec<String> {
        let abilities = match self.abilities.len() {
            0 => "".to_string(),
            _ => {
                let abilities = self
                    .abilities
                    .iter()
                    .map(|ability| ability.emit_code())
                    .collect::<Vec<String>>()
                    .join(", ")
                    .to_string();
                format!("has {} ", abilities)
            },
        };
        let mut code = vec![format!("struct {} {}{{", self.name.emit_code(), abilities)];

        let mut fields_code = Vec::new();
        for (field_name, field_type) in &self.fields {
            fields_code.push(format!(
                "{}: {},",
                field_name.emit_code(),
                field_type.emit_code(),
            ));
        }
        append_code_lines_with_indentation(&mut code, fields_code, INDENTATION_SIZE);
        code.push("}\n".to_string());
        code
    }
}

impl CodeGenerator for Ability {
    fn emit_code_lines(&self) -> Vec<String> {
        match self {
            Ability::Copy => vec!["copy".to_string()],
            Ability::Drop => vec!["drop".to_string()],
            Ability::Store => vec!["store".to_string()],
            Ability::Key => vec!["key".to_string()],
        }
    }
}

impl CodeGenerator for Function {
    fn emit_code_lines(&self) -> Vec<String> {
        let parameters = match self.signature.parameters.len() {
            0 => "".to_string(),
            _ => {
                let params: Vec<String> = self
                    .signature
                    .parameters
                    .iter()
                    .map(|(ident, typ)| format!("{}: {}", ident.emit_code(), typ.emit_code()))
                    .collect();
                params.join(", ").to_string()
            },
        };

        let return_type = match self.signature.return_type {
            Some(ref typ) => format!(": {}", typ.emit_code()),
            None => "".to_string(),
        };

        let visibility = if self.visibility.public {
            "public "
        } else {
            ""
        };

        let mut code = vec![format!(
            "{}fun {}({}){} {{",
            visibility,
            self.name.emit_code(),
            parameters,
            return_type
        )];
        let mut body = match self.body {
            Some(ref body) => body.emit_code_lines(),
            None => Vec::new(),
        };

        if let Some(ref expr) = self.return_stmt {
            body.push(expr.emit_code().to_string());
        }

        append_code_lines_with_indentation(&mut code, body, INDENTATION_SIZE);
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
            Expression::Boolean(b) => vec![b.to_string()],
            Expression::FunctionCall(c) => c.emit_code_lines(),
            Expression::StructInitialization(s) => s.emit_code_lines(),
        }
    }
}

impl CodeGenerator for StructInitialization {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = vec![format!("{} {{", self.name.emit_code())];

        let mut field_code = Vec::new();
        for (field, expr) in &self.fields {
            field_code.push(format!("{}: {}", field.emit_code(), expr.emit_code()));
        }
        append_code_lines_with_indentation(&mut code, field_code, INDENTATION_SIZE);
        code.push("}\n".to_string());
        code
    }
}

impl CodeGenerator for FunctionCall {
    fn emit_code_lines(&self) -> Vec<String> {
        let mut code = format!("{}(", self.name.emit_code());
        code.push_str(
            self.args
                .iter()
                .map(|arg| arg.emit_code())
                .collect::<Vec<String>>()
                .join(", ")
                .as_str(),
        );
        code.push(')');
        vec![code]
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
            T::Bool => "bool".to_string(),
            T::Struct(id) => id.emit_code(),
            _ => unimplemented!(),
        }]
    }
}
