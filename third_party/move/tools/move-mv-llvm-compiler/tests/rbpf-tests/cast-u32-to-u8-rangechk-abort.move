// abort 4017

module 0x101::Test1 {
  public fun test_castu8(a: u32): u8 {
    (a as u8)
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_castu8(255u32) == 255, 10);  // Ok: source fits in dest.

    0x101::Test1::test_castu8(256u32);  // Abort: source too big.
  }
}
