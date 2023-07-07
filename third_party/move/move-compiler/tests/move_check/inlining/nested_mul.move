module 0x42::mathtest {
    public inline fun mul_div(a: u64, b: u64, c: u64): u64 {
        (((a as u128) * (b as u128) / (c as u128)) as u64)
    }
}

module 0x42::test {
    use 0x42::mathtest;
    fun test_nested_mul_div() {
        let a = mathtest::mul_div(1, mathtest::mul_div(1, 1, 1),1);
	assert!(a == 1, 0);
    }
}
