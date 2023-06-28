// abort 4017

module 0x101::Test1 {
  public fun test_addu8(a: u8, b: u8): u8 {
    let c = a + b;
    c
  }
}

script {
  fun main() {
    let a: u8 = 253;
    assert!(0x101::Test1::test_addu8(a, 1) == 254, 10);  // Ok: no overflow.

    0x101::Test1::test_addu8(a, 3);  // Abort: overflow.
  }
}
