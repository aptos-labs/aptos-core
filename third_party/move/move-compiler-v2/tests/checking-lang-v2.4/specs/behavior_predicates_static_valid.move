// Tests for valid behavior predicate type checking with qualified function targets.
// Note: Function-typed parameters require inline functions, but inline functions
// don't support spec blocks yet. So we test with qualified function targets only.

module 0x42::M {

    // ========================================
    // Non-generic helper functions
    // ========================================

    // Simple unary function
    public fun increment(x: u64): u64 {
        x + 1
    }

    spec increment {
        requires x < 18446744073709551615;
        ensures result == x + 1;
    }

    // Binary function
    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    spec add {
        requires a + b <= 18446744073709551615;
        ensures result == a + b;
    }

    // Function with no return value
    fun do_nothing(_x: u64) {
    }

    spec do_nothing {
        ensures true;
    }

    // Function with multiple return values
    fun split(x: u64): (u64, u64) {
        (x / 2, x - x / 2)
    }

    spec split {
        ensures result_1 + result_2 == x;
    }

    // Function with different param types
    fun mixed_params(x: u64, flag: bool): u64 {
        if (flag) { x + 1 } else { x }
    }

    // ========================================
    // Resource and functions with modifies
    // ========================================

    // Resource struct for testing modifies_of
    struct Counter has key {
        value: u64,
    }

    // Function that modifies a resource
    public fun increment_counter(addr: address) acquires Counter {
        let counter = borrow_global_mut<Counter>(addr);
        counter.value = counter.value + 1;
    }

    spec increment_counter {
        modifies global<Counter>(addr);
        ensures global<Counter>(addr).value == old(global<Counter>(addr)).value + 1;
    }

    // Function that modifies multiple resources
    public fun swap_counters(addr1: address, addr2: address) acquires Counter {
        let v1 = borrow_global<Counter>(addr1).value;
        let v2 = borrow_global<Counter>(addr2).value;
        borrow_global_mut<Counter>(addr1).value = v2;
        borrow_global_mut<Counter>(addr2).value = v1;
    }

    spec swap_counters {
        modifies global<Counter>(addr1);
        modifies global<Counter>(addr2);
    }

    // ========================================
    // Generic helper functions
    // ========================================

    // Generic identity function (1 type param)
    public fun identity<T>(x: T): T {
        x
    }

    spec identity {
        ensures result == x;
    }

    // Generic swap function (1 type param, 2 args)
    public fun swap<T>(a: T, b: T): (T, T) {
        (b, a)
    }

    spec swap {
        ensures result_1 == b;
        ensures result_2 == a;
    }

    // Generic function with 2 type params
    public fun pair<T, U>(x: T, y: U): (T, U) {
        (x, y)
    }

    spec pair {
        ensures result_1 == x;
        ensures result_2 == y;
    }

    // Generic function returning unit
    fun generic_noop<T: drop>(_x: T) {
    }

    // ========================================
    // Tests: Non-generic functions - all behavior kinds
    // ========================================

    // Test requires_of with non-generic unary function
    fun test_requires_unary(x: u64): u64 {
        increment(x)
    }

    spec test_requires_unary {
        ensures requires_of<increment>(x);
    }

    // Test requires_of with non-generic binary function
    fun test_requires_binary(a: u64, b: u64): u64 {
        add(a, b)
    }

    spec test_requires_binary {
        ensures requires_of<add>(a, b);
    }

    // Test aborts_of with non-generic function
    fun test_aborts(x: u64): u64 {
        increment(x)
    }

    spec test_aborts {
        aborts_if aborts_of<increment>(x);
    }

    // Test ensures_of with non-generic unary function (input + result)
    fun test_ensures_unary(x: u64): u64 {
        increment(x)
    }

    spec test_ensures_unary {
        ensures ensures_of<increment>(x, result);
    }

    // Test ensures_of with non-generic binary function (two inputs + result)
    fun test_ensures_binary(a: u64, b: u64): u64 {
        add(a, b)
    }

    spec test_ensures_binary {
        ensures ensures_of<add>(a, b, result);
    }

    // Test ensures_of with function returning unit (only inputs)
    fun test_ensures_unit(x: u64) {
        do_nothing(x)
    }

    spec test_ensures_unit {
        ensures ensures_of<do_nothing>(x);
    }

    // Test ensures_of with multiple return values
    fun test_ensures_multi(x: u64): (u64, u64) {
        split(x)
    }

    spec test_ensures_multi {
        ensures ensures_of<split>(x, result_1, result_2);
    }

    // Test with mixed parameter types
    fun test_mixed_params(x: u64, b: bool): u64 {
        mixed_params(x, b)
    }

    spec test_mixed_params {
        ensures requires_of<mixed_params>(x, b);
        ensures ensures_of<mixed_params>(x, b, result);
    }

    // Test modifies_of with single resource
    fun test_modifies_single(addr: address) acquires Counter {
        increment_counter(addr)
    }

    spec test_modifies_single {
        // modifies_of returns bool indicating if the modifies spec of the function holds
        ensures modifies_of<increment_counter>(global<Counter>(addr));
    }

    // Test modifies_of with multiple resources
    fun test_modifies_multi(addr1: address, addr2: address) acquires Counter {
        swap_counters(addr1, addr2)
    }

    spec test_modifies_multi {
        ensures modifies_of<swap_counters>(global<Counter>(addr1), global<Counter>(addr2));
    }

    // ========================================
    // Tests: Generic functions WITH explicit type arguments - all behavior kinds
    // ========================================

    // Test requires_of with generic function (explicit type args)
    fun test_requires_generic_explicit(x: u64): u64 {
        identity<u64>(x)
    }

    spec test_requires_generic_explicit {
        ensures requires_of<identity<u64>>(x);
    }

    // Test aborts_of with generic function (explicit type args)
    fun test_aborts_generic_explicit(x: u64): u64 {
        identity<u64>(x)
    }

    spec test_aborts_generic_explicit {
        aborts_if aborts_of<identity<u64>>(x);
    }

    // Test ensures_of with generic function (explicit type args)
    fun test_ensures_generic_explicit(x: u64): u64 {
        identity<u64>(x)
    }

    spec test_ensures_generic_explicit {
        ensures ensures_of<identity<u64>>(x, result);
    }

    // Test with generic swap function (explicit type args)
    fun test_swap_generic_explicit(a: u64, b: u64): (u64, u64) {
        swap<u64>(a, b)
    }

    spec test_swap_generic_explicit {
        ensures requires_of<swap<u64>>(a, b);
        ensures ensures_of<swap<u64>>(a, b, result_1, result_2);
    }

    // Test with 2 type parameters (explicit type args)
    fun test_pair_generic_explicit(x: u64, y: bool): (u64, bool) {
        pair<u64, bool>(x, y)
    }

    spec test_pair_generic_explicit {
        ensures requires_of<pair<u64, bool>>(x, y);
        ensures ensures_of<pair<u64, bool>>(x, y, result_1, result_2);
    }

    // Test ensures_of with generic function returning unit (explicit type args)
    fun test_ensures_generic_unit(x: u64) {
        generic_noop<u64>(x)
    }

    spec test_ensures_generic_unit {
        ensures ensures_of<generic_noop<u64>>(x);
    }

    // ========================================
    // Tests: Generic functions WITHOUT explicit type arguments (inferred)
    // ========================================

    // Test requires_of with generic function (inferred type args)
    fun test_requires_generic_inferred(x: u64): u64 {
        identity(x)
    }

    spec test_requires_generic_inferred {
        // Type args inferred from argument type
        ensures requires_of<identity>(x);
    }

    // Test aborts_of with generic function (inferred type args)
    fun test_aborts_generic_inferred(x: u64): u64 {
        identity(x)
    }

    spec test_aborts_generic_inferred {
        aborts_if aborts_of<identity>(x);
    }

    // Test ensures_of with generic function (inferred type args)
    fun test_ensures_generic_inferred(x: u64): u64 {
        identity(x)
    }

    spec test_ensures_generic_inferred {
        ensures ensures_of<identity>(x, result);
    }

    // Test swap with inferred type args
    fun test_swap_generic_inferred(a: u64, b: u64): (u64, u64) {
        swap(a, b)
    }

    spec test_swap_generic_inferred {
        ensures requires_of<swap>(a, b);
        ensures ensures_of<swap>(a, b, result_1, result_2);
    }

    // ========================================
    // Tests: Generic functions with struct types (verifies type instantiation)
    // ========================================

    // Generic struct for testing type instantiation in behavior predicates
    struct Box<T> has copy, drop {
        value: T,
    }

    // Generic function operating on Box
    public fun unbox<T: copy + drop>(b: Box<T>): T {
        b.value
    }

    spec unbox {
        ensures result == b.value;
    }

    // Test behavior predicate with inferred struct type arguments.
    // This test verifies that type instantiation is properly registered on the node.
    // Without set_node_instantiation, the type variable in Box<T> would remain unresolved
    // as something like Box<?0> instead of Box<u64>.
    fun test_unbox_inferred(b: Box<u64>): u64 {
        unbox(b)
    }

    spec test_unbox_inferred {
        // Type of b is Box<u64>, so unbox is instantiated with T=u64
        ensures requires_of<unbox>(b);
        ensures ensures_of<unbox>(b, result);
    }
}

// Test cross-module function references
module 0x42::N {
    use 0x42::M;

    fun call_increment(x: u64): u64 {
        M::increment(x)
    }

    spec call_increment {
        // Cross-module qualified function reference
        ensures requires_of<M::increment>(x);
        ensures ensures_of<M::increment>(x, result);
    }

    fun call_add(a: u64, b: u64): u64 {
        M::add(a, b)
    }

    spec call_add {
        ensures requires_of<M::add>(a, b);
        ensures ensures_of<M::add>(a, b, result);
    }
}
