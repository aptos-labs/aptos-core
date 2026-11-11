// Differential test for `cmp::compare`, calling the real `std::cmp`. Results
// are surfaced as bools via the variant-test helpers in `std::cmp`.

// RUN: publish
module 0x42::main {
    use std::cmp;

    struct Pair has drop {
        a: u64,
        b: u64,
    }

    struct Tagged has drop {
        tag: u64,
        f: ||u64 has drop,
    }

    fun one(): u64 {
        1
    }

    fun two(): u64 {
        2
    }

    fun adder(x: u64): |u64|u64 has drop {
        |y| x + y
    }

    public fun u64_lt(): bool {
        cmp::compare(&1u64, &2u64).is_lt()
    }

    public fun u64_eq(): bool {
        cmp::compare(&7u64, &7u64).is_eq()
    }

    public fun u64_gt(): bool {
        cmp::compare(&9u64, &3u64).is_gt()
    }

    public fun bool_lt(): bool {
        cmp::compare(&false, &true).is_lt()
    }

    // Structs compare lexicographically: first field equal, second decides.
    public fun struct_lt(): bool {
        cmp::compare(&Pair { a: 1, b: 2 }, &Pair { a: 1, b: 3 }).is_lt()
    }

    // Vectors compare lexicographically.
    public fun vec_gt(): bool {
        cmp::compare(&vector[1u8, 9u8], &vector[1u8, 3u8]).is_gt()
    }

    // MonoMove does not order function values, which V1 does.
    public fun closure_same_target(): bool {
        let f: ||u64 has drop = one;
        cmp::compare(&f, &f).is_eq()
    }

    public fun closure_different_targets(): bool {
        let f: ||u64 has drop = one;
        let g: ||u64 has drop = two;
        cmp::compare(&f, &g).is_lt()
    }

    public fun closure_different_captures(): bool {
        cmp::compare(&adder(1), &adder(2)).is_lt()
    }

    // The tags tie, so the comparison reaches the closures.
    public fun closure_in_struct(): bool {
        let a = Tagged { tag: 1, f: one };
        let b = Tagged { tag: 1, f: two };
        cmp::compare(&a, &b).is_lt()
    }

    // The tags differ, so the comparison never reaches the closures.
    public fun closure_in_struct_decided_before(): bool {
        let a = Tagged { tag: 1, f: two };
        let b = Tagged { tag: 2, f: one };
        cmp::compare(&a, &b).is_lt()
    }
}

// RUN: execute 0x42::main::u64_lt
// CHECK: results: true

// RUN: execute 0x42::main::u64_eq
// CHECK: results: true

// RUN: execute 0x42::main::u64_gt
// CHECK: results: true

// RUN: execute 0x42::main::bool_lt
// CHECK: results: true

// RUN: execute 0x42::main::struct_lt
// CHECK: results: true

// RUN: execute 0x42::main::vec_gt
// CHECK: results: true

// RUN: execute 0x42::main::closure_same_target
// CHECK-V1: results: true
// CHECK-V2-SUBSTR: function values cannot be ordered

// RUN: execute 0x42::main::closure_different_targets
// CHECK-V1: results: true
// CHECK-V2-SUBSTR: function values cannot be ordered

// RUN: execute 0x42::main::closure_different_captures
// CHECK-V1: results: true
// CHECK-V2-SUBSTR: function values cannot be ordered

// RUN: execute 0x42::main::closure_in_struct
// CHECK-V1: results: true
// CHECK-V2-SUBSTR: function values cannot be ordered

// RUN: execute 0x42::main::closure_in_struct_decided_before
// CHECK: results: true
