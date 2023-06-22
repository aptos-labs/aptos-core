module 0x42::math64 {
    public inline fun mul_div(a: u64, b: u64, c: u64): u64 {
        (((a as u128) * (b as u128) / (c as u128)) as u64)
    }
}

module 0x42::Test {
    use 0x42::math64;
    fun test_nested_mul_div() {
        assert!(math64::mul_div(1, math64::mul_div(1, 1, 1),1) == 1, 0);
    }
}
