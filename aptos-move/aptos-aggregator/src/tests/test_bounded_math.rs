// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::bounded_math::{BoundedMath, BoundedMathError, SignedU128};
use claims::{assert_err_eq, assert_ok_eq};
use BoundedMathError::*;
use SignedU128::*;

#[test]
fn test_unsigned_add_delta() {
    let math = BoundedMath::new(100);
    assert_ok_eq!(math.unsigned_add_delta(10, &Positive(5)), 15);
    assert_ok_eq!(math.unsigned_add_delta(10, &Negative(5)), 5);
    assert_err_eq!(math.unsigned_add_delta(10, &Positive(950)), Overflow);
    assert_err_eq!(math.unsigned_add_delta(10, &Negative(11)), Underflow);
}

#[test]
fn test_delta_minus() {
    use SignedU128::*;
    assert_eq!(Positive(10).minus(), Negative(10));
    assert_eq!(Negative(10).minus(), Positive(10));
}

#[test]
fn delta_add() {
    let math = BoundedMath::new(100);
    assert_ok_eq!(math.signed_add(&Positive(10), &Positive(5)), Positive(15));
    assert_ok_eq!(math.signed_add(&Positive(10), &Negative(5)), Positive(5));
    assert_ok_eq!(math.signed_add(&Negative(10), &Positive(5)), Negative(5));
    assert_ok_eq!(math.signed_add(&Negative(10), &Negative(5)), Negative(15));

    assert_ok_eq!(math.signed_add(&Positive(10), &Positive(90)), Positive(100));
    assert_ok_eq!(math.signed_add(&Negative(10), &Negative(90)), Negative(100));

    assert_err_eq!(math.signed_add(&Positive(10), &Positive(91)), Overflow);
    assert_err_eq!(math.signed_add(&Negative(10), &Negative(91)), Underflow);
}
