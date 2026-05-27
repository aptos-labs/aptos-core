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
   assert!((v0 as u64) == 0u64);
   assert!((v1 as u64) == 0u64);
   assert!((v2 as u64) == 0u64);
   assert!((v3 as u64) == 0u64);
   assert!((v4 as u64) == 0u64);
   assert!((v5 as u64) == 0u64);

   // ----- Success cases: u64::MAX -----
   let w1: i128 = 18446744073709551615i128;
   let w2: i256 = 18446744073709551615i256;
   assert!((w1 as u64) == 18446744073709551615u64);
   assert!((w2 as u64) == 18446744073709551615u64);
}
}

// -----------------------------
// Abort cases: negative values
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = -1i8;
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i16;
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i32;
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i64;
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i128;
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i256;
   let _ = v0 as u64; // expect to abort
}
}

// -----------------------------
// Abort cases: values above u64::MAX
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = 18446744073709551616i128; // one above u64::MAX
   let _ = v0 as u64; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 18446744073709551616i256; // one above u64::MAX
   let _ = v0 as u64; // expect to abort
}
}
