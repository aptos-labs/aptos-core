// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Verifies Mono's V1 cast descriptor against V1.
//!
//! Cast messages depend on the operand value and Rust source and target type
//! names. This test calls the real `Value::cast_*` implementations for every
//! type pair and boundary value.
//!
//! V2 need not use this format; see [`mono_move_output::v1_error`].

use mono_move_core::IntTy;
use mono_move_output::v1_error::{describe_runtime_error, V1Equivalent};
use mono_move_runtime::{ReportedIntValue, RuntimeError};
use move_core_types::int256::{I256, U256};
use move_vm_types::values::Value;

const INT_TYPES: [IntTy; 12] = [
    IntTy::U8,
    IntTy::U16,
    IntTy::U32,
    IntTy::U64,
    IntTy::U128,
    IntTy::U256,
    IntTy::I8,
    IntTy::I16,
    IntTy::I32,
    IntTy::I64,
    IntTy::I128,
    IntTy::I256,
];

/// Builds the same operand for both VMs, or `None` if `ty` cannot hold the
/// candidate.
fn operand(ty: IntTy, candidate: &ReportedIntValue) -> Option<(Value, ReportedIntValue)> {
    let (unsigned, signed) = match candidate {
        ReportedIntValue::Unsigned(value) => (Some(**value), None),
        ReportedIntValue::Signed(value) => (None, Some(**value)),
    };
    macro_rules! narrow {
        ($source:expr, $ctor:path, $rust_ty:ty) => {{
            let narrowed = <$rust_ty>::try_from($source?).ok()?;
            ($ctor(narrowed), ReportedIntValue::from(narrowed))
        }};
    }
    Some(match ty {
        IntTy::U8 => narrow!(unsigned, Value::u8, u8),
        IntTy::U16 => narrow!(unsigned, Value::u16, u16),
        IntTy::U32 => narrow!(unsigned, Value::u32, u32),
        IntTy::U64 => narrow!(unsigned, Value::u64, u64),
        IntTy::U128 => narrow!(unsigned, Value::u128, u128),
        IntTy::U256 => narrow!(unsigned, Value::u256, U256),
        IntTy::I8 => narrow!(signed, Value::i8, i8),
        IntTy::I16 => narrow!(signed, Value::i16, i16),
        IntTy::I32 => narrow!(signed, Value::i32, i32),
        IntTy::I64 => narrow!(signed, Value::i64, i64),
        IntTy::I128 => narrow!(signed, Value::i128, i128),
        IntTy::I256 => narrow!(signed, Value::i256, I256),
    })
}

/// Runs the V1 cast, rendering its failure the way a baseline would.
fn v1_cast_failure(to: IntTy, value: Value) -> Option<String> {
    let result = match to {
        IntTy::U8 => value.cast_u8().map(|_| ()),
        IntTy::U16 => value.cast_u16().map(|_| ()),
        IntTy::U32 => value.cast_u32().map(|_| ()),
        IntTy::U64 => value.cast_u64().map(|_| ()),
        IntTy::U128 => value.cast_u128().map(|_| ()),
        IntTy::U256 => value.cast_u256().map(|_| ()),
        IntTy::I8 => value.cast_i8().map(|_| ()),
        IntTy::I16 => value.cast_i16().map(|_| ()),
        IntTy::I32 => value.cast_i32().map(|_| ()),
        IntTy::I64 => value.cast_i64().map(|_| ()),
        IntTy::I128 => value.cast_i128().map(|_| ()),
        IntTy::I256 => value.cast_i256().map(|_| ()),
    };
    result
        .err()
        .map(|err| render(err.major_status(), err.message()))
}

fn render(status: move_core_types::vm_status::StatusCode, message: Option<&str>) -> String {
    format!("{:?} | {}", status, message.unwrap_or("None"))
}

/// Boundary values around every Move integer width, in both signednesses.
fn candidates() -> Vec<ReportedIntValue> {
    let mut candidates = vec![];
    for magnitude in [
        "0",
        "1",
        "127",
        "128",
        "255",
        "256",
        "32767",
        "32768",
        "65535",
        "65536",
        "2147483647",
        "2147483648",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "18446744073709551615",
        "18446744073709551616",
        "170141183460469231731687303715884105727",
        "170141183460469231731687303715884105728",
        "340282366920938463463374607431768211455",
        "340282366920938463463374607431768211456",
        "57896044618658097711785492504343953926634992332820282019728792003956564819967",
        "57896044618658097711785492504343953926634992332820282019728792003956564819968",
    ] {
        if let Ok(value) = U256::from_str_radix(magnitude, 10) {
            candidates.push(ReportedIntValue::Unsigned(Box::new(value)));
        }
        if let Ok(value) = I256::from_str_radix(magnitude, 10) {
            candidates.push(ReportedIntValue::Signed(Box::new(value)));
        }
        if let Ok(value) = I256::from_str_radix(&format!("-{magnitude}"), 10) {
            candidates.push(ReportedIntValue::Signed(Box::new(value)));
        }
    }
    candidates.push(ReportedIntValue::Unsigned(Box::new(U256::MAX)));
    candidates.push(ReportedIntValue::Signed(Box::new(I256::MIN)));
    candidates.push(ReportedIntValue::Signed(Box::new(I256::MAX)));
    candidates
}

#[test]
fn cast_messages_match_v1() {
    let candidates = candidates();
    let mut compared = 0usize;
    let mut mismatches = vec![];
    for from in INT_TYPES {
        for to in INT_TYPES {
            for candidate in &candidates {
                let Some((v1_operand, mono_operand)) = operand(from, candidate) else {
                    continue;
                };
                let rendered_operand = mono_operand.to_string();
                // Only failing casts produce a descriptor to compare.
                let Some(v1) = v1_cast_failure(to, v1_operand) else {
                    continue;
                };
                let V1Equivalent::Described(descriptor) =
                    describe_runtime_error(&RuntimeError::CastOutOfRange {
                        from,
                        to,
                        value: mono_operand,
                    })
                else {
                    panic!("a failed cast always has a V1 counterpart");
                };
                let mono = render(descriptor.status, descriptor.message.text());
                compared += 1;
                if mono != v1 {
                    mismatches.push(format!(
                        "{from} -> {to} (value {rendered_operand}): V1 `{v1}`, mono `{mono}`"
                    ));
                }
            }
        }
    }
    mismatches.sort();
    mismatches.dedup();
    assert!(
        mismatches.is_empty(),
        "{} of {compared} failing casts render differently:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    // Guards against the loop silently comparing nothing.
    assert!(
        compared > 500,
        "expected the matrix to cover many failing casts, compared only {compared}"
    );
}
