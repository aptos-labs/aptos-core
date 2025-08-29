module 0x42::invalid_arithmetic {
    use std::i64;
    use std::i128;

    fun test_add_by_ref1(x: i64): i64 {
        &x + &x
    }

    fun test_add_by_ref2(x: i128): i128 {
        &x + &x
    }

    fun test_add_by_mix1(x: i64, y: i128): i64 {
        x + y
    }

    fun test_add_by_mix2(x: i64, y: i128): i128 {
        x + y
    }

    fun test_add_by_mix3(x: i64, y: i128): i64 {
        i64::add(x, y)
    }

    fun test_add_by_mix4(x: i64, y: i128): i128 {
        i128::add(x, y)
    }

    fun test_logic_and1(x: i64, y: i64): bool {
        x && y
    }

    fun test_logic_and2(x: i128, y: i128): bool {
        x && y
    }

    fun test_logic_or1(x: i64, y: i64): bool {
        x || y
    }

    fun test_logic_or2(x: i128, y: i128): bool {
        x || y
    }

    fun test_logic_not1(x: i64): bool {
        !x
    }

    fun test_logic_not(x: i128): bool {
        !x
    }
}
