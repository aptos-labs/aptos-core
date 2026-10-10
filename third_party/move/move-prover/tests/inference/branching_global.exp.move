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
    spec initialize_or_append(owner: &signer, value: u64) {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(owner)];
        ensures [inferred] !old(exists<R>(signer::address_of(owner))) ==> publish<R>(signer::address_of(owner), R{values: vec(value)});
        ensures [inferred] old(exists<R>(signer::address_of(owner))) ==> update<R>(signer::address_of(owner), update_field(old(R[signer::address_of(owner)]), values, concat(old(R[signer::address_of(owner)]).values, vec(value))));
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
