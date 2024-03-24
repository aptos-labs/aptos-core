/// The `string` module defines the `String` type which represents UTF8 encoded strings.
module aptos_framework::function_info {
    use std::string::{Self, String};

    friend aptos_framework::overloadable_fungible_asset;
    #[test_only]
    friend aptos_framework::function_info_tests;

    /// String is not a valid Move identifier
    const EINVALID_IDENTIFIER: u64 = 1;
    /// Function specified in the FunctionInfo doesn't exist on chain.
    const EINVALID_FUNCTION: u64 = 2;

    /// A `String` holds a sequence of bytes which is guaranteed to be in utf8 format.
    struct FunctionInfo has copy, drop, store {
        module_address: address,
        module_name: String,
        function_name: String,
    }

    /// Creates a new function info from names.
    public fun new_function_info(
        module_address: address,
        module_name: String,
        function_name: String,
    ): FunctionInfo {
        assert!(is_identifier(string::bytes(&module_name)), EINVALID_IDENTIFIER);
        assert!(is_identifier(string::bytes(&function_name)), EINVALID_IDENTIFIER);
        FunctionInfo {
            module_address,
            module_name,
            function_name,
        }
    }

    public(friend) fun check_dispatch_type_compatibility(
        lhs: &FunctionInfo,
        rhs: &FunctionInfo,
    ): bool {
        check_dispatch_type_compatibility_impl(lhs, rhs)
    }

    native fun check_dispatch_type_compatibility_impl(lhs: &FunctionInfo, r: &FunctionInfo): bool;
    native fun is_identifier(s: &vector<u8>): bool;
}
