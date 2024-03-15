//# publish
module 0x42::Test {
    public inline fun nested(a: u64, b: u64): u64 {
        let sum: u64 = 0;
        while (a < b) {
            a = a + 1;
            sum = sum + a;
        };
        sum
    }

    public fun other(a: u64, b: u64): u64 {
        let sum: u64 = 0;
        while (a < b) {
            a = a + 1;
            sum = nested(a, b) + sum;
        };
        sum
    }

    public fun test_shadowing() {
        let a = 1;
        let b = 4;
        let z = other(a, b);
        assert!(z == 10, z)
    }
}

//# run 0x42::Test::test_shadowing
