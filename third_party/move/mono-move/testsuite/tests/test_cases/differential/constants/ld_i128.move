// RUN: publish --print(stackless,micro-ops)
module 0x1::test {
    fun pos(): i128 { 1267650600228229401496703205376 }
    fun neg(): i128 { -1267650600228229401496703205376 }
    fun min_val(): i128 { -170141183460469231731687303715884105728 }
    fun max_val(): i128 { 170141183460469231731687303715884105727 }
    fun add_const(a: i128): i128 { a + (-1267650600228229401496703205376) }
}

// RUN: execute 0x1::test::pos
// CHECK: results: 1267650600228229401496703205376

// RUN: execute 0x1::test::neg
// CHECK: results: -1267650600228229401496703205376

// RUN: execute 0x1::test::min_val
// CHECK: results: -170141183460469231731687303715884105728

// RUN: execute 0x1::test::max_val
// CHECK: results: 170141183460469231731687303715884105727

// RUN: execute 0x1::test::add_const --args 1267650600228229401496703205376
// CHECK: results: 0
