module 0x42::invalid_mixed_type {
    use std::i64;
    use std::i128;

    fun test_mix_i64_I64(x: i64): i64 {
        i64::add(x, x) // while internally `i64` will be replaced with `I64`, the type system does not take them as alternatives to each other.
    }

    fun test_mix_i128_I128(x: i128): i128 {
        i128::add(x, x)
    }
}
