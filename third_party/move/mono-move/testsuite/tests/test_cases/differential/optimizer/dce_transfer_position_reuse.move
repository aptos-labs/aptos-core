// The second call reads `x` directly after copy propagation, so its copy
// into the first Transfer slot is dead. Dead-instruction elimination removes
// that copy even though an earlier block uses the same Transfer position.

// RUN: publish --print(stackless)
module 0x42::dce_transfer_position_reuse {
    fun add_into(a: u64, b: &mut u64) { *b = *b + a; }

    fun in_branch(x: u64): u64 {
        add_into(x, &mut x);
        let r = 0;
        if (x > 0) {
            let y = x;
            add_into(y, &mut y);
            r = y;
        };
        r + x
    }
}

// RUN: execute 0x42::dce_transfer_position_reuse::in_branch --args 3
// CHECK: results: 18
