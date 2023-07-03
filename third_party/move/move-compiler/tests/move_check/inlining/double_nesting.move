module 0x42::mathtest {
    public inline fun fun1(a: u64, b: u64, c: u64): u64 {
        (((2 * (a as u128)) + (3 * (b as u128))  + (5 * (c as u128))) as u64)
    }
}

module 0x42::mathtest2 {
    public inline fun fun2(a: u64, b: u64, c: u64): u64 {
        (((7 * (a as u128)) + (11 * (b as u128))  + (13 * (c as u128))) as u64)
    }
}

module 0x42::test {
    use 0x42::mathtest;
    use 0x42::mathtest2;

    fun test_nested_fun1() {
        let a = mathtest::fun1(2, mathtest::fun1(3, mathtest2::fun2(4, 5, 6), 7),
                        mathtest2::fun2(8, 9, mathtest::fun1(10, mathtest2::fun2(11, 12, 13), 14)));
        assert!(a == 81911, 0);
    }
}
