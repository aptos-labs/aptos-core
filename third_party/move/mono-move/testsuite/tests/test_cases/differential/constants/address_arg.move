// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun identity(a: address): address { a }
}

// RUN: execute 0x1::test::identity --args 0xcafe
// CHECK: results: 0xcafe

// RUN: execute 0x1::test::identity --args 0x0
// CHECK: results: 0x0

// RUN: execute 0x1::test::identity --args 0x0123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210
// CHECK: results: 0x123456789abcdeffedcba9876543210fedcba9876543210fedcba9876543210
