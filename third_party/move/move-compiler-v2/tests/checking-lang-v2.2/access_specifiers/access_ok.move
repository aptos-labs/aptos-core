module 0x42::m {

    struct S has store {}
    struct R has store {}
    struct T has store {}
    struct G<phantom T> has store {}

    fun f2() reads S {
    }

    fun f3() writes S {
    }

    fun f4() reads S(*) {
    }

    fun f_multiple() reads R writes T, S reads G<u64> {
    }

    fun f5() reads 0x42::*::* {
    }

    fun f6() reads 0x42::m::* {
    }

    fun f7() reads *(*) {
    }

    fun f8() reads *(0x42) {
    }

    fun f9(a: address) reads *(a) {
    }

    fun f10(x: u64) reads *(make_up_address(x)) {
    }

    fun make_up_address(_x: u64): address {
        @0x42
    }

    fun f11() !reads *(0x42), *(0x43) {
    }

    fun f12() pure {
    }
}
