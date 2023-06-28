module 0x100::Test {
  fun cast_u8(a: u32): u8 {
    let c = (a as u8);
    c
  }
  fun cast_u32(a: u8): u32 {
    let c = (a as u32);
    c
  }
  fun cast_u64(a: u8): u64 {
    let c = (a as u64);
    c
  }
}
