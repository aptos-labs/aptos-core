// abort 4017

module 0x101::Test1 {
  public fun test_mulu128(a: u128, b: u128): u128 {
    let c = a * b;
    c
  }
}

script {
  fun main() {
    let a: u128 = 170141183460469231731687303715884105727;
    assert!(0x101::Test1::test_mulu128(a, 2) == 340282366920938463463374607431768211454, 10);  // Ok: no overflow.

    0x101::Test1::test_mulu128(a, 3);  // Abort: overflow.
  }
}
