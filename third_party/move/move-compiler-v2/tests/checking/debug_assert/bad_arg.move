// In non-test mode the macro arguments are not type-checked or name-resolved:
// `not_defined` must trigger a diagnostic in test mode and be silently dropped
// in non-test mode (see paired `no_test.exp`).
module 0x42::m {
    public fun bad(x: u64) {
        debug_assert!(x == not_defined);
    }
}
