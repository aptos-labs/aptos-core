// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use generate_format::lint_bcs_format;
use serde_reflection::{ContainerFormat, Format, Result};

fn test_newtypestruct_with_format(f: Format) -> Result<()> {
    lint_bcs_format(&ContainerFormat::NewTypeStruct(Box::new(f)))
}

#[test]
fn test_lint_bcs_format() {
    use Format::*;

    assert!(lint_bcs_format(&ContainerFormat::UnitStruct).is_err());

    assert!(lint_bcs_format(&ContainerFormat::TupleStruct(vec![])).is_err());

    assert!(lint_bcs_format(&ContainerFormat::TupleStruct(vec![Unit, U32])).is_ok());

    assert!(test_newtypestruct_with_format(Unit).is_err());

    assert!(test_newtypestruct_with_format(Seq(Box::new(Unit))).is_err());

    assert!(test_newtypestruct_with_format(Map {
        key: Box::new(Unit),
        value: Box::new(Unit)
    })
    .is_err());

    assert!(test_newtypestruct_with_format(Seq(Box::new(Tuple(Vec::new())))).is_err());

    assert!(test_newtypestruct_with_format(Seq(Box::new(Tuple(vec![Unit])))).is_err());

    assert!(test_newtypestruct_with_format(Seq(Box::new(Tuple(vec![Unit, U32])))).is_ok());
}
