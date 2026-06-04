// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    // Standalone address comparison → 32-byte IntCmp.
    fun aeq(a: address, b: address): bool {
        a == b
    }

    fun aneq(a: address, b: address): bool {
        a != b
    }

    // Fused `if (address == address)` → 32-byte JumpIntCmp.
    fun asel(a: address, b: address): u64 {
        if (a == b) { 10 } else { 20 }
    }
}

// RUN: execute 0x1::test::aeq --args 0x1, 0x1
// CHECK: results: true
// RUN: execute 0x1::test::aeq --args 0x1, 0x2
// CHECK: results: false
// RUN: execute 0x1::test::aeq --args 0xcafe, 0xcafe
// CHECK: results: true

// RUN: execute 0x1::test::aneq --args 0x1, 0x2
// CHECK: results: true
// RUN: execute 0x1::test::aneq --args 0x42, 0x42
// CHECK: results: false

// RUN: execute 0x1::test::asel --args 0x7, 0x7
// CHECK: results: 10
// RUN: execute 0x1::test::asel --args 0x7, 0x8
// CHECK: results: 20
