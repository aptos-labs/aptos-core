// A parameter written in a loop and read on the next iteration is live at
// the loop block's end even though no other block references it.
// Dead-instruction elimination must keep the `a = n` copy.

// RUN: publish
module 0x42::dce_loop_carried_param {
    // The loop is a later block.
    fun loop_after_entry(a: u64): u64 {
        let n = 1;
        loop {
            n = n + a;
            a = n;
            if (n > 100) break;
        };
        n
    }

    // The loop is the entry block.
    fun loop_at_entry(a: u64, n: u64): u64 {
        loop {
            n = n + a;
            a = n;
            if (n > 100) break;
        };
        n
    }
}

// RUN: execute 0x42::dce_loop_carried_param::loop_after_entry --args 1
// CHECK: results: 128

// RUN: execute 0x42::dce_loop_carried_param::loop_at_entry --args 1, 1
// CHECK: results: 128
