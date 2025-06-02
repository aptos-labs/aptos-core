module 0x42::m {

    struct S has store {}
    struct G<T> has store {}

    fun f1() reads undef {
    }

    fun f2() reads 0x42::undef::* {
    }

    fun f3() reads 0x42::*::S {
    }

    fun f4() reads G {
    }

    fun f5(x: address) reads *(y) {
    }

    fun f6(x: address) reads *(make_up_address(y)) {
    }

    fun f7(x: u64) reads *(make_up_address_wrong(x)) {
    }

    fun f8(x: u64) reads *(undefined(x)) {
    }

    fun make_up_address(foo: u64): address {
        @0x42
    }

    fun make_up_address_wrong(foo: u64): u64 {
        0x42
    }
}
