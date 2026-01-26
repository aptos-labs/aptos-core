// Tests for spec function tuple return types
module 0x42::TupleTest {

    // === Spec functions returning tuples ===

    // 2-tuple
    spec fun pair(x: u64, y: u64): (u64, u64) { (x, y) }

    // 3-tuple
    spec fun triple(x: u64, y: u64, z: u64): (u64, u64, u64) { (x, y, z) }

    // 4-tuple
    spec fun quad(a: u64, b: u64, c: u64, d: u64): (u64, u64, u64, u64) { (a, b, c, d) }

    // 8-tuple (max size)
    spec fun octet(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64, h: u64):
        (u64, u64, u64, u64, u64, u64, u64, u64) {
        (a, b, c, d, e, f, g, h)
    }

    // Swap using tuple - returns the arguments in reverse order
    spec fun swap(x: u64, y: u64): (u64, u64) {
        (y, x)
    }

    // === Generic spec functions returning tuples ===

    // Generic 2-tuple
    spec fun generic_pair<T1, T2>(x: T1, y: T2): (T1, T2) { (x, y) }

    // Generic swap
    spec fun generic_swap<T1, T2>(x: T1, y: T2): (T2, T1) { (y, x) }

    // Mixed generic and concrete types
    spec fun mixed_pair<T>(x: T, y: u64): (T, u64) { (x, y) }

    // === Test functions with specs ===

    fun test_pair(): bool { true }
    spec test_pair {
        // Test tuple equality
        ensures pair(1, 2) == (1, 2);
    }

    fun test_triple(): bool { true }
    spec test_triple {
        // Test tuple equality
        ensures triple(1, 2, 3) == (1, 2, 3);
    }

    fun test_quad(): bool { true }
    spec test_quad {
        ensures quad(1, 2, 3, 4) == (1, 2, 3, 4);
    }

    fun test_octet(): bool { true }
    spec test_octet {
        // Test 8-tuple (max size)
        ensures octet(1, 2, 3, 4, 5, 6, 7, 8) == (1, 2, 3, 4, 5, 6, 7, 8);
    }

    fun test_swap(): bool { true }
    spec test_swap {
        // Test that swap(1, 2) returns (2, 1)
        ensures swap(1, 2) == (2, 1);
    }

    // Test that spec functions can be compared
    fun test_comparison(): bool { true }
    spec test_comparison {
        ensures pair(1, 2) == pair(1, 2);
        ensures triple(1, 2, 3) == triple(1, 2, 3);
        ensures swap(1, 2) == swap(1, 2);
    }

    // Test negative case - swapped values are different
    fun test_swap_neq(): bool { true }
    spec test_swap_neq {
        ensures swap(1, 2) != (1, 2);
    }

    // === Tests for generic spec functions ===

    fun test_generic_pair(): bool { true }
    spec test_generic_pair {
        ensures generic_pair<u64, bool>(42, true) == (42, true);
        ensures generic_pair<address, u128>(@0x1, 100) == (@0x1, 100);
    }

    fun test_generic_swap(): bool { true }
    spec test_generic_swap {
        ensures generic_swap<u64, bool>(42, true) == (true, 42);
        ensures generic_swap<bool, u64>(true, 42) == (42, true);
    }

    fun test_mixed_pair(): bool { true }
    spec test_mixed_pair {
        ensures mixed_pair<bool>(true, 42) == (true, 42);
        ensures mixed_pair<address>(@0x1, 100) == (@0x1, 100);
    }
}
