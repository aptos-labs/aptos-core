// regression-test for incorrectly double-declaring function `a` here.

module 0x101::foo {
  public fun a(): u8 {
    1
  }
}

module 0x102::bar {
  use 0x101::foo;

  public fun b(): u8 {
    foo::a()
  }

  public fun c(): u8 {
    foo::a()
  }
}

script {
  use 0x102::bar;

  fun main() {
    let v = bar::c();
    assert!(v == 1, 11);
  }
}
