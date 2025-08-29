module 0x42::valid_mixed_type {
    use std::i64;
    use std::i128;

    fun test_mix_i64_I64(x: i64): i64 {
        let a = i64::add(x, x);
        let b = i64::from(1u64);
        let c = i64::neg_from(1u64);
        let d = i64::abs(-100);
        let e = i64::min(x, x);
        let f = i64::max(x, x);
        let g = i64::pow(x, 1);
        let y : i64 = -10;
        y + a + b + c + d + e + f + g
    }

    fun test_mix_i128_I128(x: i128): i128 {
        let a = i128::add(x, x);
        let b = i128::from(1u128);
        let c = i128::neg_from(1u128);
        let d = i128::abs(-100);
        let e = i128::min(x, x);
        let f = i128::max(x, x);
        let g = i128::pow(x, 1);
        let y : i128 = -10;
        y + a + b + c + d + e + f + g
    }
}
