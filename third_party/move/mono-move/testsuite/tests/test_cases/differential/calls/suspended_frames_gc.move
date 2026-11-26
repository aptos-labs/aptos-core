// A recursive function four calls deep, in which each call owns a vector that it reads only
// after the deeper calls return. The innermost call collects, allocates a decoy vector and
// collects again, so every caller's vector must survive both collections intact. The golden
// pins the lowering and the frame layout.

// RUN: publish --print(micro-ops,frame-layout)
module 0x42::suspended {
    use std::vector;

    fun descend(depth: u64, out: &mut vector<u64>) {
        let mine = vector[depth + 1, depth * 10];
        if (depth == 0) {
            0x0::test_utils::force_gc();
            let decoy = vector[depth + 9001, depth + 9002];
            0x0::test_utils::force_gc();
            vector::push_back(out, *vector::borrow(&decoy, 0) + *vector::borrow(&decoy, 1));
        } else descend(depth - 1, out);
        vector::push_back(out, *vector::borrow(&mine, 0) * 100 + *vector::borrow(&mine, 1));
    }

    public fun run(depth: u64): vector<u64> {
        let out = vector::empty<u64>();
        descend(depth, &mut out);
        out
    }
}

// RUN: execute 0x42::suspended::run --args 3
// CHECK: results: [18003, 100, 210, 320, 430]
// CHECK-GC-COUNT: 2
