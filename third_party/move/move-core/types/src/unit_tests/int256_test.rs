// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::int256::{I256, U256};
use num::BigInt;
use proptest::prelude::*;
use std::str::FromStr;

// Both wrappers convert into `BigInt`, whose decimal rendering is the oracle.
fn expected(value: impl Into<BigInt>) -> String {
    value.into().to_string()
}

fn ten_pow(exponent: u32) -> U256 {
    ethnum::U256::from(10u8).pow(exponent).into()
}

#[test]
fn unsigned_decimal_boundaries() {
    let chunk = ten_pow(19);
    let values = [
        U256::ZERO,
        U256::ONE,
        U256::from(9u8),
        U256::from(10u8),
        U256::checked_sub(chunk, U256::ONE).unwrap(),
        chunk,
        U256::checked_add(chunk, U256::ONE).unwrap(),
        ten_pow(38),
        U256::checked_sub(ten_pow(57), U256::ONE).unwrap(),
        U256::from(u128::MAX),
        U256::checked_add(U256::from(u128::MAX), U256::ONE).unwrap(),
        U256::MAX,
    ];
    for value in values {
        let text = value.to_string();
        assert_eq!(text, expected(value));
        assert_eq!(format!("{:?}", value), format!("U256({})", text));
        assert_eq!(U256::from_str(&text).unwrap(), value);
    }
}

#[test]
fn signed_decimal_boundaries() {
    let chunk = I256::try_from(ten_pow(19)).unwrap();
    let u128_max = I256::try_from(U256::from(u128::MAX)).unwrap();
    let values = [
        I256::ZERO,
        I256::ONE,
        -I256::ONE,
        I256::from(-10i8),
        chunk,
        -chunk,
        I256::from(i128::MIN),
        I256::from(i128::MAX),
        -I256::from(i128::MIN),
        I256::checked_sub(I256::from(i128::MIN), I256::ONE).unwrap(),
        u128_max,
        -u128_max,
        I256::checked_sub(-u128_max, I256::ONE).unwrap(),
        I256::MIN,
        I256::MAX,
    ];
    for value in values {
        let text = value.to_string();
        assert_eq!(text, expected(value));
        assert_eq!(format!("{:?}", value), format!("I256({})", text));
        assert_eq!(I256::from_str(&text).unwrap(), value);
    }
}

#[test]
fn formatting_flags_match_primitives() {
    let unsigned = U256::from(42u8);
    assert_eq!(format!("{:>6}", unsigned), format!("{:>6}", 42u64));
    assert_eq!(format!("{:<6}|", unsigned), format!("{:<6}|", 42u64));
    assert_eq!(format!("{:06}", unsigned), format!("{:06}", 42u64));
    assert_eq!(format!("{:+}", unsigned), format!("{:+}", 42u64));

    let signed = I256::from(-42i8);
    assert_eq!(format!("{:>6}", signed), format!("{:>6}", -42i64));
    assert_eq!(format!("{:06}", signed), format!("{:06}", -42i64));
    assert_eq!(format!("{:+}", I256::from(42i8)), format!("{:+}", 42i64));
}

proptest! {
    #[test]
    fn unsigned_display_matches_bigint(value in any::<U256>()) {
        let text = value.to_string();
        prop_assert_eq!(&text, &expected(value));
        prop_assert_eq!(U256::from_str(&text).unwrap(), value);
    }

    #[test]
    fn signed_display_matches_bigint(value in any::<I256>()) {
        let text = value.to_string();
        prop_assert_eq!(&text, &expected(value));
        prop_assert_eq!(I256::from_str(&text).unwrap(), value);
    }
}
