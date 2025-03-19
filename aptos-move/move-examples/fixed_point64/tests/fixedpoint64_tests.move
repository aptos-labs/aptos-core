#[test_only]
module fixed_point64::fixedpoint64_tests {
    #[test]
    fun test_one_again() {
        use fixed_point64::fixed_point64;
        assert!(fixed_point64::one() == 1, 1);
    }
}
