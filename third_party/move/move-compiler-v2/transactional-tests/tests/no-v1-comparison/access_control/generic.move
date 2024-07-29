//# publish
module 0x42::test {

    struct R<T> has key, drop { value: T }

    fun init(s: &signer) {
        move_to(s, R<bool>{value: true});
    }

    fun ok1(): bool reads R {
        borrow_global<R<bool>>(@0x1).value
    }

    fun ok2(): bool reads R<bool> {
        borrow_global<R<bool>>(@0x1).value
    }

    fun fail1(): bool reads R<u64> {
        borrow_global<R<bool>>(@0x1).value
    }
}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose -- 0x42::test::ok1

//# run --verbose -- 0x42::test::ok2

//# run --verbose -- 0x42::test::fail1
