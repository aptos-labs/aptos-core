module 0xcafe::a1 {
    public entry fun noop_generic<T>() {
        // Do nothing.
    }
}

module 0xcafe::a2 {
    struct A2 has key, store, copy, drop {}
}

module 0xcafe::a3 {
    const ENOT_AUTHORIZED: u64 = 1;

    struct Counter has key {
        value: u64,
    }

    public entry fun initialize(account: &signer) {
        assert!(std::signer::address_of(account) == @0xcafe, ENOT_AUTHORIZED);
        move_to(account, Counter { value: 0 });
    }

    public entry fun increment_counter() acquires Counter {
        let cnt = &mut borrow_global_mut<Counter>(@0xcafe).value;
        *cnt = *cnt + 1;
    }
}
