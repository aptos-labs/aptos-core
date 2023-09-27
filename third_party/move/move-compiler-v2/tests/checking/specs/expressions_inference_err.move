// Inference errors may only be reported if all else succeeds, so we put them in a different file.

module 0x42::M {
  spec module {
    fun incomplete_types(): u64 {
      let f = |x|x; // Incomplete types.
      0
    }
  }
}
