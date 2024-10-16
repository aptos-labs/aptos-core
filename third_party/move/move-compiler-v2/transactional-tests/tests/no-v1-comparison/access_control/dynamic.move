//# publish
module 0x42::test {
    use 0x1::signer;

    struct R has key, drop { value: bool }

    fun init(s: &signer) {
        move_to(s, R{value: true});
    }

    fun ok1(a: address): bool reads R(a) {
        borrow_global<R>(a).value
    }

    fun ok2(s: &signer): bool reads R(signer::address_of(s)) {
        borrow_global<R>(signer::address_of(s)).value
    }

    fun fail1(_a: address): bool reads R(_a) {
        borrow_global<R>(@0x2).value
    }

    fun fail2(_s: &signer): bool reads R(signer::address_of(_s)) {
        borrow_global<R>(@0x2).value
    }
}

//# run --verbose --signers 0x1 -- 0x42::test::init

//# run --verbose --args @0x1 -- 0x42::test::ok1

//# run --verbose --signers 0x1 -- 0x42::test::ok2

//# run --verbose --args @0x1 -- 0x42::test::fail1

//# run --verbose --signers 0x1 -- 0x42::test::fail2
