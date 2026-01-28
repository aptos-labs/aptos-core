// Mock implementation of the formatting functions used in the macros.
module aptos_std::string_utils {
    use std::string::String;

    public fun format1<T0: drop>(_fmt: &vector<u8>, _a: T0): String {
        abort 0
    }

    public fun format2<T0: drop, T1: drop>(_fmt: &vector<u8>, _a: T0, _b: T1): String {
        abort 0
    }

    public fun format3<T0: drop, T1: drop, T2: drop>(_fmt: &vector<u8>, _a: T0, _b: T1, _c: T2): String {
        abort 0
    }

    public fun format4<T0: drop, T1: drop, T2: drop, T3: drop>(_fmt: &vector<u8>, _a: T0, _b: T1, _c: T2, _d: T3): String {
        abort 0
    }
}

module 0x42::M {
    fun no_arguments() {
        assert!()
    }

    fun too_many_arguments<T0: drop, T1: drop, T2: drop, T3: drop, T4: drop>(cond: bool, a: T0, b: T1, c: T2, d: T3, e: T4) {
        assert!(cond, b"a = {}, b = {}, c = {}, d = {}, e = {}", a, b, c, d, e)
    }

    fun not_string_literal<T0: drop>(cond: bool, a: T0) {
        let fmt = b"a = {}";
        assert!(cond, fmt, a)
    }

    fun wrong_type_argument<T>(cond: bool, x: T) {
        assert!(cond, x)
    }

    fun unmatched_opening_brace<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {", a);
    }

    fun unmatched_closing_brace<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = }", a);
    }

    fun invalid_placeholder<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {a}", a);
    }

    fun too_many_format_arguments<T0: drop, T1: drop>(cond: bool, a: T0, b: T1) {
        assert!(cond, b"a = {}", a, b);
    }

    fun too_few_format_arguments<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {}, b = {}", a);
    }

    fun placeholder_with_no_arguments(cond: bool) {
        assert!(cond, b"{}");
    }

    fun extra_opening_brace<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {{}", a);
    }

    fun extra_closing_brace<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {}}", a);
    }

    fun escaped_braces_args_mismatch<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {{}}", a);
    }
}
