// abort 4017

module 0x101::Test1 {
  public fun test_addu32(a: u32, b: u32): u32 {
    let c = a + b;
    c
  }
}

script {
  fun main() {
    let a: u32 = 4294967294;  // UMAX-2.
    assert!(0x101::Test1::test_addu32(a, 1) == 4294967295, 10);  // Ok: no overflow.

    0x101::Test1::test_addu32(a, 3);  // Abort: overflow.
  }
}
