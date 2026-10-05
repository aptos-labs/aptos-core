module 0x42::deferred_modifies_check {
    struct R has key {
        value: u64,
    }

    public fun initialize(account: &signer) {
        move_to(account, R { value: 0 });
    }
    spec initialize(account: &signer) {
        use 0x1::signer;
        pragma opaque = true;
        modifies R[signer::address_of(account)];
        ensures [inferred] publish<R>(signer::address_of(account), R{value: 0});
        aborts_if [inferred] exists<R>(signer::address_of(account));
    }


    public fun write_value(addr: address, value: u64) acquires R {
        borrow_global_mut<R>(addr).value = value;
    }
    spec write_value(addr: address, value: u64) {
        pragma opaque = true;
        modifies R[addr];
        ensures [inferred] update<R>(addr, update_field(old(R[addr]), value, value));
        aborts_if [inferred] !exists<R>(addr);
    }


    public fun update_from_caller(addr: address, value: u64) acquires R {
        write_value(addr, value);
    }

    spec update_from_caller {
        modifies global<R>(addr);
        pragma opaque = true;
        ensures [inferred] ensures_of<write_value>(addr, value);
        aborts_if [inferred] aborts_of<write_value>(addr, value);
    }
}
/*
Verification: Succeeded.
*/
