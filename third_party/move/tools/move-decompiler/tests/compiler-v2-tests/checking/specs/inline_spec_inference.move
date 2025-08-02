module 0x42::m {
  fun foo() {
    let i = 10;
    spec {
      assert forall j in 0..i: j < i;
    };
    i = i + 1
  }
}
