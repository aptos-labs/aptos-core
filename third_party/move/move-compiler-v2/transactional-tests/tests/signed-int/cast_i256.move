//# run --verbose
script {
fun main() {
   let v0: i256 = 0i256;
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
   assert!((v1 as i256) == 0i256);
   assert!((v2 as i256) == 0i256);
   assert!((v3 as i256) == 0i256);
   assert!((v4 as i256) == 0i256);
   assert!((v5 as i256) == 0i256);
   assert!((v6 as i256) == 0i256);
   assert!((v7 as i256) == 0i256);
   assert!((v8 as i256) == 0i256);
   assert!((v9 as i256) == 0i256);
   assert!((v10 as i256) == 0i256);
   assert!((v11 as i256) == 0i256);
   assert!((v12 as i256) == 0i256);
}
}

//# run --verbose
script {
fun main() {
   let v0: i256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967i256;
   assert!((v0 as u256) == 57896044618658097711785492504343953926634992332820282019728792003956564819967u256);

   let v6: u256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967u256;
   assert!((v6 as i256) == 57896044618658097711785492504343953926634992332820282019728792003956564819967i256);
}
}

//# run --verbose
script {
fun main() {
   let v0: i256 = -57896044618658097711785492504343953926634992332820282019728792003956564819968i256;
   assert!((v0 as i256) == -57896044618658097711785492504343953926634992332820282019728792003956564819968i256);
}
}

//# run --verbose
script {
fun main() {
   let v0 = 57896044618658097711785492504343953926634992332820282019728792003956564819968u256; // one above i256::MAX
   let v2 = v0 as i256; // expect to abort
}
}
