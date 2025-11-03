//# run --verbose
script {
fun main() {
   let v0: i8 = 0i8;
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
   assert!((v1 as i8) == 0i8);
   assert!((v2 as i8) == 0i8);
   assert!((v3 as i8) == 0i8);
   assert!((v4 as i8) == 0i8);
   assert!((v5 as i8) == 0i8);
   assert!((v6 as i8) == 0i8);
   assert!((v7 as i8) == 0i8);
   assert!((v8 as i8) == 0i8);
   assert!((v9 as i8) == 0i8);
   assert!((v10 as i8) == 0i8);
   assert!((v11 as i8) == 0i8);
   assert!((v12 as i8) == 0i8);
}
}

//# run --verbose
script {
fun main() {
   let v0: i8 = 127i8;
   assert!((v0 as u8) == 127u8);
   assert!((v0 as u16) == 127u16);
   assert!((v0 as u32) == 127u32);
   assert!((v0 as u64) == 127u64);
   assert!((v0 as u128) == 127u128);
   assert!((v0 as u256) == 127u256);
   assert!((v0 as i8) == 127i8);
   assert!((v0 as i16) == 127i16);
   assert!((v0 as i32) == 127i32);
   assert!((v0 as i64) == 127i64);
   assert!((v0 as i128) == 127i128);
   assert!((v0 as i256) == 127i256);

   let v1: u8 = 127u8;
   let v2: u16 = 127u16;
   let v3: u32 = 127u32;
   let v4: u64 = 127u64;
   let v5: u128 = 127u128;
   let v6: u256 = 127u256;
   let v7: i8 = 127i8;
   let v8: i16 = 127i16;
   let v9: i32 = 127i32;
   let v10: i64 = 127i64;
   let v11: i128 = 127i128;
   let v12: i256 = 127i256;
   assert!((v1 as i8) == 127i8);
   assert!((v2 as i8) == 127i8);
   assert!((v3 as i8) == 127i8);
   assert!((v4 as i8) == 127i8);
   assert!((v5 as i8) == 127i8);
   assert!((v6 as i8) == 127i8);
   assert!((v7 as i8) == 127i8);
   assert!((v8 as i8) == 127i8);
   assert!((v9 as i8) == 127i8);
   assert!((v10 as i8) == 127i8);
   assert!((v11 as i8) == 127i8);
   assert!((v12 as i8) == 127i8);
}
}

//# run --verbose
script {
fun main() {
   let v0: i8 = -128i8;
   assert!((v0 as i8) == -128i8);
   assert!((v0 as i16) == -128i16);
   assert!((v0 as i32) == -128i32);
   assert!((v0 as i64) == -128i64);
   assert!((v0 as i128) == -128i128);
   assert!((v0 as i256) == -128i256);

   let v7 = -128i8;
   let v8 = -128i16;
   let v9 = -128i32;
   let v10 = -128i64;
   let v11 = -128i128;
   let v12 = -128i256;
   assert!((v7 as i8) == -128i8);
   assert!((v8 as i8) == -128i8);
   assert!((v9 as i8) == -128i8);
   assert!((v10 as i8) == -128i8);
   assert!((v11 as i8) == -128i8);
   assert!((v12 as i8) == -128i8);
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u8;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u16;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u32;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u64;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u128;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128u256;
   let v2 = v0 as i8; // expect to abort
}
}

// Signed types beyond i8::MAX (127)

//# run --verbose
script {
fun main() {
   let v0 = 128i16;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128i32;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128i64;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128i128;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 128i256;
   let v2 = v0 as i8; // expect to abort
}
}

// Signed types below i8::MIN (-128)

//# run --verbose
script {
fun main() {
   let v0 = -129i16;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -129i32;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -129i64;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -129i128;
   let v2 = v0 as i8; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -129i256;
   let v2 = v0 as i8; // expect to abort
}
}
