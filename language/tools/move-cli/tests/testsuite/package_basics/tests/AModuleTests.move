#[test_only]
module Std::AModuleTests {
    use Std::AModule;

    #[test]
    fun double_zero_zero() {
        assert!(AModule::double_except_three(0) == 0, 0)
    }

    #[test]
    #[expected_failure(abort_code = 7)]
    fun double_three() {
        AModule::double_except_three(3);
    }

    #[test]
    #[expected_failure(abort_code = 0)]
    fun double_zero_zero_wrong() {
        assert!(AModule::double_except_three(0) == 1, 0)
    }

    #[test]
    fun double_one_one() {
        assert!(AModule::double_except_three(1) == 1, 0)
    }
}
