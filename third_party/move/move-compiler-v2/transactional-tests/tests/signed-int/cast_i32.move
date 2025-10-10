//# run --verbose
script {
fun main() {
   let v0: i32 = 0i32;
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
   let v7: i32 = 0i32;
   let v8: i64 = 0i64;
   let v9: i128 = 0i128;
   let v10: i256 = 0i256;
   assert!((v1 as i32) == 0i32);
   assert!((v2 as i32) == 0i32);
   assert!((v3 as i32) == 0i32);
   assert!((v4 as i32) == 0i32);
   assert!((v5 as i32) == 0i32);
   assert!((v6 as i32) == 0i32);
   assert!((v7 as i32) == 0i32);
   assert!((v8 as i32) == 0i32);
   assert!((v9 as i32) == 0i32);
   assert!((v10 as i32) == 0i32);
}
}

//# run --verbose
script {
fun main() {
   let v0: i32 = 2147483647i32;
   assert!((v0 as u32) == 2147483647u32);
   assert!((v0 as u64) == 2147483647u64);
   assert!((v0 as u128) == 2147483647u128);
   assert!((v0 as u256) == 2147483647u256);
   assert!((v0 as i32) == 2147483647i32);
   assert!((v0 as i64) == 2147483647i64);
   assert!((v0 as i128) == 2147483647i128);
   assert!((v0 as i256) == 2147483647i256);

   let v3: u32 = 2147483647u32;
   let v4: u64 = 2147483647u64;
   let v5: u128 = 2147483647u128;
   let v6: u256 = 2147483647u256;
   let v7: i32 = 2147483647i32;
   let v8: i64 = 2147483647i64;
   let v9: i128 = 2147483647i128;
   let v10: i256 = 2147483647i256;
   assert!((v3 as i32) == 2147483647i32);
   assert!((v4 as i32) == 2147483647i32);
   assert!((v5 as i32) == 2147483647i32);
   assert!((v6 as i32) == 2147483647i32);
   assert!((v7 as i32) == 2147483647i32);
   assert!((v8 as i32) == 2147483647i32);
   assert!((v9 as i32) == 2147483647i32);
   assert!((v10 as i32) == 2147483647i32);
}
}

//# run --verbose
script {
fun main() {
   let v0: i32 = -2147483648i32;
   assert!((v0 as i32) == -2147483648i32);
   assert!((v0 as i64) == -2147483648i64);
   assert!((v0 as i128) == -2147483648i128);
   assert!((v0 as i256) == -2147483648i256);

   let v7 = -2147483648i32;
   let v8 = -2147483648i64;
   let v9 = -2147483648i128;
   let v10 = -2147483648i256;
   assert!((v7 as i32) == -2147483648i32);
   assert!((v8 as i32) == -2147483648i32);
   assert!((v9 as i32) == -2147483648i32);
   assert!((v10 as i32) == -2147483648i32);
}
}

// Unsigned → i32 (overflow when ≥ 2_147_483_648)

//# run --verbose
script {
fun main() {
   let v0 = 2147483648u32;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 2147483648u64;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 2147483648u128;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 2147483648u256;
   let v2 = v0 as i32; // expect to abort
}
}

// Signed → i32 (overflow above 2_147_483_647)

//# run --verbose
script {
fun main() {
   let v0 = 2147483648i64;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 2147483648i128;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 2147483648i256;
   let v2 = v0 as i32; // expect to abort
}
}

// Signed → i32 (overflow below -2_147_483_648)

//# run --verbose
script {
fun main() {
   let v0 = -2147483649i64;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -2147483649i128;
   let v2 = v0 as i32; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = -2147483649i256;
   let v2 = v0 as i32; // expect to abort
}
}
