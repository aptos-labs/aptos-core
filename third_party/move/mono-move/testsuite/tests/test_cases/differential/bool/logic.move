// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    // `!` lowers to the BoolNot micro-op.
    fun negate(b: bool): bool {
        !b
    }

    // `&&` / `||` are short-circuiting in Move: they compile to branches.
    fun both(a: bool, b: bool): bool {
        a && b
    }

    fun either(a: bool, b: bool): bool {
        a || b
    }

    // Branch directly on a boolean parameter; also checks a bool laid out
    // ahead of wider (u64) parameters.
    fun pick(c: bool, x: u64, y: u64): u64 {
        if (c) { x } else { y }
    }
}

// RUN: execute 0x1::test::negate --args true
// CHECK: results: false
// RUN: execute 0x1::test::negate --args false
// CHECK: results: true

// RUN: execute 0x1::test::both --args true, true
// CHECK: results: true
// RUN: execute 0x1::test::both --args true, false
// CHECK: results: false
// `a == false` short-circuits without evaluating `b`.
// RUN: execute 0x1::test::both --args false, true
// CHECK: results: false

// RUN: execute 0x1::test::either --args false, false
// CHECK: results: false
// RUN: execute 0x1::test::either --args false, true
// CHECK: results: true
// `a == true` short-circuits without evaluating `b`.
// RUN: execute 0x1::test::either --args true, false
// CHECK: results: true

// RUN: execute 0x1::test::pick --args true, 5, 9
// CHECK: results: 5
// RUN: execute 0x1::test::pick --args false, 5, 9
// CHECK: results: 9
