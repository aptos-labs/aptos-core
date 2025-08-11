//# publish
module 0x66::test {
    struct Func has key {
        f: ||u64 has store + copy,
    }

    public fun identity<T>(x: T): T { x }

    public fun init(account: &signer) {
        let f = || 0x66::test::identity(23);
        move_to(account, Func { f });
    }

    public fun call(addr: address) acquires Func {
        let f = &borrow_global<Func>(addr).f;
        let value = (*f)();
        assert!(value == 23, 777);
    }
}

//# run 0x66::test::init --signers 0x66

//# run 0x66::test::call --args @0x66
