// Cross-module lazy load + vector descriptor publish test.

// RUN: publish
module 0xc0ffee::foo {
    public fun push_and_sum(n: u64): u64 {
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

module 0xc0ffee::bar {
    public fun sum_first_n(n: u64): u64 {
        0xc0ffee::foo::push_and_sum(n)
    }
}

// RUN: execute 0xc0ffee::bar::sum_first_n --args 0
// CHECK: results: 0

// RUN: execute 0xc0ffee::bar::sum_first_n --args 10
// CHECK: results: 45

// RUN: execute 0xc0ffee::bar::sum_first_n --args 100
// CHECK: results: 4950
