// abort 4017

module 0x101::Test1 {
  public fun test_shlu64(a: u64, b: u8): u64 {
    let c = a << b;
    c
  }
  public fun test_shru64(a: u64, b: u8): u64 {
    let c = a >> b;
    c
  }
}

script {
  fun main() {
    let a: u64 = 1;
    let b: u8 = 63;
    assert!(0x101::Test1::test_shlu64(a, b) == 0x8000000000000000, 20);  // Ok: count in range.
    0x101::Test1::test_shru64(a, 64);  // Abort: count out of range.
  }
}
