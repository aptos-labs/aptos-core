//# publish
module 0x99::closure_compare {

    fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun different_masks_are_not_equal() {
        let captured: u64 = 7;
        // Same body and same captured value, but capture position differs => different masks.
        let cap_first: |u64|u64 has copy + drop = |b| add(captured, b);
        let cap_second: |u64|u64 has copy + drop = |a| add(a, captured);
        assert!(cap_first != cap_second, 0);
    }

    public fun same_mask_and_captures_are_equal() {
        let captured: u64 = 7;
        let c1: |u64|u64 has copy + drop = |b| add(captured, b);
        let c2: |u64|u64 has copy + drop = |b| add(captured, b);
        assert!(c1 == c2, 0);
    }
}

//# run 0x99::closure_compare::different_masks_are_not_equal

//# run 0x99::closure_compare::same_mask_and_captures_are_equal
