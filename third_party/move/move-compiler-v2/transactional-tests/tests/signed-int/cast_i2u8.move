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
   assert!((v0 as u8) == 0u8);
   assert!((v1 as u8) == 0u8);
   assert!((v2 as u8) == 0u8);
   assert!((v3 as u8) == 0u8);
   assert!((v4 as u8) == 0u8);
   assert!((v5 as u8) == 0u8);

   // ----- Success cases: 255 -----
   let w1: i16 = 255i16;
   let w2: i32 = 255i32;
   let w3: i64 = 255i64;
   let w4: i128 = 255i128;
   let w5: i256 = 255i256;
   assert!((w1 as u8) == 255u8);
   assert!((w2 as u8) == 255u8);
   assert!((w3 as u8) == 255u8);
   assert!((w4 as u8) == 255u8);
   assert!((w5 as u8) == 255u8);
}
}

// -----------------------------
// Abort cases: negative values
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = -1i8;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i16;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i32;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i64;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i128;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i256;
   let _ = v0 as u8; // expect to abort
}
}

// -----------------------------
// Abort cases: values above 255
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = 256i16;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 256i32;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 256i64;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 256i128;
   let _ = v0 as u8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 256i256;
   let _ = v0 as u8; // expect to abort
}
}
