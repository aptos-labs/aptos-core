#[test_only]
module aptos_framework::function_info_tests {
    use aptos_framework::function_info::{Self, FunctionInfo};
    use std::signer;
    use std::string;

    public fun lhs(_s: &FunctionInfo) {}

    public fun rhs() {}

    public fun rhs2(_u: u8) {}

    public fun lhs_generic<T>(_s: &FunctionInfo) {}

    public fun rhs_generic<T>() {}

    #[test]
    fun test_valid_identifier() {
        let s1 = string::utf8(b"abcd");
        let _ = function_info::new_function_info_from_address(@0xcafe, s1, s1);
    }

    #[test]
    #[expected_failure(abort_code = 0x1, location = aptos_framework::function_info)]
    fun test_invalid_identifier() {
        let s1 = string::utf8(b"0abcd");
        let _ = function_info::new_function_info_from_address(@0xcafe, s1, s1);
    }

    #[test]
    fun test_func_type_eq() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs"));
        assert!(function_info::check_dispatch_type_compatibility(&lhs, &rhs), 0x1);
    }

    #[test]
    fun test_func_type_eq_generic() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs_generic"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs_generic"));
        assert!(function_info::check_dispatch_type_compatibility(&lhs, &rhs), 0x1);
    }

    #[test]
    #[expected_failure(abort_code = 0x1, location = Self)]
    fun test_func_type_eq_reject_same_module() {
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"rhs"));
        assert!(function_info::check_dispatch_type_compatibility(&lhs, &rhs), 0x1);
    }

    #[test]
    #[expected_failure(abort_code = 0x2, location = aptos_framework::function_info)]
    fun test_func_type_bad_lhs() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs"));

        // rhs has less than one arguments.
        assert!(function_info::check_dispatch_type_compatibility(&rhs, &lhs), 0x42);
    }

    #[test]
    #[expected_failure(abort_code = 0x42, location = aptos_framework::function_info_tests)]
    fun test_func_type_neq() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs2"));
        assert!(function_info::check_dispatch_type_compatibility(&lhs, &rhs), 0x42);
    }

    #[test]
    #[expected_failure(abort_code = 0x42, location = aptos_framework::function_info_tests)]
    fun test_func_type_neq_generic() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");
        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs_generic"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs"));
        assert!(function_info::check_dispatch_type_compatibility(&lhs, &rhs), 0x42);
    }

    #[test(alice = @0xcafe)]
    fun test_load_function_value_with_signer(alice: signer) {
        let m = string::utf8(b"function_info_tests_helpers");
        let f = function_info::new_function_info_from_address(
            @aptos_framework, m, string::utf8(b"pass_signer"));
        let hook = f.load_function_value<|signer|signer has copy + drop>();
        let returned = hook(alice);
        assert!(signer::address_of(&returned) == @0xcafe, 1);
    }

    #[test]
    #[expected_failure(abort_code = 0x2, location = aptos_framework::function_info)]
    fun test_load_function_value_incompatible_type() {
        let m = string::utf8(b"function_info_tests_helpers");
        // `rhs2` takes `u8`, not `u64`, so resolution must fail.
        let f = function_info::new_function_info_from_address(
            @aptos_framework, m, string::utf8(b"rhs2"));
        let _hook = f.load_function_value<|u64| has copy + drop>();
    }

    #[test]
    #[expected_failure(abort_code = 0x2, location = aptos_framework::function_info)]
    fun test_func_type_rhs_doesnt_exist() {
        let m = string::utf8(b"function_info_tests_helpers");
        let m2 = string::utf8(b"function_info_tests");

        let lhs = function_info::new_function_info_from_address(@aptos_framework, m2, string::utf8(b"lhs"));
        let rhs = function_info::new_function_info_from_address(@aptos_framework, m, string::utf8(b"rhs3"));

        // rhs has less than one arguments.
        assert!(function_info::check_dispatch_type_compatibility(&rhs, &lhs), 0x42);
    }
}
