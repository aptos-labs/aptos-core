// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    // Standalone bool comparison → 1-byte IntCmp.
    fun beq(a: bool, b: bool): bool {
        a == b
    }

    fun bneq(a: bool, b: bool): bool {
        a != b
    }

    // Fused `if (bool == bool)` → JumpIntCmp on a 1-byte operand.
    fun bsel(a: bool, b: bool): u64 {
        if (a == b) { 10 } else { 20 }
    }
}

// `beq` — full truth table.
// RUN: execute 0x1::test::beq --args true, true
// CHECK: results: true
// RUN: execute 0x1::test::beq --args true, false
// CHECK: results: false
// RUN: execute 0x1::test::beq --args false, true
// CHECK: results: false
// RUN: execute 0x1::test::beq --args false, false
// CHECK: results: true

// `bneq`.
// RUN: execute 0x1::test::bneq --args true, false
// CHECK: results: true
// RUN: execute 0x1::test::bneq --args true, true
// CHECK: results: false

// `bsel` — both branch directions.
// RUN: execute 0x1::test::bsel --args true, true
// CHECK: results: 10
// RUN: execute 0x1::test::bsel --args true, false
// CHECK: results: 20
