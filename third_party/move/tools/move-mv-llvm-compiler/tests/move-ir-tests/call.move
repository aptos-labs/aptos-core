module 0x100::Test {
  fun get_sub(a: u8, b: u8): u8 {
    let c = a - b;
    c
  }

  fun test() {
    let r = get_sub(10, 3);
    assert!(7 == r, 10);
  }
}
