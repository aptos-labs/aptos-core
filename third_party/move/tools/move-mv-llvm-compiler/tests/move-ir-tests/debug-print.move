module 0x10::debug {
  native public fun print<T>(x: &T);
}

script {
  use 0x10::debug;

  fun main() {
    debug::print(&10);
  }
}
