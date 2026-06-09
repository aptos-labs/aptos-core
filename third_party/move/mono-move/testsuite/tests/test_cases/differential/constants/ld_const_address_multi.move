// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    const A: address = @0xAAAA;
    const B: address = @0xBBBB;

    fun ret_a(): address { A }
    fun ret_b(): address { B }
}

// RUN: execute 0x1::test::ret_a
// CHECK: results: 0xaaaa

// RUN: execute 0x1::test::ret_b
// CHECK: results: 0xbbbb
