module 0x42::constants {
  const V1_64: i64 = 0; // constant with annotated type but non-annotated value
  const V2_64: i64 = 1; // constant with annotated type but non-annotated value
  const V3_64: i64 = 1i64; // constant with annotated type and annotated value
  const V4_64: i64 = -1; // constant with annotated type but non-annotated negative value
  const V5_64: i64 = -1i64; // constant with annotated type and annotated negative value
  const V6_64: i64 = 9223372036854775807; // constant with annotated type but non-annotated, max value
  const V7_64: i64 = 9223372036854775807i64; // constant with annotated type and annotated, max value
  const V8_64: i64 = -9223372036854775808; // constant with annotated type but non-annotated, min value
  const V9_64: i64 = -9223372036854775808i64; // constant with annotated type and annotated, min value

  const V1_128: i128 = 0; // constant with annotated type but non-annotated value
  const V2_128: i128 = 1; // constant with annotated type but non-annotated value
  const V3_128: i128 = 1i128; // constant with annotated type and annotated value
  const V4_128: i128 = -1; // constant with annotated type but non-annotated negative value
  const V5_128: i128 = -1i128; // constant with annotated type and annotated negative value
  const V6_128: i128 = 170141183460469231731687303715884105727; // constant with annotated type but non-annotated, max value
  const V7_128: i128 = 170141183460469231731687303715884105727i128; // constant with annotated type and annotated, max value
  const V8_128: i128 = -170141183460469231731687303715884105728; // constant with annotated type but non-annotated, min value
  const V9_128: i128 = -170141183460469231731687303715884105728i128; // constant with annotated type and annotated, min value

  public fun  test_i64() : i64 {
    let a_ann = 0i64; // constant with non-annotated type but annotated value
    let b_ann = 1i64; // constant with non-annotated type but annotated value
    let c_ann = -1i64; // constant with non-annotated type but annotated, negative value
    let d_ann = 9223372036854775807i64; // constant with non-annotated type but annotated, max value
    let e_ann = -9223372036854775808i64; // constant with non-annotated type but annotated, min value

    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 9223372036854775807; // constant with non-annotated type and non-annotated, max value
    let e = -9223372036854775808; // constant with non-annotated type and non-annotated, min value

    let (x, y, z) = (-1, -2, -3); // constants in tuple

    V1_64 + V2_64 + V3_64 + V4_64 + V5_64 + V6_64 + V7_64 + V8_64 + V9_64 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }

  public fun  test_i128() : i128 {
    let a_ann = 0i128; // constant with non-annotated type but annotated value
    let b_ann = 1i128; // constant with non-annotated type but annotated value
    let c_ann = -1i128; // constant with non-annotated type but annotated, negative value
    let d_ann = 170141183460469231731687303715884105727i128; // constant with non-annotated type but annotated, max value
    let e_ann = -170141183460469231731687303715884105728i128; // constant with non-annotated type but annotated, min value

    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 170141183460469231731687303715884105727; // constant with non-annotated type and non-annotated, max value
    let e = -170141183460469231731687303715884105728; // constant with non-annotated type and non-annotated, min value

    let (x, y, z) = (-1, -2, -3); // constants in tuple

    V1_128 + V2_128 + V3_128 + V4_128 + V5_128 + V6_128 + V7_128 + V8_128 + V9_128 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }
}
