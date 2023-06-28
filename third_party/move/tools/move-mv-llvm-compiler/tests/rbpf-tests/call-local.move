// regression-test for incorrectly double-declaring function `a` here.

module 0x101::foo {
  public fun a(): u8 {
    1
  }

  public fun test_sub(a: u8, b: u8): u8 {
    a-b
  }

  public fun b(): u8 {
    a()
  }
}

script {
  use 0x101::foo;

  fun main() {
    let v = foo::a();
    assert!(v == 1, 11);
    v = foo::test_sub(10, 3);
    assert!(v == 7, 12);
  }
}
