module 0x100::Test {
  fun test_add(a: u8, b: u8): u8 {
    let c = a + b;
    c
  }
  fun test_sub(a: u8, b: u8): u8 {
    let c = a - b;
    c
  }
  fun test_mul(a: u8, b: u8): u8 {
    let c = a * b;
    c
  }
  fun test_div(a: u8, b: u8): u8 {
    let c = a / b;
    c
  }
  fun test_mod(a: u8, b: u8): u8 {
    let c = a % b;
    c
  }
}
