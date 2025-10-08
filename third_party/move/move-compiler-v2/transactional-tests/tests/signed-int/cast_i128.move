//# run --verbose
script {
fun main() {
   let v0: i128 = 0i128;
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
   assert!((v1 as i128) == 0i128);
   assert!((v2 as i128) == 0i128);
   assert!((v3 as i128) == 0i128);
   assert!((v4 as i128) == 0i128);
   assert!((v5 as i128) == 0i128);
   assert!((v6 as i128) == 0i128);
   assert!((v7 as i128) == 0i128);
   assert!((v8 as i128) == 0i128);
   assert!((v9 as i128) == 0i128);
   assert!((v10 as i128) == 0i128);
   assert!((v11 as i128) == 0i128);
   assert!((v12 as i128) == 0i128);
}
}

//# run --verbose
script {
fun main() {
   let v0: i128 = 170141183460469231731687303715884105727i128; // i128::MAX
   assert!((v0 as u128) == 170141183460469231731687303715884105727u128);
   assert!((v0 as u256) == 170141183460469231731687303715884105727u256);
   assert!((v0 as i128) == 170141183460469231731687303715884105727i128);
   assert!((v0 as i256) == 170141183460469231731687303715884105727i256);

   let v5: u128 = 170141183460469231731687303715884105727u128;
   let v6: u256 = 170141183460469231731687303715884105727u256;
   let v7: i128 = 170141183460469231731687303715884105727i128;
   let v8: i256 = 170141183460469231731687303715884105727i256;
   assert!((v5 as i128) == 170141183460469231731687303715884105727i128);
   assert!((v6 as i128) == 170141183460469231731687303715884105727i128);
   assert!((v7 as i128) == 170141183460469231731687303715884105727i128);
   assert!((v8 as i128) == 170141183460469231731687303715884105727i128);
}
}

//# run --verbose
script {
fun main() {
   let v0: i128 = -170141183460469231731687303715884105728i128; // i128::MIN
   assert!((v0 as i128) == -170141183460469231731687303715884105728i128);
   assert!((v0 as i256) == -170141183460469231731687303715884105728i256);

   let v7 = -170141183460469231731687303715884105728i128;
   let v8 = -170141183460469231731687303715884105728i256;
   assert!((v7 as i128) == -170141183460469231731687303715884105728i128);
   assert!((v8 as i128) == -170141183460469231731687303715884105728i128);
}
}

// Unsigned → i128 (overflow when ≥ 170141183460469231731687303715884105728)

//# run --verbose
script {
fun main() {
   let v0 = 170141183460469231731687303715884105728u128;
   let v2 = v0 as i128; // expect to abort
}
}

//# run --verbose
script {
fun main() {
   let v0 = 170141183460469231731687303715884105728u256;
   let v2 = v0 as i128; // expect to abort
}
}

// Signed → i128 (overflow above i128::MAX)

//# run --verbose
script {
fun main() {
   let v0 = 170141183460469231731687303715884105728i256;
   let v2 = v0 as i128; // expect to abort
}
}

// Signed → i128 (overflow below i128::MIN)

//# run --verbose
script {
fun main() {
   let v0 = -170141183460469231731687303715884105729i256;
   let v2 = v0 as i128; // expect to abort
}
}
