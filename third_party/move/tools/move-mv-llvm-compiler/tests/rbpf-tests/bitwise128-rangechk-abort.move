// abort 4017

module 0x101::Test1 {
  public fun test_shlu128(a: u128, b: u8): u128 {
    let c = a << b;
    c
  }
  public fun test_shru128(a: u128, b: u8): u128 {
    let c = a >> b;
    c
  }
}

script {
  fun main() {
    let a: u128 = 1;
    let b: u8 = 127;
    assert!(0x101::Test1::test_shlu128(a, b) == 0x80000000000000000000000000000000, 20);  // Ok: count in range.
    0x101::Test1::test_shru128(a, 128);  // Abort: count out of range.
  }
}
