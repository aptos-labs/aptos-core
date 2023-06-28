// Check that large constants are created correctly.

module 0x100::Test {
  fun takes_u128(a: u128): u128 {
    a
  }
  fun test_const_u128(): u128 {
    let u1: u128 = 7;
    takes_u128(u1);
    let u2: u128 = 1 << 32;
    takes_u128(u2);
    let u3: u128 = 1 << 64;
    takes_u128(u3);
    let u4: u128 = 1 << 127;
    takes_u128(u4)
  }
}
