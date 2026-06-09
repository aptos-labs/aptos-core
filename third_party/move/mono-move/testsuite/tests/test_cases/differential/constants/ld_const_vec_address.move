// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const ADDRS: vector<address> = vector[@0x1, @0xCAFE, @0x0];

    fun len(): u64 {
        std::vector::length(&ADDRS)
    }

    fun addr(n: u64): address {
        *std::vector::borrow(&ADDRS, n)
    }
}

// RUN: execute 0x1::test::len
// CHECK: results: 3

// RUN: execute 0x1::test::addr --args 0
// CHECK: results: 0x1

// RUN: execute 0x1::test::addr --args 1
// CHECK: results: 0xcafe

// RUN: execute 0x1::test::addr --args 2
// CHECK: results: 0x0
