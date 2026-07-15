// Basic forms of `debug_assert_eq!` and `debug_assert_ne!`. In test mode each
// expands like the corresponding `assert_eq!` / `assert_ne!` and references
// `string_utils::format2`; in non-test mode the macros expand to `()` without
// touching the operands (see paired `no_test.exp`).
module aptos_std::string_utils {
    use std::string::String;

    public fun format2<T0: drop, T1: drop>(_fmt: &vector<u8>, _a: T0, _b: T1): String {
        abort 0
    }
}

module 0x42::m {
    public fun check_eq<T: copy + drop>(left: T, right: T) {
        debug_assert_eq!(left, right);
    }

    public fun check_ne<T: copy + drop>(left: T, right: T) {
        debug_assert_ne!(left, right);
    }
}
