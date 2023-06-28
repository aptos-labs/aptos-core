//
module 0x101::Test1 {
  public fun test_neq_u32(a: u32, b: u32): bool {
    a != b
  }
}

script {
  fun main() {
    assert!(0x101::Test1::test_neq_u32(4294967295u32, 4294967294u32), 10);
    assert!(!0x101::Test1::test_neq_u32(4294967295u32, 4294967295u32), 10);
  }
}
