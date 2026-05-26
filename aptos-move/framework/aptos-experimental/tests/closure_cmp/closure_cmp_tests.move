#[test_only]
module aptos_experimental::closure_cmp_tests {
    use std::cmp::compare;

    #[test_only]
    fun add_pair(x: u64, y: u64): u64 {
        x + y
    }

    #[test]
    fun test_compare_closures_with_different_masks() {
        let captured: u64 = 7;

        // Same body and same captured value, but capture position differs => different masks.
        let cap_first: |u64|u64 has drop = |b| add_pair(captured, b);
        let cap_second: |u64|u64 has drop = |a| add_pair(a, captured);
        assert!(compare(&cap_first, &cap_second).is_ne(), 0);

        // Identical function, mask, and captures must compare equal.
        let cap_first_dup: |u64|u64 has drop = |b| add_pair(captured, b);
        assert!(compare(&cap_first, &cap_first_dup).is_eq(), 1);
    }
}
