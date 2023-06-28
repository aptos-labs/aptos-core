// abort 4017

module 0x101::Test1 {
  public fun test_addu128(a: u128, b: u128): u128 {
    let c = a + b;
    c
  }
}

script {
  fun main() {
    let a: u128 = 340282366920938463463374607431768211454; // UMAX-2
    assert!(0x101::Test1::test_addu128(a, 1) == 340282366920938463463374607431768211455, 10);  // Ok: no overflow.

    0x101::Test1::test_addu128(a, 3);  // Abort: overflow.
  }
}
