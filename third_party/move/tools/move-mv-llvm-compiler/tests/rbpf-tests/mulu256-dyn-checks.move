// abort 4017

module 0x101::Test1 {
  public fun test_mulu256(a: u256, b: u256): u256 {
    let c = a * b;
    c
  }
}

script {
  fun main() {
    let a: u256 = 57896044618658097711785492504343953926634992332820282019728792003956564819967;
    assert!(0x101::Test1::test_mulu256(a, 2) == 115792089237316195423570985008687907853269984665640564039457584007913129639934, 10);  // Ok: no overflow.

    0x101::Test1::test_mulu256(a, 3);  // Abort: overflow.
  }
}
