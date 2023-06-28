// abort 4017

module 0x101::Test1 {
  public fun test_subu8(a: u8, b: u8): u8 {
    let c = a - b;
    c
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_subu8(1, 1) == 0, 10);  // Ok: no overflow.

    0x101::Test1::test_subu8(0, 1);  // Abort: overflow.
  }
}
