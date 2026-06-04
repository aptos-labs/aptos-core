// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun eq(x: u64, y: u64): bool {
        x == y
    }

    fun compare(x: u64, y: u64): u64 {
        if (eq(x, y)) { 10 } else { 42 }
    }
}

// `eq` returns a 1-byte boolean directly (standalone comparison, not fused
// into a branch).
// RUN: execute 0x1::test::eq --args 3, 3
// CHECK: results: true

// RUN: execute 0x1::test::eq --args 3, 4
// CHECK: results: false

// `compare` branches on the boolean returned by a call to `eq`.
// RUN: execute 0x1::test::compare --args 7, 7
// CHECK: results: 10

// RUN: execute 0x1::test::compare --args 7, 8
// CHECK: results: 42
