// abort 4017

module 0x101::Test1 {
  public fun test_shlu32(a: u32, b: u8): u32 {
    let c = a << b;
    c
  }
  public fun test_shru32(a: u32, b: u8): u32 {
    let c = a >> b;
    c
  }
}

script {
  fun main() {
    let a: u32 = 1;
    let b: u8 = 31;
    assert!(0x101::Test1::test_shlu32(a, b) == 0x80000000, 20);  // Ok: count in range.
    0x101::Test1::test_shru32(a, 32);  // Abort: count out of range.
  }
}
