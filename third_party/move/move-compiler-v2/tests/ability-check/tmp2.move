module 0x42::test {
  struct R {}

  fun foo() {
    let x = R {};
    if (true) {
      x;
    } else {

    }
  }
}
