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
    fun foo1(cond: bool) {
        assert!(cond)
    }

    fun foo2(cond: bool, code: u64) {
        assert!(cond, code)
    }

    fun foo3(cond: bool, message: vector<u8>) {
        assert!(cond, message)
    }

    fun foo4<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {}", a)
    }

    fun foo5<T0: drop, T1: drop>(cond: bool, a: T0, b: T1) {
        assert!(cond, b"a = {}, b = {}", a, b)
    }

    fun foo6<T0: drop, T1: drop, T2: drop>(cond: bool, a: T0, b: T1, c: T2) {
        assert!(cond, b"a = {}, b = {}, c = {}", a, b, c)
    }

    fun foo7<T0: drop, T1: drop, T2: drop, T3: drop>(cond: bool, a: T0, b: T1, c: T2, d: T3) {
        assert!(cond, b"a = {}, b = {}, c = {}, d = {}", a, b, c, d)
    }

    fun bar<T0: drop>(cond: bool, a: T0) {
        assert!(cond, b"a = {{{}}}", a)
    }
}
