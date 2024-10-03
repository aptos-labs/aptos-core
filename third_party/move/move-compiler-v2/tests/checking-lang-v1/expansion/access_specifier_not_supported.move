module 0x42::m {

    struct S has key {}
    struct R has key {}
    struct T has key {}
    struct G<phantom T> has key {}

    fun f2() reads S {
    }

    fun f3() writes S {
    }

    fun f4() acquires S(*) {
    }

    fun f5() acquires 0x42::*::* {
    }

    fun f6() acquires 0x42::m::R {
    }

    fun f7() acquires *(*) {
    }

    fun f8() acquires *(0x42) {
    }

    fun f9(_a: address) acquires *(_a) {
    }
}
