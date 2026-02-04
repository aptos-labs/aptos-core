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
    fun foo1<T: copy + drop>(left: T, right: T) {
        assert_eq!(left, right)
    }

    fun foo2<T: copy + drop>(left: T, right: T, message: vector<u8>) {
        assert_eq!(left, right, message)
    }

    fun foo3<T: copy + drop, T0: drop>(left: T, right: T, a: T0) {
        assert_eq!(left, right, b"a = {}", a)
    }

    fun foo4<T: copy + drop, T0: drop, T1: drop>(left: T, right: T, a: T0, b: T1) {
        assert_eq!(left, right, b"a = {}, b = {}", a, b)
    }

    fun foo5<T: copy + drop, T0: drop, T1: drop, T2: drop>(left: T, right: T, a: T0, b: T1, c: T2) {
        assert_eq!(left, right, b"a = {}, b = {}, c = {}", a, b, c)
    }

    fun foo6<T: copy + drop, T0: drop, T1: drop, T2: drop, T3: drop>(left: T, right: T, a: T0, b: T1, c: T2, d: T3) {
        assert_eq!(left, right, b"a = {}, b = {}, c = {}, d = {}", a, b, c, d)
    }
}
