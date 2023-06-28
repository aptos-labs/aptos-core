module 0x100::Test {
  fun test_or(a: u8, b: u8): u8 {
    let c = a | b;
    c
  }
  fun test_and(a: u8, b: u8): u8 {
    let c = a & b;
    c
  }
  fun test_xor(a: u8, b: u8): u8 {
    let c = a ^ b;
    c
  }
  fun test_shl8(a: u8, b: u8): u8 {
    let c = a << b;
    c
  }
  fun test_shr8(a: u8, b: u8): u8 {
    let c = a >> b;
    c
  }
  fun test_shl32(a: u32, b: u8): u32 {
    let c = a << b;
    c
  }
  fun test_shr32(a: u32, b: u8): u32 {
    let c = a >> b;
    c
  }
  fun test_shl64(a: u64, b: u8): u64 {
    let c = a << b;
    c
  }
  fun test_shr64(a: u64, b: u8): u64 {
    let c = a >> b;
    c
  }
  fun test_shl128(a: u128, b: u8): u128 {
    let c = a << b;
    c
  }
  fun test_shr128(a: u128, b: u8): u128 {
    let c = a >> b;
    c
  }
}
