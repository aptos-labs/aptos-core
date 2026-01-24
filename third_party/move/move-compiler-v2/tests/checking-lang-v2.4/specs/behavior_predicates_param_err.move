// Tests for behavior predicates with function parameter targets that produce errors.

module 0x42::M {

    struct Counter has key {
        value: u64,
    }

    // Test modifies_of with function parameter - not supported
    fun apply_modifies(f: |address|, addr: address) {
        f(addr)
    }

    spec apply_modifies {
        ensures modifies_of<f>(global<Counter>(addr));
    }
}
