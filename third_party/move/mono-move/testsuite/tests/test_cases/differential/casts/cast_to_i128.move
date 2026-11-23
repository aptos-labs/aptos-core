// Casts to i128 from a narrower signed, a same-width unsigned and a wider
// signed source: sign extension, and the range check at both i128 bounds.

// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun from_i64(value: i64): i128 { value as i128 }
    fun from_u128(value: u128): i128 { value as i128 }
    fun from_i256(value: i256): i128 { value as i128 }
}

// RUN: execute 0x1::test::from_i64 --args -5
// CHECK: results: -5

// RUN: execute 0x1::test::from_i64 --args -9223372036854775808
// CHECK: results: -9223372036854775808

// RUN: execute 0x1::test::from_u128 --args 170141183460469231731687303715884105727
// CHECK: results: 170141183460469231731687303715884105727

// RUN: execute 0x1::test::from_u128 --args 170141183460469231731687303715884105728
// CHECK-ERROR-PARITY

// RUN: execute 0x1::test::from_i256 --args -170141183460469231731687303715884105728
// CHECK: results: -170141183460469231731687303715884105728

// RUN: execute 0x1::test::from_i256 --args -170141183460469231731687303715884105729
// CHECK-ERROR-PARITY

// RUN: execute 0x1::test::from_i256 --args 170141183460469231731687303715884105727
// CHECK: results: 170141183460469231731687303715884105727

// RUN: execute 0x1::test::from_i256 --args 170141183460469231731687303715884105728
// CHECK-ERROR-PARITY
