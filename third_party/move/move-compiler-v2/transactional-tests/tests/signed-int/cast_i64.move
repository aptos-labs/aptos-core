//# run --verbose
script {
fun main() {
   let v0: i64 = 0i64;
   assert!((v0 as u8) == 0u8);
   assert!((v0 as u16) == 0u16);
   assert!((v0 as u32) == 0u32);
   assert!((v0 as u64) == 0u64);
   assert!((v0 as u128) == 0u128);
   assert!((v0 as u256) == 0u256);
   assert!((v0 as i8) == 0i8);
   assert!((v0 as i16) == 0i16);
   assert!((v0 as i32) == 0i32);
   assert!((v0 as i64) == 0i64);
   assert!((v0 as i128) == 0i128);
   assert!((v0 as i256) == 0i256);

   let v1: u8 = 0u8;
   let v2: u16 = 0u16;
   let v3: u32 = 0u32;
   let v4: u64 = 0u64;
   let v5: u128 = 0u128;
   let v6: u256 = 0u256;
   let v7: i8 = 0i8;
   let v8: i16 = 0i16;
   let v9: i32 = 0i32;
   let v10: i64 = 0i64;
   let v11: i128 = 0i128;
   let v12: i256 = 0i256;
   assert!((v1 as i64) == 0i64);
   assert!((v2 as i64) == 0i64);
   assert!((v3 as i64) == 0i64);
   assert!((v4 as i64) == 0i64);
   assert!((v5 as i64) == 0i64);
   assert!((v6 as i64) == 0i64);
   assert!((v7 as i64) == 0i64);
   assert!((v8 as i64) == 0i64);
   assert!((v9 as i64) == 0i64);
   assert!((v10 as i64) == 0i64);
   assert!((v11 as i64) == 0i64);
   assert!((v12 as i64) == 0i64);
}
}

//# run --verbose
script {
fun main() {
   let v0: i64 = 9223372036854775807i64; // i64::MAX
   assert!((v0 as u64) == 9223372036854775807u64);
   assert!((v0 as u128) == 9223372036854775807u128);
   assert!((v0 as u256) == 9223372036854775807u256);
   assert!((v0 as i64) == 9223372036854775807i64);
   assert!((v0 as i128) == 9223372036854775807i128);
   assert!((v0 as i256) == 9223372036854775807i256);

   let v4: u64 = 9223372036854775807u64;
   let v5: u128 = 9223372036854775807u128;
   let v6: u256 = 9223372036854775807u256;
   let v7: i64 = 9223372036854775807i64;
   let v8: i128 = 9223372036854775807i128;
   let v9: i256 = 9223372036854775807i256;
   assert!((v4 as i64) == 9223372036854775807i64);
   assert!((v5 as i64) == 9223372036854775807i64);
   assert!((v6 as i64) == 9223372036854775807i64);
   assert!((v7 as i64) == 9223372036854775807i64);
   assert!((v8 as i64) == 9223372036854775807i64);
   assert!((v9 as i64) == 9223372036854775807i64);
}
}

//# run --verbose
script {
fun main() {
   let v0: i64 = -9223372036854775808i64; // i64::MIN
   assert!((v0 as i64) == -9223372036854775808i64);
   assert!((v0 as i128) == -9223372036854775808i128);
   assert!((v0 as i256) == -9223372036854775808i256);

   let v7 = -9223372036854775808i64;
   let v8 = -9223372036854775808i128;
   let v9 = -9223372036854775808i256;
   assert!((v7 as i64) == -9223372036854775808i64);
   assert!((v8 as i64) == -9223372036854775808i64);
   assert!((v9 as i64) == -9223372036854775808i64);
}
}

// Unsigned → i64 (overflow when ≥ 9_223_372_036_854_775_808)

//# run --verbose
script {
fun main() {
   let v0 = 9223372036854775808u64;
   let v2 = v0 as i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 9223372036854775808u128;
   let v2 = v0 as i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 9223372036854775808u256;
   let v2 = v0 as i64; // expect to abort
}
}

// Signed → i64 (overflow above i64::MAX = 9_223_372_036_854_775_807)

//# run --verbose
script {
fun main() {
   let v0 = 9223372036854775808i128;
   let v2 = v0 as i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 9223372036854775808i256;
   let v2 = v0 as i64; // expect to abort
}
}

// Signed → i64 (overflow below i64::MIN = -9_223_372_036_854_775_808)

//# run --verbose
script {
fun main() {
   let v0 = -9223372036854775809i128;
   let v2 = v0 as i64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -9223372036854775809i256;
   let v2 = v0 as i64; // expect to abort
}
}
