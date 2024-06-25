// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use arbitrary::Unstructured;
use move_smith::{
    ast::*,
    codegen::*,
    config::*,
    move_smith::*,
    names::{Identifier, IdentifierKind as IDKind},
    types::*,
    utils::*,
};
use num_bigint::BigUint;
use std::cell::RefCell;

fn simple_module() -> Module {
    Module {
        name: Identifier::new_str("SimpleModule", IDKind::Module),
        functions: vec![RefCell::new(Function {
            signature: FunctionSignature {
                type_parameters: Vec::new(),
                name: Identifier::new_str("fun1", IDKind::Function),
                parameters: vec![
                    (Identifier::new_str("param1", IDKind::Var), Type::U64),
                    (Identifier::new_str("param2", IDKind::Var), Type::U8),
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
            name: Identifier::new_str("0xCAFE::SimpleModule::fun1", IDKind::Function),
            type_args: vec![],
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
        runs: vec![],
    }
}

#[test]
fn test_emit_code() {
    let lines = simple_module().emit_code_lines();
    println!("{}", lines.join("\n"));
    assert_eq!(lines.len(), 7);
    assert_eq!(lines[0], "//# publish");
    assert_eq!(lines[1], "module 0xCAFE::SimpleModule {");
    assert_eq!(
        lines[2],
        "    public fun fun1(param1: u64, param2: u8): u32 {"
    );
    assert_eq!(lines[3], "        42u32;");
    assert_eq!(lines[4], "        111u32");
    assert_eq!(lines[5], "    }");
    assert_eq!(lines[6], "}\n");
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

    assert!(compile_move_code(lines, true, true));
}

#[test]
fn test_run_transactional_test() {
    let code = simple_compile_unit().emit_code();
    run_transactional_test(code, &Config::default()).unwrap();
}

#[test]
fn test_run_transactional_test_should_fail() {
    let code = r#" //# publish
module 0xCAFE::Module0 {
    struct HasCopyDrop has copy, drop {}

    struct C2<T1: drop, T2: copy> has copy, drop, store {}

    fun m1<T1: copy+drop, T2: copy+drop>(x: T1) {
        m2<C2<HasCopyDrop, T2>, HasCopyDrop>(C2{});
    }
    fun m2<T3: copy+drop, T4: copy+drop>(x: T3): T3 {
        m1<T3, T4>(x);
        x
    }
}"#;
    let result = run_transactional_test(code.to_string(), &Config::default());
    assert!(result.is_err());
}