// Tests for behavior predicate parsing errors.

module 0x42::M {

    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }

    spec apply {
        // Test missing closing angle bracket - f( is not a valid name
        ensures requires_of<f(x, y);
    }
}
