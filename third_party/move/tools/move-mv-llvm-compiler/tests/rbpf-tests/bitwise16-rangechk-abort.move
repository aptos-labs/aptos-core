// abort 4017

module 0x101::Test1 {
  public fun test_shlu16(a: u16, b: u8): u16 {
    let c = a << b;
    c
  }
  public fun test_shru16(a: u16, b: u8): u16 {
    let c = a >> b;
    c
  }
}

script {
  fun main() {
    let a: u16 = 1;
    let b: u8 = 15;
    assert!(0x101::Test1::test_shlu16(a, b) == 0x8000, 20);  // Ok: count in range.
    0x101::Test1::test_shru16(a, 16);  // Abort: count out of range.
  }
}
