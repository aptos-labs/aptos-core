// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun double(x: u64): u64 {
        x + x
    }

    fun quad(x: u64): u64 {
        double(double(x))
    }
}

module 0x1::utils {
    public fun triple(x: u64): u64 {
        x + x + x
    }
}

// RUN: execute 0x1::test::quad --args 0
// CHECK: results: 0

// RUN: execute 0x1::test::quad --args 1
// CHECK: results: 4

// RUN: execute 0x1::test::quad --args 3
// CHECK: results: 12

// RUN: execute 0x1::test::quad --args 10
// CHECK: results: 40

// RUN: execute 0x1::utils::triple --args 5
// CHECK: results: 15

// RUN: execute 0x1::utils::triple --args 3
// CHECK: results: 9

// RUN: execute 0x1::test::double --args 7
// CHECK: results: 14
