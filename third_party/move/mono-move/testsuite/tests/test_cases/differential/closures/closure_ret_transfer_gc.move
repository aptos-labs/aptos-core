// RUN: publish --print(stackless,micro-ops)
module 0x42::closure_ret_transfer_gc {
    use std::vector;

    fun make(n: u64): vector<u64> {
        vector[n, n + 1]
    }

    fun combine(v: vector<u64>, w: vector<u64>): u64 {
        *vector::borrow(&v, 1) + vector::length(&w)
    }

    public fun churn(n: u64): u64 {
        let f: |u64|vector<u64> has copy + drop = make;
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = acc + combine(f(i), vector[i, i, i, i]);
            i = i + 1;
        };
        acc
    }
}

// RUN: execute 0x42::closure_ret_transfer_gc::churn --args 200 --heap-size 256
// CHECK: results: 20900
// CHECK-GC-COUNT: 199
