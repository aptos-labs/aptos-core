// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const SHORT: address = @0xCAFE;
    const LONG: address =
        @0x0123456789ABCDEFFEDCBA9876543210FEDCBA9876543210FEDCBA9876543210;
    const ZERO: address = @0x0;

    fun ret_short(): address { SHORT }
    fun ret_long(): address { LONG }
    fun ret_zero(): address { ZERO }
}

// RUN: execute 0x1::test::ret_short
// CHECK: results: 0xcafe

// RUN: execute 0x1::test::ret_long
// CHECK: results: 0x123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210

// RUN: execute 0x1::test::ret_zero
// CHECK: results: 0x0
