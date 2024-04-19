//# publish
module 0x42::test {
    struct R has key, drop { value: bool }

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun ok1(): bool reads 0x42::*::* {
        borrow_global<R>(@0x1).value
    }

    fun ok2(): bool reads 0x42::test::* {
        borrow_global<R>(@0x1).value
    }

    fun ok3(): bool reads 0x42::test::*(*) {
        borrow_global<R>(@0x1).value
    }

    fun ok4(): bool reads *(0x1) {
        borrow_global<R>(@0x1).value
    }

    fun fail1(): bool reads 0x43::*::* {
        borrow_global<R>(@0x1).value
    }

    fun fail2(): bool reads *(0x2) {
        borrow_global<R>(@0x1).value
    }
}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose -- 0x42::test::ok1

//# run --verbose -- 0x42::test::ok2

//# run --verbose -- 0x42::test::ok3

//# run --verbose -- 0x42::test::ok4

//# run --verbose -- 0x42::test::fail1

//# run --verbose -- 0x42::test::fail2
