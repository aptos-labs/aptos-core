//# publish
module 0xc0ffee::m {
    public fun inc(y: u64): u64 {
        y + 1
    }

    public fun inc2(y: u64): u64 {
        y + 2
    }

    // Wide operand first.
    fun wide_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f == g
    }

    fun narrow_first(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        g == f
    }

    fun wide_first_neq(f: |u64|u64 has copy + drop, g: |u64|u64 has drop): bool {
        f != g
    }

    // Neither ability set is a subset of the other.
    fun incomparable(f: |u64|u64 has copy + drop, g: |u64|u64 has drop + store): bool {
        f == g
    }

    fun incomparable_swapped(f: |u64|u64 has copy + drop, g: |u64|u64 has drop + store): bool {
        g == f
    }

    fun incomparable_ref(f: &|u64|u64 has copy + drop, g: &|u64|u64 has drop + store): bool {
        f == g
    }

    fun incomparable_mut_ref(
        f: &mut |u64|u64 has copy + drop,
        g: &mut |u64|u64 has drop + store
    ): bool {
        f == g
    }

    fun main() {
        assert!(wide_first(inc, inc), 0);
        assert!(narrow_first(inc, inc), 1);
        assert!(!wide_first(inc, inc2), 2);
        assert!(!narrow_first(inc, inc2), 3);
        assert!(!wide_first_neq(inc, inc), 4);
        assert!(wide_first_neq(inc, inc2), 5);

        assert!(incomparable(inc, inc), 6);
        assert!(incomparable_swapped(inc, inc), 7);
        assert!(!incomparable(inc, inc2), 8);
        assert!(!incomparable_swapped(inc, inc2), 9);

        let f: |u64|u64 has copy + drop = inc;
        let g: |u64|u64 has drop + store = inc;
        let h: |u64|u64 has drop + store = inc2;
        assert!(incomparable_ref(&f, &g), 10);
        assert!(!incomparable_ref(&f, &h), 11);
        assert!(incomparable_mut_ref(&mut f, &mut g), 12);
        assert!(!incomparable_mut_ref(&mut f, &mut h), 13);
    }
}

//# run 0xc0ffee::m::main
