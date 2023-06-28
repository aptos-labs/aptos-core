// abort 4017

module 0x101::Test1 {
  public fun test_subu128(a: u128, b: u128): u128 {
    let c = a - b;
    c
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_subu128(1, 1) == 0, 10);  // Ok: no overflow.

    0x101::Test1::test_subu128(0, 1);  // Abort: overflow.
  }
}
