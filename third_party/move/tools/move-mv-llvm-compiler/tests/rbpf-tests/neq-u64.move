//
module 0x101::Test1 {
  public fun test_neq_u64(a: u64, b: u64): bool {
    a != b
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_neq_u64(18446744073709551615u64, 18446744073709551614u64), 10);
    assert!(!0x101::Test1::test_neq_u64(18446744073709551615u64, 18446744073709551615u64), 10);
  }
}
