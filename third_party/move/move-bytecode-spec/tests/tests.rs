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
fn test_simple_1() {
    #[bytecode_spec]
    enum Bytecode {
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([("name", "add")])]);
}

#[test]
#[allow(dead_code)]
fn test_simple_2() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        Add,
        Sub,
    }

    assert_eq!(Bytecode::spec(), vec![
        test_map([("name", "add"), ("group", "arithmetic")]),
        test_map([("name", "sub")]),
    ]);
}

#[test]
#[allow(dead_code)]
fn test_simple_3() {
    #[bytecode_spec]
    enum Bytecode {
        #[group = "arithmetic"]
        #[description = "add two numbers up"]
        Add,
        #[group = "arithmetic"]
        Sub,
    }

    assert_eq!(Bytecode::spec(), vec![
        test_map([
            ("name", "add"),
            ("group", "arithmetic"),
            ("description", "add two numbers up"),
        ]),
        test_map([("name", "sub"), ("group", "arithmetic")]),
    ]);
}

#[test]
#[allow(dead_code)]
fn test_indentation() {
    #[bytecode_spec]
    enum Bytecode {
        #[description = r#"
             first line
               second line
              third line
        "#]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("description", "first line\n  second line\n third line")
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_lines_in_between() {
    #[bytecode_spec]
    enum Bytecode {
        #[description = r#"

             first line


               second line


        "#]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("description", "first line\n\n\n  second line")
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_entry_1() {
    #[bytecode_spec]
    enum Bytecode {
        #[description = ""]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("description", "")
    ])]);
}

#[test]
#[allow(dead_code)]
fn test_empty_entry_2() {
    #[bytecode_spec]
    enum Bytecode {
        #[description = "  "]
        Add,
    }

    assert_eq!(Bytecode::spec(), vec![test_map([
        ("name", "add"),
        ("description", "")
    ])]);
}
