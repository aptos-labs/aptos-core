// abort 4017

module 0x101::Test1 {
  public fun test_castu64(a: u256): u64 {
    (a as u64)
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_castu64(18446744073709551615u256) == 18446744073709551615, 10);  // Ok: source fits in dest.

    0x101::Test1::test_castu64(18446744073709551616u256);  // Abort: source too big.
  }
}
