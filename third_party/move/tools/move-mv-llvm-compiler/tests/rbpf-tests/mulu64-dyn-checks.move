// abort 4017

module 0x101::Test1 {
  public fun test_mulu64(a: u64, b: u64): u64 {
    let c = a * b;
    c
  }
}

script {
  fun main() {
    let a: u64 = 9223372036854775807;
    assert!(0x101::Test1::test_mulu64(a, 2) == 18446744073709551614, 10);  // Ok: no overflow.

    0x101::Test1::test_mulu64(a, 3);  // Abort: overflow.
  }
}
