// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::effects::Op;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TestError;

#[test]
fn test_ops() {
    let f = |i: u32| -> u32 { i + 10 };

    // Test map preserves op variant.
    assert_eq!(Op::New(1).map(f), Op::New(11));
    assert_eq!(Op::Modify(2).map(f), Op::Modify(12));
    assert_eq!(Op::Delete.map(f), Op::Delete);

    let f = |i: u32| -> anyhow::Result<u32, TestError> { i.checked_sub(10).ok_or(TestError) };

    // Test and_then preserves op variant and returns an error if
    // function application fails.
    assert_eq!(Op::New(11).and_then(f), Ok(Op::New(1)));
    assert_eq!(Op::New(1).and_then(f), Err(TestError));
    assert_eq!(Op::Modify(12).and_then(f), Ok(Op::Modify(2)));
    assert_eq!(Op::Modify(2).and_then(f), Err(TestError));
    assert_eq!(Op::Delete.and_then(f), Ok(Op::Delete));

    // Test data is correctly passed out.
    assert_eq!(Op::New(1).ok(), Some(1));
    assert_eq!(Op::Modify(2).ok(), Some(2));
    assert_eq!(Op::<u32>::Delete.ok(), None);
}
