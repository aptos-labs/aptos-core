// abort 4017

module 0x101::Test1 {
  public fun test_subu64(a: u64, b: u64): u64 {
    let c = a - b;
    c
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_subu64(1, 1) == 0, 10);  // Ok: no overflow.

    0x101::Test1::test_subu64(0, 1);  // Abort: overflow.
  }
}
