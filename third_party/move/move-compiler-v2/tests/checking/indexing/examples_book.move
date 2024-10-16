module 0x1::m {

  struct R has key, drop { value: bool }

  fun f1() acquires R {
    let x = &mut R[@0x1];
    x.value = false;
    assert!(R[@0x1].value == false, 1);
    R[@0x1].value = true;
    assert!(R[@0x1].value == true, 2);
  }
}
