spec velor_framework::function_info {
    spec module {
        fun spec_is_identifier(s: vector<u8>): bool;
    }

    // native function
    spec check_dispatch_type_compatibility_impl(lhs: &FunctionInfo, r: &FunctionInfo): bool {
        // TODO: temporary mockup
        pragma opaque;
    }

    // native function
    spec load_function_impl(f: &FunctionInfo){
        // TODO: temporary mockup
        pragma opaque;
    }

    // native function
    spec is_identifier(s: &vector<u8>): bool {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result == spec_is_identifier(s);
    }

    spec new_function_info(
        module_signer: &signer,
        module_name: String,
        function_name: String,
    ): FunctionInfo {
        aborts_if !spec_is_identifier(string::bytes(module_name));
        aborts_if !spec_is_identifier(string::bytes(function_name));
        ensures result == FunctionInfo {
            module_address: signer::address_of(module_signer),
            module_name,
            function_name,
        };
    }

    spec new_function_info_from_address(
        module_address: address,
        module_name: String,
        function_name: String,
    ): FunctionInfo {
        aborts_if !spec_is_identifier(string::bytes(module_name));
        aborts_if !spec_is_identifier(string::bytes(function_name));
        ensures result == FunctionInfo {
            module_address,
            module_name,
            function_name,
        };
    }

    spec load_module_from_function(f: &FunctionInfo) {
        // TODO: temporary mockup
        pragma verify = false;
        pragma opaque;
    }

    spec check_dispatch_type_compatibility(
        framework_function: &FunctionInfo,
        dispatch_target: &FunctionInfo,
    ): bool {
        // TODO: temporary mockup
        pragma verify = false;
        pragma opaque;
    }
}
