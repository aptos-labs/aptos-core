// abort 4017

module 0x101::Test1 {
  public fun test_mulu16(a: u16, b: u16): u16 {
    let c = a * b;
    c
  }
}

script {
  fun main() {
    let a: u16 = 32767;
    assert!(0x101::Test1::test_mulu16(a, 2) == 65534, 10);  // Ok: no overflow.

    0x101::Test1::test_mulu16(a, 3);  // Abort: overflow.
  }
}
