// `&&` and `||` skip the second operand when the first determines the result.
// The call counter detects evaluation of a skipped operand.

// RUN: publish
module 0x1::test {
    fun side(n: u64, calls: &mut u64): bool {
        *calls = *calls + 1;
        n > 0
    }

    // `true || _`: second call must be skipped.
    fun or_short(): u64 {
        let calls = 0;
        let r = side(1, &mut calls) || side(2, &mut calls);
        calls * 10 + (if (r) { 1 } else { 0 })
    }

    // `false || f()`: second call must run.
    fun or_long(): u64 {
        let calls = 0;
        let r = side(0, &mut calls) || side(2, &mut calls);
        calls * 10 + (if (r) { 1 } else { 0 })
    }

    // `false && _`: second call must be skipped.
    fun and_short(): u64 {
        let calls = 0;
        let r = side(0, &mut calls) && side(2, &mut calls);
        calls * 10 + (if (r) { 1 } else { 0 })
    }

    // `true && f()`: second call must run.
    fun and_long(): u64 {
        let calls = 0;
        let r = side(1, &mut calls) && side(2, &mut calls);
        calls * 10 + (if (r) { 1 } else { 0 })
    }

    // The nested expression skips side(2) and evaluates side(3).
    fun mixed(): u64 {
        let calls = 0;
        let r = (side(1, &mut calls) || side(2, &mut calls)) && side(3, &mut calls);
        calls * 10 + (if (r) { 1 } else { 0 })
    }
}

// RUN: execute 0x1::test::or_short
// CHECK: results: 11

// RUN: execute 0x1::test::or_long
// CHECK: results: 21

// RUN: execute 0x1::test::and_short
// CHECK: results: 10

// RUN: execute 0x1::test::and_long
// CHECK: results: 21

// RUN: execute 0x1::test::mixed
// CHECK: results: 21
