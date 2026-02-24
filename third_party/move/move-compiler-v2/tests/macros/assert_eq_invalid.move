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
        assert_eq!()
    }

    fun single_argument<T: drop>(x: T) {
        assert_eq!(x)
    }

    fun too_many_arguments<T: drop, T0: drop, T1: drop, T2: drop, T3: drop, T4: drop>(left: T, right: T, a: T0, b: T1, c: T2, d: T3, e: T4) {
        assert_eq!(left, right, b"a = {}, b = {}, c = {}, d = {}, e = {}", a, b, c, d, e)
    }

    fun not_string_literal<T: drop, T0: drop>(left: T, right: T, a: T0) {
        let fmt = b"a = {}";
        assert_eq!(left, right, fmt, a)
    }
}
