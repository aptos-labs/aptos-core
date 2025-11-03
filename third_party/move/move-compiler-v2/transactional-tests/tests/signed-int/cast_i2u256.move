//# run --verbose
script {
fun main() {
   // ----- Success cases: 0 -----
   let v0: i8 = 0i8;
   let v1: i16 = 0i16;
   let v2: i32 = 0i32;
   let v3: i64 = 0i64;
   let v4: i128 = 0i128;
   let v5: i256 = 0i256;
   assert!((v0 as u256) == 0u256);
   assert!((v1 as u256) == 0u256);
   assert!((v2 as u256) == 0u256);
   assert!((v3 as u256) == 0u256);
   assert!((v4 as u256) == 0u256);
   assert!((v5 as u256) == 0u256);

   // ----- Success cases: large positive values (near signed max) -----
   let w1: i8 = 127i8;
   let w2: i16 = 32767i16;
   let w3: i32 = 2147483647i32;
   let w4: i64 = 9223372036854775807i64;
   let w5: i128 = 170141183460469231731687303715884105727i128; // i128::MAX
   let w6: i256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967i256; // i256::MAX
   assert!((w1 as u256) == 127u256);
   assert!((w2 as u256) == 32767u256);
   assert!((w3 as u256) == 2147483647u256);
   assert!((w4 as u256) == 9223372036854775807u256);
   assert!((w5 as u256) == 170141183460469231731687303715884105727u256);
   assert!((w6 as u256) == 57896044618658097711785492504343953926634992332820282019728792003956564819967u256);
}
}

// -----------------------------
// Abort cases: negative values
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = -1i8;
   let _ = v0 as u256; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i16;
   let _ = v0 as u256; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i32;
   let _ = v0 as u256; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i64;
   let _ = v0 as u256; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i128;
   let _ = v0 as u256; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i256;
   let _ = v0 as u256; // expect to abort
}
}
