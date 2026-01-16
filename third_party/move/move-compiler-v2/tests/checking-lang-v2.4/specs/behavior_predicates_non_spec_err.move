// Tests that behavior predicates are only allowed in spec context.

module 0x42::M {

    fun apply(f: |u64| u64, x: u64): u64 {
        // Error: behavior predicates only allowed in specifications
        if (requires_of<f>(x)) {
            f(x)
        } else {
            0
        }
    }
}
