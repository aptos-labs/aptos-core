// abort 4017

module 0x101::Test1 {
  public fun test_mulu8(a: u8, b: u8): u8 {
    let c = a * b;
    c
  }
}

script {
  fun main() {
    let a: u8 = 127;
    assert!(0x101::Test1::test_mulu8(a, 2) == 254, 10);  // Ok: no overflow.

    0x101::Test1::test_mulu8(a, 3);  // Abort: overflow.
  }
}
