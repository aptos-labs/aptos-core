// Tests for behavior predicates with function parameter targets that produce errors.

module 0x42::M {

    struct Counter has key {
        value: u64,
    }

    // Test ensures_of with wrong number of arguments for function parameter
    fun apply_ensures_wrong_arity(f: |address|, addr: address) {
        f(addr)
    }

    spec apply_ensures_wrong_arity {
        ensures ensures_of<f>(addr, 42);
    }
}
