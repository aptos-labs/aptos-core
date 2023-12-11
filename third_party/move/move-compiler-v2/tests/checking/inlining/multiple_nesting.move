module 0x42::mathtest {
    use 0x42::mathtest2;
    public inline fun mul_div(a: u64, b: u64, c: u64): u64 {
        mathtest2::mul_div2(c, a, b)
    }
}

module 0x42::mathtest2 {
    use 0x42::mathtest;
    public inline fun mul_div2(a: u64, b: u64, c: u64): u64 {
        mathtest::mul_div(b, a, c)
    }
}

module 0x42::test {
    use 0x42::mathtest;
    use 0x42::mathtest2;
    fun test_nested_mul_div() {
        let a = mathtest::mul_div(1, mathtest::mul_div(1, 1, 1),
                mathtest2::mul_div2(1, 1, 1));
	assert!(a == 1, 0);
    }
}
