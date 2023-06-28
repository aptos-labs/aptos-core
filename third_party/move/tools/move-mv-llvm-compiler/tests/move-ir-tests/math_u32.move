module 0x100::Test {
  fun test(a: u32, b: u32): u32 {
    let c = a + b;
    c
  }
  fun test_sub(a: u32, b: u32): u32 {
    let c = a - b;
    c
  }
  fun test_mul_trunc(a: u32, b: u32): u32 {
    let c = a * b;
    c
  }
  fun test_div(a: u32, b: u32): u32 {
    let c = a / b;
    c
  }
  /* FIXME: Implement cast
  fun test_mul(a: u32, b: u32): u64 {
    let c = (a as u64);
    let d = (b as u64);
    let e = c * d;
    c
  }*/
}
