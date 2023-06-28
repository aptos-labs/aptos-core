// abort 4017

module 0x101::Test1 {
  public fun test_addu64(a: u64, b: u64): u64 {
    let c = a + b;
    c
  }
}

script {
  fun main() {
    let a: u64 = 18446744073709551614; // UMAX-2
    assert!(0x101::Test1::test_addu64(a, 1) == 18446744073709551615, 10);  // Ok: no overflow.

    0x101::Test1::test_addu64(a, 3);  // Abort: overflow.
  }
}
