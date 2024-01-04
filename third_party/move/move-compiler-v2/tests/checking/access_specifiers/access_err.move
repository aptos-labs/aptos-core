module 0x42::m {

    struct S has store {}
    struct G<T> has store {}

    fun f1() acquires undef {
    }

    fun f2() acquires 0x42::undef::* {
    }

    fun f3() acquires 0x42::*::S {
    }

    fun f4() acquires G {
    }

    fun f5(x: address) acquires *(y) {
    }

    fun f6(x: address) acquires *(make_up_address(y)) {
    }

    fun f7(x: u64) acquires *(make_up_address_wrong(x)) {
    }

    fun f8(x: u64) acquires *(undefined(x)) {
    }

    fun make_up_address(foo: u64): address {
        @0x42
    }

    fun make_up_address_wrong(foo: u64): u64 {
        0x42
    }
}
