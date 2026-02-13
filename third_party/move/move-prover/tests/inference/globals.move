module 0x42::globals {
    struct Counter has key {
        value: u64,
    }

    // Exists - should infer: ensures result == exists<Counter>(@addr)
    fun has_counter(addr: address): bool {
        exists<Counter>(addr)
    }

    // GetGlobal (via resource indexing) - should infer:
    //   ensures result == global<Counter>(@addr).value
    //   aborts_if !exists<Counter>(@addr)
    fun get_value(addr: address): u64 acquires Counter {
        Counter[addr].value
    }

    // MoveFrom - should infer:
    //   ensures result == global<Counter>(@addr)
    //   aborts_if !exists<Counter>(@addr)
    fun remove_counter(addr: address): Counter acquires Counter {
        move_from<Counter>(addr)
    }

    // MoveTo - should infer: aborts_if exists<Counter>(@addr)
    fun create_counter(account: &signer) {
        move_to(account, Counter { value: 0 });
    }

    // Resource indexing with field read
    fun read_counter(addr: address): u64 acquires Counter {
        let counter_ref = &Counter[addr];
        counter_ref.value
    }

    // Mutable resource indexing with field modification
    // Should infer:
    //   ensures global<Counter>(addr).value == new_value
    //   aborts_if !old(exists<Counter>(addr))
    fun update_counter(addr: address, new_value: u64) acquires Counter {
        let counter_ref = &mut Counter[addr];
        counter_ref.value = new_value;
    }

    // MoveTo with explicit Counter construction
    // Should infer:
    //   ensures exists<Counter>(signer_addr)
    //   ensures global<Counter>(signer_addr) == Counter { value: init_value }
    //   aborts_if old(exists<Counter>(signer_addr))
    fun create_with_value(account: &signer, init_value: u64) {
        move_to(account, Counter { value: init_value });
    }

    // MoveFrom returning field value after unpack
    // Should infer ensures, aborts, and modifies for the resource removal
    fun remove_value(addr: address): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(addr);
        value
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
}
