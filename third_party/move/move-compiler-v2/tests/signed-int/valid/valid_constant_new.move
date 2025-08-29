module 0x42::constants {
  const V3_64: i64 = 1i64; // constant with annotated type and annotated value
  const V5_64: i64 = -1i64; // constant with annotated type and annotated negative value
  const V7_64: i64 = 9223372036854775807i64; // constant with annotated type and annotated, max value
  const V9_64: i64 = -9223372036854775808i64; // constant with annotated type and annotated, min value

  const V10_64: i64 = 9223372036854775808i64;
  const V11_64: i64 = -9223372036854775809i64;


  public fun  test_i64(): i64{
    V3_64 + V5_64 + V7_64 + V9_64
  }
}
