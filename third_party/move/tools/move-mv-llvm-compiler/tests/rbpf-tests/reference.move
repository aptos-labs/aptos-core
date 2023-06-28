module 0x101::foo {
  public fun deref(v: &u8): u8 {
    *v
  }

  public fun assign(v: &mut u8) {
    *v = 11;
  }

  public fun freeze_(v: &mut u8): u8 {
    deref(freeze(v))
  }
}

script {
  use 0x101::foo;

  fun main() {
    let v = foo::deref(&10);
    assert!(v == 10, 1);

    foo::assign(&mut v);
    assert!(v == 11, 1);

    let v2 = foo::freeze_(&mut v);
    assert!(v2 == 11, 1);
  }
}
