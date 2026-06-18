// Differential test for `cmp::compare`, calling the real `std::cmp`. Results
// are surfaced as bools via the variant-test helpers in `std::cmp`.

// RUN: publish
module 0x42::main {
    use std::cmp;

    struct Pair has drop {
        a: u64,
        b: u64,
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
