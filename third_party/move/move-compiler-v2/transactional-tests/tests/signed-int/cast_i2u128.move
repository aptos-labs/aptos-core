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
   assert!((v0 as u128) == 0u128);
   assert!((v1 as u128) == 0u128);
   assert!((v2 as u128) == 0u128);
   assert!((v3 as u128) == 0u128);
   assert!((v4 as u128) == 0u128);
   assert!((v5 as u128) == 0u128);

   // ----- Success cases: u128::MAX -----
   let w1: i256 = 340282366920938463463374607431768211455i256;
   assert!((w1 as u128) == 340282366920938463463374607431768211455u128);
}
}

// -----------------------------
// Abort cases: negative values
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = -1i8;
   let _ = v0 as u128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i16;
   let _ = v0 as u128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i32;
   let _ = v0 as u128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i64;
   let _ = v0 as u128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i128;
   let _ = v0 as u128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i256;
   let _ = v0 as u128; // expect to abort
}
}

// -----------------------------
// Abort cases: values above u128::MAX
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = 340282366920938463463374607431768211456i256; // one above u128::MAX
   let _ = v0 as u128; // expect to abort
}
}
