module 0x42::constants {

  const V1_8: i8 = 0; // constant with annotated type but non-annotated value
  const V2_8: i8 = 1; // constant with annotated type but non-annotated value
  const V3_8: i8 = 1i8; // constant with annotated type and annotated value
  const V4_8: i8 = -1; // constant with annotated type but non-annotated negative value
  const V5_8: i8 = -1i8; // constant with annotated type and annotated negative value
  const V6_8: i8 = 127; // constant with annotated type but non-annotated, max value
  const V7_8: i8 = 127i8; // constant with annotated type and annotated, max value
  const V8_8: i8 = -128; // constant with annotated type but non-annotated, min value
  const V9_8: i8 = -128i8; // constant with annotated type and annotated, min value

  const V1_16: i16 = 0; // constant with annotated type but non-annotated value
  const V2_16: i16 = 1; // constant with annotated type but non-annotated value
  const V3_16: i16 = 1i16; // constant with annotated type and annotated value
  const V4_16: i16 = -1; // constant with annotated type but non-annotated negative value
  const V5_16: i16 = -1i16; // constant with annotated type and annotated negative value
  const V6_16: i16 = 32767; // constant with annotated type but non-annotated, max value
  const V7_16: i16 = 32767i16; // constant with annotated type and annotated, max value
  const V8_16: i16 = -32768; // constant with annotated type but non-annotated, min value
  const V9_16: i16 = -32768i16; // constant with annotated type and annotated, min value

  const V1_32: i32 = 0; // constant with annotated type but non-annotated value
  const V2_32: i32 = 1; // constant with annotated type but non-annotated value
  const V3_32: i32 = 1i32; // constant with annotated type and annotated value
  const V4_32: i32 = -1; // constant with annotated type but non-annotated negative value
  const V5_32: i32 = -1i32; // constant with annotated type and annotated negative value
  const V6_32: i32 = 2147483647; // constant with annotated type but non-annotated, max value
  const V7_32: i32 = 2147483647i32; // constant with annotated type and annotated, max value
  const V8_32: i32 = -2147483648; // constant with annotated type but non-annotated, min value
  const V9_32: i32 = -2147483648i32; // constant with annotated type and annotated, min value

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

  const V1_256: i256 = 0; // constant with annotated type but non-annotated value
  const V2_256: i256 = 1; // constant with annotated type but non-annotated value
  const V3_256: i256 = 1i256; // constant with annotated type and annotated value
  const V4_256: i256 = -1; // constant with annotated type but non-annotated negative value
  const V5_256: i256 = -1i256; // constant with annotated type and annotated negative value
  const V6_256: i256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967; // constant with annotated type but non-annotated, max value
  const V7_256: i256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967i256; // constant with annotated type and annotated, max value
  const V8_256: i256 = -57896044618658097711785492504343953926634992332820282019728792003956564819968; // constant with annotated type but non-annotated, min value
  const V9_256: i256 = -57896044618658097711785492504343953926634992332820282019728792003956564819968i256; // constant with annotated type and annotated, min value

  public fun  test_i8() : i8 {
    let a_ann = 0i8; // constant with non-annotated type but annotated value
    let b_ann = 1i8; // constant with non-annotated type but annotated value
    let c_ann = -1i8; // constant with non-annotated type but annotated, negative value
    let d_ann = 127i8; // constant with non-annotated type but annotated, max value
    let e_ann = -128i8; // constant with non-annotated type but annotated, min value
    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 127; // constant with non-annotated type and non-annotated, max value
    let e = -128; // constant with non-annotated type and non-annotated, min value
    let (x, y, z) = (-1, -2, -3); // constants in tuple
    V1_8 + V2_8 + V3_8 + V4_8 + V5_8 + V6_8 + V7_8 + V8_8 + V9_8 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }

  public fun  test_i16() : i16 {
    let a_ann = 0i16; // constant with non-annotated type but annotated value
    let b_ann = 1i16; // constant with non-annotated type but annotated value
    let c_ann = -1i16; // constant with non-annotated type but annotated, negative value
    let d_ann = 32767i16; // constant with non-annotated type but annotated, max value
    let e_ann = -32768i16; // constant with non-annotated type but annotated, min value
    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 32767; // constant with non-annotated type and non-annotated, max value
    let e = -32768; // constant with non-annotated type and non-annotated, min value
    let (x, y, z) = (-1, -2, -3); // constants in tuple
    V1_16 + V2_16 + V3_16 + V4_16 + V5_16 + V6_16 + V7_16 + V8_16 + V9_16 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }

  public fun  test_i32() : i32 {
    let a_ann = 0i32; // constant with non-annotated type but annotated value
    let b_ann = 1i32; // constant with non-annotated type but annotated value
    let c_ann = -1i32; // constant with non-annotated type but annotated, negative value
    let d_ann = 2147483647i32; // constant with non-annotated type but annotated, max value
    let e_ann = -2147483648i32; // constant with non-annotated type but annotated, min value
    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 2147483647; // constant with non-annotated type and non-annotated, max value
    let e = -2147483648; // constant with non-annotated type and non-annotated, min value
    let (x, y, z) = (-1, -2, -3); // constants in tuple
    V1_32 + V2_32 + V3_32 + V4_32 + V5_32 + V6_32 + V7_32 + V8_32 + V9_32 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }

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

  public fun  test_i256() : i256 {
    let a_ann = 0i256; // constant with non-annotated type but annotated value
    let b_ann = 1i256; // constant with non-annotated type but annotated value
    let c_ann = -1i256; // constant with non-annotated type but annotated, negative value
    let d_ann = 57896044618658097711785492504343953926634992332820282019728792003956564819967i256; // constant with non-annotated type but annotated, max value
    let e_ann = -57896044618658097711785492504343953926634992332820282019728792003956564819968i256; // constant with non-annotated type but annotated, min value
    let a = 0; // constant with non-annotated type and non-annotated value
    let b = 1; // constant with non-annotated type and non-annotated value
    let c = -1; // constant with non-annotated type and non-annotated, negative value
    let d = 57896044618658097711785492504343953926634992332820282019728792003956564819967; // constant with non-annotated type and non-annotated, max value
    let e = -57896044618658097711785492504343953926634992332820282019728792003956564819968i256; // constant with non-annotated type and non-annotated, min value
    let (x, y, z) = (-1, -2, -3); // constants in tuple
    V1_256 + V2_256 + V3_256 + V4_256 + V5_256 + V6_256 + V7_256 + V8_256 + V9_256 + a_ann + b_ann + c_ann + d_ann + e_ann + a + b + c + d + e + x + y + z
  }
}
