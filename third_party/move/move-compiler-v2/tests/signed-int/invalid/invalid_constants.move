module 0x42::invalid_constants {
  use std::i64;
  use std::i128;

  const V1_64: i64 = 1u64; // type mismatch: expected i64, actual u64
  const V2_64: i64 = 9223372036854775808; // out of upper bound: max value of i64 is 9223372036854775807
  const V3_64: i64 = 9223372036854775808i64; // out of upper bound: max value of i64 is 9223372036854775807
  const V4_64: i64 = -9223372036854775809; // out of lower bound: min value of i64 is -9223372036854775808
  const V5_64: i64 = -9223372036854775809i64; // out of lower bound: min value of i64 is -9223372036854775808
  const V6_64: i64 = -1i128; // type mismatch: expected i64, actual i128

  const V1_128: i128 = 1u128; // type mismatch: expected i128, actual u128
  const V2_128: i128 = 170141183460469231731687303715884105728; // out of upper bound: max value of i128 is 170141183460469231731687303715884105727
  const V3_128: i128 = 170141183460469231731687303715884105728i128; // out of upper bound: max value of i128 is 170141183460469231731687303715884105727
  const V4_128: i128 = -170141183460469231731687303715884105729; // out of lower bound: min value of i64 is -170141183460469231731687303715884105728
  const V5_128: i128 = -170141183460469231731687303715884105729i128; // out of lower bound: min value of i64 is -170141183460469231731687303715884105728
  const V6_128: i128 = -1i64; // type mismatch: expected i128, actual i64

  const V_STRUCT64: i64::I64 = -1;
  const V_STRUCT128: i128::I128 = -1;

  public fun  test_i64() : i64 {
    let a = 1u64; // type mismatch in `res5 + a`: expected i64, actual u64
    let b = 9223372036854775808; // interpreted as a possible u64|u128|u256|i128
    let c = 9223372036854775808i64; // constant does not fit into i64 && type mismatch in `res7 + c`, expected u64|u128|u256|i128, actual i64
    let d = -9223372036854775809; // interpreted as an i128
    let e = -9223372036854775809i64; // constant does not fit into i64 && type mismatch in `res9 + e`, expected i128, actual i64

    let res1 = V1_64 + V2_64;
    let res2 = res1 + V3_64;
    let res3 = res2 + V4_64;
    let res4 = res3 + V5_64;
    let res5 = res4 + V6_64;
    let res6 = res5 + a;
    let res7 = res6 + b;
    let res8 = res7 + c;
    let res9 = res8 + d;
    let res10 = res9 + e;
    res10
  }

  public fun  test_i128() : i128 {
    let a = 1u128; // type mismatch in `res5 + a`: expected i128, actual u128
    let b = 170141183460469231731687303715884105728; // interpreted as a possible u128|u256
    let c = 170141183460469231731687303715884105728i128; // type mismatch in `res7 + c`, expected u128|u256, actual i128
    let d = -170141183460469231731687303715884105729; // no type can be inffered
    let e = -170141183460469231731687303715884105729i128; // constant does not fit into i128

    let res1 = V1_128 + V2_128;
    let res2 = res1 + V3_128;
    let res3 = res2 + V4_128;
    let res4 = res3 + V5_128;
    let res5 = res4 + V6_128;
    let res6 = res5 + a;
    let res7 = res6 + b;
    let res8 = res7 + c;
    let res9 = res8 + d;
    let res10 = res9 + e;
    res10
  }
}
