// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun read(r: &u64): u64 { *r }

    fun frozen_read(v: u64): u64 {
        let x = v;
        read(&mut x)
    }
}

// RUN: execute 0x1::test::frozen_read --args 0
// CHECK: results: 0

// RUN: execute 0x1::test::frozen_read --args 42
// CHECK: results: 42

// RUN: execute 0x1::test::frozen_read --args 18446744073709551615
// CHECK: results: 18446744073709551615
