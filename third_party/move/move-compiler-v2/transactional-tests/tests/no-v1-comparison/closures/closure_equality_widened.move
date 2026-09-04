//# publish
module 0xc0ffee::m {
    struct DropStore has drop, store { x: u64 }

    public fun with_capture(d: DropStore, y: u64): u64 {
        let DropStore { x } = d;
        x + y
    }

    public fun compare(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) == g
    }
}

//# publish
module 0xc0ffee::via_call {
    struct DropStore has drop, store { x: u64 }

    public fun with_capture(d: DropStore, y: u64): u64 {
        let DropStore { x } = d;
        x + y
    }

    fun eq_helper(a: |u64|u64 has drop, b: |u64|u64 has drop): bool {
        a == b
    }

    public fun compare(d: DropStore, g: |u64|u64 has drop): bool {
        eq_helper(|y| with_capture(d, y), g)
    }

    fun main() {
        let d1 = DropStore { x: 7 };
        let d2 = DropStore { x: 7 };
        let d3 = DropStore { x: 8 };
        let d4 = DropStore { x: 7 };
        assert!(compare(d1, |y| with_capture(d2, y)), 0);
        assert!(!compare(d3, |y| with_capture(d4, y)), 1);
    }
}

//# publish
module 0xc0ffee::shapes {
    struct DropStore has drop, store { x: u64 }

    struct CopyDropStore has copy, drop, store { x: u64 }

    public fun with_capture(d: DropStore, y: u64): u64 {
        let DropStore { x } = d;
        x + y
    }

    public fun with_copy_capture(c: CopyDropStore, y: u64): u64 {
        c.x + y
    }

    public fun inc(y: u64): u64 {
        y + 1
    }

    public fun inc2(y: u64): u64 {
        y + 2
    }

    // Exercises repeated-operand copy/move handling.
    fun same_operand(f: |u64|u64 has copy + drop): bool {
        f == f
    }

    // `!=` follows a separate generator arm and requires the same normalization.
    fun widened_neq(d: DropStore, g: |u64|u64 has drop): bool {
        (|y| with_capture(d, y)) != g
    }

    // Exercises equality normalization through immutable references.
    fun ref_compare(f: &|u64|u64 has drop, g: &|u64|u64 has drop): bool {
        g == f
    }

    fun main() {
        assert!(same_operand(inc), 0);
        let same = DropStore { x: 7 };
        assert!(!widened_neq(DropStore { x: 7 }, |y| with_capture(same, y)), 1);
        let other = DropStore { x: 8 };
        assert!(widened_neq(DropStore { x: 7 }, |y| with_capture(other, y)), 6);

        // Both operands share a checked type but derive different ability sets.
        let lhs: |u64|u64 has drop = |y| with_capture(DropStore { x: 7 }, y);
        let rhs: |u64|u64 has drop = |y| with_copy_capture(CopyDropStore { x: 7 }, y);
        assert!(rhs != lhs, 2);

        let wide: |u64|u64 has copy + drop + store = inc;
        let narrow: |u64|u64 has drop = inc;
        let other: |u64|u64 has drop = inc2;
        assert!(ref_compare(&wide, &narrow), 3);
        assert!(!ref_compare(&wide, &other), 4);
    }
}

//# run 0xc0ffee::via_call::main

//# run 0xc0ffee::shapes::main
