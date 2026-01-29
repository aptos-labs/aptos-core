module 0x42::resources {

    struct Counter has key, drop {
        value: u64,
    }

    struct Wallet has key, drop {
        balance: u64,
        frozen: bool,
    }

    // Test move_to - publishing a resource
    fun publish_counter(account: &signer, initial: u64) {
        let c = Counter { value: initial };
        move_to(account, c);
    }

    // Test move_from - acquiring a resource
    fun acquire_counter(addr: address): u64 acquires Counter {
        let c = move_from<Counter>(addr);
        c.value
    }

    // Test borrow_global (immutable)
    fun read_counter(addr: address): u64 acquires Counter {
        let c = borrow_global<Counter>(addr);
        c.value
    }

    // Test borrow_global_mut - modifying resource
    fun increment_counter(addr: address) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        c.value = c.value + 1;
    }

    // Test exists check with conditional borrow
    fun check_and_read(addr: address): u64 acquires Counter {
        let result;
        if (exists<Counter>(addr)) {
            let c = borrow_global<Counter>(addr);
            result = c.value;
        } else {
            result = 0;
        };
        result
    }

    // Test conditional move_from
    fun conditional_acquire(addr: address, cond: bool): u64 acquires Counter {
        let result;
        if (cond) {
            let c = move_from<Counter>(addr);
            result = c.value;
        } else {
            result = 0;
        };
        result
    }

    // Test with multiple resource types
    fun read_multiple(addr: address): u64 acquires Counter, Wallet {
        let c = borrow_global<Counter>(addr);
        let w = borrow_global<Wallet>(addr);
        c.value + w.balance
    }

    // ========== Function call tests with global resources ==========

    // Helper: reads counter value (immutable ref)
    fun read_value(c: &Counter): u64 {
        c.value
    }

    // Helper: modifies counter (mutable ref)
    fun add_to_counter(c: &mut Counter, amount: u64) {
        c.value = c.value + amount;
    }

    // Test: call with mutable ref should mark global as modified
    fun call_with_mut_ref(addr: address) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        add_to_counter(c, 1);
    }

    // Test: call with immutable ref should NOT mark global as modified
    fun call_with_immut_ref(addr: address): u64 acquires Counter {
        let c = borrow_global<Counter>(addr);
        read_value(c)
    }

    // Test: conditional call with mutable ref
    fun conditional_mut_call(addr: address, cond: bool) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        if (cond) {
            add_to_counter(c, 1);
        };
    }

    // Test: call in loop with mutable ref
    fun loop_mut_call(addr: address, n: u64) acquires Counter {
        let i = 0;
        while (i < n) {
            let c = borrow_global_mut<Counter>(addr);
            add_to_counter(c, 1);
            i = i + 1;
        };
    }

    // Test: nested function calls
    fun nested_helper(c: &mut Counter) {
        add_to_counter(c, 10);
    }

    fun call_nested(addr: address) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        nested_helper(c);
    }

    // ========== Closure/function value tests ==========

    // Helper that modifies Counter via borrow_global_mut
    fun modifier_func(addr: address) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        c.value = c.value + 1;
    }

    // Helper that only reads Counter
    fun reader_func(addr: address): u64 acquires Counter {
        let c = borrow_global<Counter>(addr);
        c.value
    }

    // Test: invoke a locally created closure that modifies global
    // Note: Move's type system doesn't track acquires through function values
    fun invoke_local_modifier(addr: address) {
        let f = modifier_func;
        f(addr);
    }

    // Test: invoke a locally created closure that only reads global
    fun invoke_local_reader(addr: address): u64 {
        let f = reader_func;
        f(addr)
    }

    // Test: closure passed as argument - analysis cannot track what it accesses
    fun invoke_passed_closure(addr: address, f: |address|) {
        f(addr);
    }

    // Test: passed closure with mutable ref - should mark global as modified
    fun invoke_passed_closure_mut_ref(addr: address, f: |&mut Counter|) acquires Counter {
        let c = borrow_global_mut<Counter>(addr);
        f(c);
    }

    // Test: caller passes modifier_func to invoke_passed_closure
    // This demonstrates the soundness issue: Counter is modified but analysis misses it
    fun test_pass_modifier_closure(addr: address) {
        invoke_passed_closure(addr, modifier_func);
    }
}
