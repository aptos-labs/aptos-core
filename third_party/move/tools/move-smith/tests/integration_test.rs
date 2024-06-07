// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use arbitrary::Unstructured;
use move_smith::{ast::*, codegen::*, move_smith::*, names::Identifier, types::*, utils::*};
use num_bigint::BigUint;
use std::cell::RefCell;

fn simple_module() -> Module {
    Module {
        name: Identifier(String::from("SimpleModule")),
        functions: vec![RefCell::new(Function {
            signature: FunctionSignature {
                name: Identifier(String::from("fun1")),
                parameters: vec![
                    (Identifier(String::from("param1")), Type::U64),
                    (Identifier(String::from("param2")), Type::U8),
                ],
                return_type: Some(Type::U32),
            },
            visibility: Visibility { public: true },
            body: Some(Block {
                stmts: vec![Statement::Expr(Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(42u32),
                    typ: Type::U32,
                }))],
                return_expr: Some(Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(111u32),
                    typ: Type::U32,
                })),
            }),
        })],
        structs: Vec::new(),
    }
}

fn simple_script() -> Script {
    Script {
        main: vec![FunctionCall {
            name: Identifier(String::from("0xCAFE::SimpleModule::fun1")),
            args: vec![
                Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(555u64),
                    typ: Type::U64,
                }),
                Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(255u8),
                    typ: Type::U8,
                }),
            ],
        }],
    }
}

fn simple_compile_unit() -> CompileUnit {
    CompileUnit {
        modules: vec![simple_module()],
        scripts: vec![simple_script()],
    }
}

#[test]
fn test_emit_code() {
    let lines = simple_module().emit_code_lines();
    println!("{}", lines.join("\n"));
    assert_eq!(lines.len(), 8);
    assert_eq!(lines[0], "//# publish");
    assert_eq!(lines[1], "module 0xCAFE::SimpleModule {");
    assert_eq!(
        lines[2],
        "    public fun fun1(param1: u64, param2: u8): u32"
    );
    assert_eq!(lines[3], "    {");
    assert_eq!(lines[4], "        42u32;");
    assert_eq!(lines[5], "        111u32");
    assert_eq!(lines[6], "    }");
    assert_eq!(lines[7], "}\n");
}

#[test]
fn test_generation_and_compile() {
    let raw_data = get_random_bytes(12345, 4096);
    let mut u = Unstructured::new(&raw_data);
    let mut smith = MoveSmith::default();
    smith.generate(&mut u).unwrap();
    let compile_unit = smith.get_compile_unit();
    let lines = compile_unit.emit_code();
    println!("{}", lines);

    compile_modules(lines);
}

#[test]
fn test_run_transactional_test() {
    let code = simple_compile_unit().emit_code();
    run_transactional_test(code).unwrap();
}
