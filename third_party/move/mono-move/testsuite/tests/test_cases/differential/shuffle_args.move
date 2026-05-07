// RUN: publish
module 0xc0ffee::shuffle_args {
    public fun shuffle_1(
        a0: u64, a1: u64, a2: u64, a3: u64, a4: u64,
        a5: u64, a6: u64, a7: u64, a8: u64, a9: u64,
    ): (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (a0, a1, a2, a3, a4, a5, a6, a7, a8, a9)
    }

    public fun shuffle_2(
        a0: u64, a1: u64, a2: u64, a3: u64, a4: u64,
        a5: u64, a6: u64, a7: u64, a8: u64, a9: u64,
    ): (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (a9, a8, a7, a6, a5, a4, a3, a2, a1, a0)
    }

    public fun shuffle_3(
        a0: u64, a1: u64, a2: u64, a3: u64, a4: u64,
        a5: u64, a6: u64, a7: u64, a8: u64, a9: u64,
    ): (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (a1, a2, a3, a4, a5, a6, a7, a8, a9, a0)
    }

    public fun shuffle_4(
        a0: u64, a1: u64, a2: u64, a3: u64, a4: u64,
        a5: u64, a6: u64, a7: u64, a8: u64, a9: u64,
    ): (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        // Cycles (0,3), (2,7), (5,6); identity at 1, 4, 8, 9.
        (a3, a1, a7, a0, a4, a6, a5, a2, a8, a9)
    }
}

// RUN: execute 0xc0ffee::shuffle_args::shuffle_1 --args 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
// CHECK: results: 1, 2, 3, 4, 5, 6, 7, 8, 9, 10

// RUN: execute 0xc0ffee::shuffle_args::shuffle_2 --args 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
// CHECK: results: 10, 9, 8, 7, 6, 5, 4, 3, 2, 1

// RUN: execute 0xc0ffee::shuffle_args::shuffle_3 --args 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
// CHECK: results: 2, 3, 4, 5, 6, 7, 8, 9, 10, 1

// RUN: execute 0xc0ffee::shuffle_args::shuffle_4 --args 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
// CHECK: results: 4, 2, 8, 1, 5, 7, 6, 3, 9, 10
