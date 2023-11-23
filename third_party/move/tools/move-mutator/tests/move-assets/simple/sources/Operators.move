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
}
