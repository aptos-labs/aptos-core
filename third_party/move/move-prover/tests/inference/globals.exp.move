module 0x42::globals {
    struct Counter has key {
        value: u64,
    }

    // Exists - should infer: ensures result == exists<Counter>(@addr)
    fun has_counter(addr: address): bool {
        exists<Counter>(addr)
    }
    spec has_counter(addr: address): bool {
        ensures [inferred] result == exists<Counter>(addr);
    }


    // GetGlobal (via resource indexing) - should infer:
    //   ensures result == global<Counter>(@addr).value
    //   aborts_if !exists<Counter>(@addr)
    fun get_value(addr: address): u64 acquires Counter {
        Counter[addr].value
    }
    spec get_value(addr: address): u64 {
        ensures [inferred] result == global<Counter>(addr).value;
        aborts_if [inferred] !exists<Counter>(addr);
    }


    // MoveFrom - should infer:
    //   ensures result == global<Counter>(@addr)
    //   aborts_if !exists<Counter>(@addr)
    fun remove_counter(addr: address): Counter acquires Counter {
        move_from<Counter>(addr)
    }
    spec remove_counter(addr: address): Counter {
        ensures [inferred] result == global<Counter>(addr);
        ensures [inferred] !exists<Counter>(addr);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // MoveTo - should infer: aborts_if exists<Counter>(@addr)
    fun create_counter(account: &signer) {
        move_to(account, Counter { value: 0 });
    }
    spec create_counter(account: &signer) {
        ensures [inferred] exists<Counter>(0x1::signer::address_of(account));
        ensures [inferred] global<Counter>(0x1::signer::address_of(account)) == Counter{value: 0};
        aborts_if [inferred] exists<Counter>(0x1::signer::address_of(account));
        modifies [inferred] global<Counter>(0x1::signer::address_of(account));
    }


    // Resource indexing with field read
    fun read_counter(addr: address): u64 acquires Counter {
        let counter_ref = &Counter[addr];
        counter_ref.value
    }
    spec read_counter(addr: address): u64 {
        ensures [inferred] result == global<Counter>(addr).value;
        aborts_if [inferred] !exists<Counter>(addr);
    }


    // Mutable resource indexing with field modification
    // Should infer:
    //   ensures global<Counter>(addr).value == new_value
    //   aborts_if !old(exists<Counter>(addr))
    fun update_counter(addr: address, new_value: u64) acquires Counter {
        let counter_ref = &mut Counter[addr];
        counter_ref.value = new_value;
    }
    spec update_counter(addr: address, new_value: u64) {
        ensures [inferred] global<Counter>(addr) == update_field(old(global<Counter>(addr)), value, new_value);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // MoveTo with explicit Counter construction
    // Should infer:
    //   ensures exists<Counter>(signer_addr)
    //   ensures global<Counter>(signer_addr) == Counter { value: init_value }
    //   aborts_if old(exists<Counter>(signer_addr))
    fun create_with_value(account: &signer, init_value: u64) {
        move_to(account, Counter { value: init_value });
    }
    spec create_with_value(account: &signer, init_value: u64) {
        ensures [inferred] exists<Counter>(0x1::signer::address_of(account));
        ensures [inferred] global<Counter>(0x1::signer::address_of(account)) == Counter{value: init_value};
        aborts_if [inferred] exists<Counter>(0x1::signer::address_of(account));
        modifies [inferred] global<Counter>(0x1::signer::address_of(account));
    }


    // MoveFrom returning field value after unpack
    // Should infer ensures, aborts, and modifies for the resource removal
    fun remove_value(addr: address): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(addr);
        value
    }
    spec remove_value(addr: address): u64 {
        ensures [inferred] result == global<Counter>(addr).value;
        ensures [inferred] !exists<Counter>(addr);
        aborts_if [inferred] !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }


    // Conditional MoveFrom
    // Should infer path-conditional ensures and modifies
    fun conditional_remove(cond: bool, addr: address): u64 acquires Counter {
        if (cond) {
            let Counter { value } = move_from<Counter>(addr);
            value
        } else {
            0
        }
    }
    spec conditional_remove(cond: bool, addr: address): u64 {
        ensures [inferred] cond ==> result == global<Counter>(addr).value;
        ensures [inferred] cond ==> !exists<Counter>(addr);
        ensures [inferred] !cond ==> result == 0;
        aborts_if [inferred] cond && !exists<Counter>(addr);
        modifies [inferred] global<Counter>(addr);
    }

}
/*
Verification: Succeeded.
*/
