//# publish
module 0x42::test {
    struct R has key, drop { value: bool }
    struct Other has key, drop {}

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun ok1(): bool reads R {
        borrow_global<R>(@0x1).value
    }

    fun ok2(): bool reads R {
        exists<R>(@0x1)
    }

    fun ok3(s: &signer): bool writes R {
        move_to(s, R{value: true});
        true
    }

    fun ok4(): bool writes R {
        borrow_global_mut<R>(@0x1).value = false;
        true
    }

    fun ok5(): bool acquires R {
        borrow_global_mut<R>(@0x1).value = false;
        true
    }


    fun fail1(): bool reads Other {
        !borrow_global<R>(@0x1).value
    }

    fun fail2(): bool reads Other {
        !exists<R>(@0x1)
    }

    fun fail3(): bool reads R writes Other {
        let r = move_from<R>(@0x1);
        !r.value
    }

    fun fail4(): bool reads R writes Other {
        borrow_global_mut<R>(@0x1).value = false;
        false
    }

    fun fail5(): bool reads Other {
        fail_no_subsumes()
    }

    fun fail_no_subsumes(): bool reads R {
        borrow_global<R>(@0x1).value
    }
}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose -- 0x42::test::ok1

//# run --verbose -- 0x42::test::ok2

//# run --verbose --signers 0x2 -- 0x42::test::ok3

//# run --verbose -- 0x42::test::ok4

//# run --verbose -- 0x42::test::ok5

//# run --verbose -- 0x42::test::fail1

//# run --verbose -- 0x42::test::fail2

//# run --verbose -- 0x42::test::fail3

//# run --verbose -- 0x42::test::fail4

//# run --verbose -- 0x42::test::fail5
