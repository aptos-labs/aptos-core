// `debug_assert!`, `debug_assert_eq!`, and `debug_assert_ne!` require Move 2.5.
// Under a lower language version each must produce a language-version gate
// diagnostic; the check fires regardless of `--compile-test-code`.
module 0x42::m {
    public fun gated() {
        debug_assert!(true);
    }

    public fun gated_eq() {
        debug_assert_eq!(true, true);
    }

    public fun gated_ne() {
        debug_assert_ne!(true, false);
    }
}
