
module 0x101::Test1 {
  public fun test_shlu256(a: u256, b: u8): u256 {
    let c = a << b;
    c
  }
  public fun test_shru256(a: u256, b: u8): u256 {
    let c = a >> b;
    c
  }
}

script {
  fun main() {
    let a: u256 = 1;
    let b: u8 = 255;
    assert!(0x101::Test1::test_shlu256(a, b) == 0x8000000000000000000000000000000000000000000000000000000000000000, 20);  // Ok: count in range.

    // u256 shift count always legal today in Move since count is restricted to u8.
  }
}
