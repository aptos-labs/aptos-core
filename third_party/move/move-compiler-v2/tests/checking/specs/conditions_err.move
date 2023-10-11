module 0x42::M {

  fun add_some(x: &mut u64): u64 { *x = *x + 1; *x }

  spec add_some {
    aborts_if x; // Type of condition not bool.
    ensures old(x) + x; // Type of condition not bool.
    ensures result_1 == 0; // Using result which does not exist.
  }

  fun with_emits<T: drop>(_guid: vector<u8>, _msg: T, x: u64): u64 { x }

  spec with_emits {
    emits _msg to _guid if x; // Type of condition for "if" is not bool.
  }
}
