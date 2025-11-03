module 0xcafe::a1 {
    public entry fun noop_generic<T>() {
        // Do nothing.
    }
}

module 0xcafe::a2 {
    struct A2 has key, store, copy, drop {}
}

module 0xcafe::a3 {
    struct Counter has key {
        value: u64,
    }

    fun init_module(account: &signer) {
        move_to(account, Counter { value: 0 });
    }

    public entry fun increment_counter() acquires Counter {
        let cnt = &mut borrow_global_mut<Counter>(@0xcafe).value;
        *cnt = *cnt + 1;
    }
}
