module TestAccount::Operators {
    fun sum(x: u64, y: u64): u64 {
        x + y
    }

    fun sub(x: u64, y: u64): u64 {
        x - y
    }

    fun mul(x: u64, y: u64): u64 {
        x * y
    }

    fun mod(x: u64, y: u64): u64 {
        x % y
    }

    fun div(x: u64, y: u64): u64 {
        x / y
    }

    fun and(x: u64, y: u64): u64 {
        x & y
    }

    fun or(x: u64, y: u64): u64 {
        x | y
    }

    fun xor(x: u64, y: u64): u64 {
        x ^ y
    }

    fun lsh(x: u64, y: u8): u64 {
        x << y
    }

    fun rsh(x: u64, y: u8): u64 {
        x >> y
    }

    fun logical_or(x: bool, y: bool): bool {
        x || y
    }

    fun logical_and(x: bool, y: bool): bool {
        x && y
    }

    fun logical_not(x: bool): bool {
        !x
    }

    fun eq(x: u8, y: u8): bool {
        x == y
    }

    fun neq(x: u8, y: u8): bool {
        x != y
    }

    fun lt(x: u64, y: u64): bool {
        x < y
    }

    fun lte(x: u64, y: u64): bool {
        x <= y
    }

    fun gt(x: u64, y: u64): bool {
        x > y
    }

    fun gte(x: u64, y: u64): bool {
        x >= y
    }

    spec sum {
        ensures result == x+y;
    }

    spec sub {
        ensures result == x-y;
    }

    spec mul {
        ensures result == x*y;
    }

    spec mod {
        aborts_if y == 0;
        ensures result == x%y;
    }

    spec div {
        aborts_if y == 0;
        ensures result == x/y;
    }

    spec and {
        ensures result == int2bv(x) & int2bv(y);
    }

    spec or {
        ensures result == int2bv(x) | int2bv(y);
    }

    spec xor {
        ensures result == int2bv(x) ^ int2bv(y);
    }

    spec lsh {
        aborts_if y >= 64;
        ensures result == x<<y;
    }

    spec rsh {
        aborts_if y >= 64;
        ensures result == x>>y;
    }

    spec logical_or {
        ensures result == x || y;
    }

    spec logical_not {
        ensures result == !x;
    }

    spec eq {
        ensures result == (x == y);
    }

    spec neq {
        ensures result == (x != y);
    }

    spec lt {
        ensures result == (x < y);
    }

    spec lte {
        ensures result == (x <= y);
    }

    spec gt {
        ensures result == (x > y);
    }

    spec gte {
        ensures result == (x >= y);
    }

}
