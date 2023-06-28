module 0x100::Test {
  fun test(a: u128, b: u128): u128 {
    let c = a + b;
    c
  }
  fun test_sub(a: u128, b: u128): u128 {
    let c = a - b;
    c
  }
  fun test_mul(a: u128, b: u128): u128 {
    let c = a * b;
    c
  }
  fun test_div(a: u128, b: u128): u128 {
    let c = a / b;
    c
  }
  fun test_mod(a: u128, b: u128): u128 {
    let c = a % b;
    c
  }
}
