// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use move_bytecode_spec::bytecode_spec;
use std::collections::BTreeMap;

fn test_map(
    entries: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> BTreeMap<String, String> {
    entries
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn test_empty_enum() {
    #[bytecode_spec]
    enum Empty {}

    assert_eq!(Empty::spec(), vec![]);
}

#[test]
#[allow(dead_code)]
fn test_two_instructions() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = "add two numbers up"]
        #[semantics = "stack >> a; stack >> b; stack << a + b;"]
        Add,
        #[group = "arithmetic"]
        #[description = "subtract two numbers"]
        #[semantics = "stack >> a; stack >> b; stack << b - a;"]
        Sub,
    }

    assert_eq!(Bytecode::spec(), vec![
        test_map([
            ("name", "add"),
            ("group", "arithmetic"),
            ("description", "add two numbers up"),
            ("semantics", "stack >> a; stack >> b; stack << a + b;")
        ]),
        test_map([
            ("name", "sub"),
            ("group", "arithmetic"),
            ("description", "subtract two numbers"),
            ("semantics", "stack >> a; stack >> b; stack << b - a;")
        ]),
    ]);
}

#[test]
#[allow(dead_code)]
fn test_name_optional() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = ""]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("description", ""),
        ("semantics", ""),
        ("group", "arithmetic"),
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_name_override() {
    #[bytecode_spec]
    enum Bytecode {
        #[name = "ADD"]
        #[group = "arithmetic"]
        #[description = ""]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "ADD"),
        ("description", ""),
        ("semantics", ""),
        ("group", "arithmetic"),
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_indentation() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = r#"
             first line
               second line
              third line
        "#]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("group", "arithmetic"),
        ("description", "first line\n  second line\n third line"),
        ("semantics", ""),
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_lines_in_between() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = r#"

             first line


               second line


        "#]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("group", "arithmetic"),
        ("description", "first line\n\n\n  second line"),
        ("semantics", ""),
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_entry_1() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = ""]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("group", "arithmetic"),
        ("description", ""),
        ("semantics", ""),
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_entry_2() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = "  "]
        #[semantics = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("group", "arithmetic"),
        ("description", ""),
        ("semantics", ""),
    ])]);
}
