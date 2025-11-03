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
   assert!((v0 as u16) == 0u16);
   assert!((v1 as u16) == 0u16);
   assert!((v2 as u16) == 0u16);
   assert!((v3 as u16) == 0u16);
   assert!((v4 as u16) == 0u16);
   assert!((v5 as u16) == 0u16);

   // ----- Success cases: 65535 -----
   let w1: i32 = 65535i32;
   let w2: i64 = 65535i64;
   let w3: i128 = 65535i128;
   let w4: i256 = 65535i256;
   assert!((w1 as u16) == 65535u16);
   assert!((w2 as u16) == 65535u16);
   assert!((w3 as u16) == 65535u16);
   assert!((w4 as u16) == 65535u16);
}
}

// -----------------------------
// Abort cases: negative values
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = -1i8;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i16;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i32;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i64;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i128;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -1i256;
   let _ = v0 as u16; // expect to abort
}
}

// -----------------------------
// Abort cases: values above 65535
// -----------------------------

//# run --verbose
script {
fun main() {
   let v0 = 65536i32;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 65536i64;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 65536i128;
   let _ = v0 as u16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 65536i256;
   let _ = v0 as u16; // expect to abort
}
}
