module 0x100::Test {
  fun test(a: u64, b: u64): u64 {
    let c = a + b;
    c
  }
  fun test_sub(a: u64, b: u64): u64 {
    let c = a - b;
    c
  }
  fun test_mul(a: u64, b: u64): u64 {
    let c = a * b;
    c
  }
  fun test_div(a: u64, b: u64): u64 {
    let c = a / b;
    c
  }
}
