// RUN: publish --print(bytecode,stackless,micro-ops,frame-layout)
module 0xc0ffee::vec_sum {
    // Builds a vector<u64> with elements 0..n, then sums and returns
    // their total via pop_back. Exercises VecPack(0)->VecNew,
    // VecPushBack, VecLen, and VecPopBack end-to-end.
    //
    // Keep n small: VecPushBack is allowed past the lowering tripwire
    // without per-PC safe_point_layouts (follow-up 2), so any GC cycle
    // would risk missing callee-region pointers. The fixed n=100 cap
    // here stays well within the default heap.
    public fun sum_first_n(n: u64): u64 {
        let v = std::vector::empty<u64>();
        let i = 0;
        while (i < n) {
            std::vector::push_back(&mut v, i);
            i = i + 1;
        };
        let acc = 0;
        let len = std::vector::length(&v);
        while (len > 0) {
            acc = acc + std::vector::pop_back(&mut v);
            len = len - 1;
        };
        acc
    }
}

// RUN: execute 0xc0ffee::vec_sum::sum_first_n --args 0
// CHECK: results: 0

// RUN: execute 0xc0ffee::vec_sum::sum_first_n --args 10
// CHECK: results: 45

// RUN: execute 0xc0ffee::vec_sum::sum_first_n --args 100
// CHECK: results: 4950
