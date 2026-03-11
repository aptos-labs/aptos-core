module 0x42::m {

    struct S has store {}
    struct R has store {}
    struct T has store {}
    struct G<phantom T> has store {}

    fun f1() acquires S {
    }

    fun f2() reads S {
    }

    fun f3() writes S {
    }

    fun f4() acquires S(*) {
    }

    fun f_multiple() acquires R reads R writes T, S reads G<u64> {
    }

    fun f5() acquires 0x42::*::* {
    }

    fun f6() acquires 0x42::m::* {
    }

    fun f7() acquires *(*) {
    }

    fun f8() acquires *(0x42) {
    }

    fun f9(a: address) acquires *(a) {
    }

    fun f10(x: u64) acquires *(make_up_address(x)) {
    }

    fun make_up_address(x: u64): address {
        @0x42
    }

    fun f11() !reads *(0x42), *(0x43) {
    }

    fun f12() pure {
    }
}
