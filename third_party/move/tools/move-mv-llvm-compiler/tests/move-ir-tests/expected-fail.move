// xfail: no expected.ll file: "tests/move-ir-tests/expected-fail-build/modules/0_Test.expected.ll"
module 0x100::Test {
  fun test() {
    abort 10
  }
}
