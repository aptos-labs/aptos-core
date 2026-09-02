module 0x66::branching_global {
    use 0x1::signer::address_of;
    use 0x1::vector;

    struct R has key {
        values: vector<u64>,
    }

    public fun initialize_or_append(owner: &signer, value: u64) acquires R {
        let addr = address_of(owner);
        if (!exists<R>(addr)) {
            move_to(owner, R { values: vector[value] })
        } else {
            vector::push_back(&mut borrow_global_mut<R>(addr).values, value)
        }
    }
}
