// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun small(): u128 { 42 }
    fun mid(): u128 { 1267650600228229401496703205376 }
    fun max(): u128 { 340282366920938463463374607431768211455 }
    fun add_const(a: u128): u128 { a + 1267650600228229401496703205376 }
}

// RUN: execute 0x1::test::small
// CHECK: results: 42

// RUN: execute 0x1::test::mid
// CHECK: results: 1267650600228229401496703205376

// RUN: execute 0x1::test::max
// CHECK: results: 340282366920938463463374607431768211455

// RUN: execute 0x1::test::add_const --args 1
// CHECK: results: 1267650600228229401496703205377
