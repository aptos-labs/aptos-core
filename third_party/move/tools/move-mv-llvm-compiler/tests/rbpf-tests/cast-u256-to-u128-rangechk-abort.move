// abort 4017

module 0x101::Test1 {
  public fun test_castu128(a: u256): u128 {
    (a as u128)
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_castu128(340282366920938463463374607431768211455u256) == 340282366920938463463374607431768211455, 10);  // Ok: source fits in dest.

    0x101::Test1::test_castu128(340282366920938463463374607431768211456u256);  // Abort: source too big.
  }
}
