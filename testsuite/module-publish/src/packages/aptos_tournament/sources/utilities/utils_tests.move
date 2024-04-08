#[test_only]
module tournament::utils_tests {

    use tournament::misc_utils::rand_range;
    use tournament::test_utils;

    #[test(aptos_framework = @0x1)]
    fun test_rand_range(aptos_framework: &signer) {
        test_utils::init_test_framework(aptos_framework, 1);
        test_utils::enable_features_for_test(aptos_framework);
        let min = 1;
        let max = 6;
        let i = 0;
        let any_min = false;
        let any_max = false;
        let n: u64 = 100;
        assert!(n >= max, 0); // test will fail otherwise
        while (i < n) {
            let rand = rand_range(min, max);
            any_min = any_min || rand == min;
            any_max = any_max || rand == max;
            assert!(rand >= min, 0);
            assert!(rand <= max, 0);
            test_utils::fast_forward_microseconds(1);
            i = i + 1;
        };
        assert!(any_min, 0);
        assert!(any_max, 0);
    }
}
