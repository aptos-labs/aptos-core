// `debug_assert!` requires Move 2.5. Under a lower language version the
// macro must produce a language-version gate diagnostic; the check fires
// regardless of `--compile-test-code`.
module 0x42::m {
    public fun gated() {
        debug_assert!(true);
    }
}
