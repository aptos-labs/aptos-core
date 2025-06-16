module 0x1::function_values {
    use std::signer;
    use 0x1::proxy::{borrow_and_add, move_roundtrip_with_action};

    public entry fun add_1(account: &signer, value: u64) {
        let f = |a, v| borrow_and_add(a, v);
        let addr = signer::address_of(account);
        f(addr, value);
    }

    public entry fun add_2(account: &signer, value: u64) {
        let addr = signer::address_of(account);
        let f = || borrow_and_add(addr, value);
        f();
    }

    public entry fun add_3(account: &signer, value: u64) {
        let f = |c| {
            c.add(value);
            c
        };
        move_roundtrip_with_action(account, f);
    }
}
