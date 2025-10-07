module 0xcafe::m3 {
    use 0xcafe::m2;

    struct M3 has key, store, copy, drop {}

    public entry fun noop() {
        // Do nothing.
    }

    public entry fun load_m2_m1() {
        m2::load_m1();
    }
}

module 0xcafe::m4 {
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
