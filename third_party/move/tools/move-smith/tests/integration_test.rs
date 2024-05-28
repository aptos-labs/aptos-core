// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use arbitrary::Unstructured;
use move_smith::{ast::*, codegen::*, move_smith::*, types::*, utils::*};
use num_bigint::BigUint;
use rand::{rngs::StdRng, Rng, SeedableRng};

fn simple_module() -> Module {
    Module {
        name: String::from("SimpleModule"),
        members: vec![ModuleMember::Function(Function {
            name: String::from("fun1"),
            body: FunctionBody {
                stmts: vec![Statement::Expr(Expression::NumberLiteral(NumberLiteral {
                    value: BigUint::from(42u32),
                    typ: Type::U32,
                }))],
            },
        })],
    }
}

fn get_raw_data() -> Vec<u8> {
    let seed: u64 = 12345;
    let mut rng = StdRng::seed_from_u64(seed);
    let mut buffer = vec![0u8; 1024];
    rng.fill(&mut buffer[..]);
    buffer
}

#[test]
fn test_emit_code() {
    let lines = simple_module().emit_code_lines();
    println!("{}", lines.join("\n"));
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "module 0xCAFE::SimpleModule {");
    assert_eq!(lines[1], "    fun fun1() {");
    assert_eq!(lines[2], "        42u32;");
    assert_eq!(lines[3], "    }");
    assert_eq!(lines[4], "}\n");
}

#[test]
fn test_generation_and_compile() {
    let raw_data = get_raw_data();
    let mut u = Unstructured::new(&raw_data);
    let mut smith = MoveSmith::default();
    let module = smith.generate_module(&mut u).unwrap();
    let lines = module.emit_code();
    println!("{}", lines);

    compile_modules(lines);
}
