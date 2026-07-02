// Arity errors for `debug_assert_eq!` and `debug_assert_ne!`. These errors fire
// before macro expansion completes, so the diagnostics quote the actual macro
// name. In non-test mode the macros expand silently to `()` and no arity error
// is produced (see paired `no_test.exp`).
module 0x42::m {
    public fun too_few_eq<T: drop>(x: T) {
        debug_assert_eq!(x);
    }

    public fun too_few_ne<T: drop>(x: T) {
        debug_assert_ne!(x);
    }

    public fun too_many_eq<T: copy + drop>(left: T, right: T) {
        debug_assert_eq!(left, right, b"x", left, right, left, right, left);
    }

    public fun too_many_ne<T: copy + drop>(left: T, right: T) {
        debug_assert_ne!(left, right, b"x", left, right, left, right, left);
    }
}
