// abort 4017

module 0x101::Test1 {
  public fun test_castu32(a: u128): u32 {
    (a as u32)
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_castu32(4294967295u128) == 4294967295, 10);  // Ok: source fits in dest.

    0x101::Test1::test_castu32(4294967296u128);  // Abort: source too big.
  }
}
