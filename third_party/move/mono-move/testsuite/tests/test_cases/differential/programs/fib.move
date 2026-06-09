// Recursive Fibonacci (exponential time).

// RUN: publish
module 0x1::fib {
    public fun fib(n: u64): u64 {
        if (n < 2) { return n };
        fib(n - 1) + fib(n - 2)
    }
}

// RUN: execute 0x1::fib::fib --args 0
// CHECK: results: 0
// RUN: execute 0x1::fib::fib --args 1
// CHECK: results: 1
// RUN: execute 0x1::fib::fib --args 2
// CHECK: results: 1
// RUN: execute 0x1::fib::fib --args 10
// CHECK: results: 55
// RUN: execute 0x1::fib::fib --args 20
// CHECK: results: 6765
