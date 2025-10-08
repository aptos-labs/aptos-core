//# run --verbose
script {
fun main() {
   let v0: i16 = 0i16;
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
   assert!((v1 as i16) == 0i16);
   assert!((v2 as i16) == 0i16);
   assert!((v3 as i16) == 0i16);
   assert!((v4 as i16) == 0i16);
   assert!((v5 as i16) == 0i16);
   assert!((v6 as i16) == 0i16);
   assert!((v7 as i16) == 0i16);
   assert!((v8 as i16) == 0i16);
   assert!((v9 as i16) == 0i16);
   assert!((v10 as i16) == 0i16);
   assert!((v11 as i16) == 0i16);
   assert!((v12 as i16) == 0i16);
}
}

//# run --verbose
script {
fun main() {
   let v0: i16 = 32767i16;
   assert!((v0 as u16) == 32767u16);
   assert!((v0 as u32) == 32767u32);
   assert!((v0 as u64) == 32767u64);
   assert!((v0 as u128) == 32767u128);
   assert!((v0 as u256) == 32767u256);
   assert!((v0 as i16) == 32767i16);
   assert!((v0 as i32) == 32767i32);
   assert!((v0 as i64) == 32767i64);
   assert!((v0 as i128) == 32767i128);
   assert!((v0 as i256) == 32767i256);

   let v2: u16 = 32767u16;
   let v3: u32 = 32767u32;
   let v4: u64 = 32767u64;
   let v5: u128 = 32767u128;
   let v6: u256 = 32767u256;
   let v7: i16 = 32767i16;
   let v8: i32 = 32767i32;
   let v9: i64 = 32767i64;
   let v10: i128 = 32767i128;
   let v11: i256 = 32767i256;
   assert!((v2 as i16) == 32767i16);
   assert!((v3 as i16) == 32767i16);
   assert!((v4 as i16) == 32767i16);
   assert!((v5 as i16) == 32767i16);
   assert!((v6 as i16) == 32767i16);
   assert!((v7 as i16) == 32767i16);
   assert!((v8 as i16) == 32767i16);
   assert!((v9 as i16) == 32767i16);
   assert!((v10 as i16) == 32767i16);
   assert!((v11 as i16) == 32767i16);
}
}

//# run --verbose
script {
fun main() {
   let v0: i16 = -32768i16;
   assert!((v0 as i16) == -32768i16);
   assert!((v0 as i32) == -32768i32);
   assert!((v0 as i64) == -32768i64);
   assert!((v0 as i128) == -32768i128);
   assert!((v0 as i256) == -32768i256);

   let v7 = -32768i16;
   let v8 = -32768i32;
   let v9 = -32768i64;
   let v10 = -32768i128;
   let v11 = -32768i256;
   assert!((v7 as i16) == -32768i16);
   assert!((v8 as i16) == -32768i16);
   assert!((v9 as i16) == -32768i16);
   assert!((v10 as i16) == -32768i16);
   assert!((v11 as i16) == -32768i16);
}
}

// Unsigned → i16 (values ≥ 32768 overflow)

//# run --verbose
script {
fun main() {
   let v0 = 32768u16;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768u32;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768u64;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768u128;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768u256;
   let v2 = v0 as i16; // expect to abort
}
}

// Signed → i16, overflow above 32767

//# run --verbose
script {
fun main() {
   let v0 = 32768i32;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768i64;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768i128;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 32768i256;
   let v2 = v0 as i16; // expect to abort
}
}

// Signed → i16, overflow below -32768

//# run --verbose
script {
fun main() {
   let v0 = -32769i32;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -32769i64;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -32769i128;
   let v2 = v0 as i16; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -32769i256;
   let v2 = v0 as i16; // expect to abort
}
}
