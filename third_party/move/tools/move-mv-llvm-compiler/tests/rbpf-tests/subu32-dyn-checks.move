// abort 4017

module 0x101::Test1 {
  public fun test_subu32(a: u32, b: u32): u32 {
    let c = a - b;
    c
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_subu32(1, 1) == 0, 10);  // Ok: no overflow.

    0x101::Test1::test_subu32(0, 1);  // Abort: overflow.
  }
}
