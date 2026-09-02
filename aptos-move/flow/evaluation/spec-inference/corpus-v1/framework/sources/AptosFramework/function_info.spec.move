spec aptos_framework::function_info {
    spec module {
        fun spec_is_identifier(s: vector<u8>): bool;
    }

    // native function
    spec check_dispatch_type_compatibility_impl(
        lhs: &FunctionInfo, r: &FunctionInfo
    ): bool {
        // TODO: temporary mockup
        pragma opaque;
    }

    // native function
    spec load_function_impl(f: &FunctionInfo) {
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
        module_signer: &signer, module_name: String, function_name: String
    ): FunctionInfo {
        use 0x1::signer;
        aborts_if !spec_is_identifier(std::string::bytes(module_name));
        aborts_if !spec_is_identifier(std::string::bytes(function_name));
        ensures result
            == FunctionInfo {
                module_address: signer::address_of(module_signer),
                module_name,
                function_name
            };
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result
            == FunctionInfo {
                module_address: signer::address_of(module_signer),
                module_name: module_name,
                function_name: function_name
            };
        aborts_if [inferred] aborts_of<new_function_info_from_address>(
            signer::address_of(module_signer), module_name, function_name
        );
    }

    spec load_function_value<FuncType>(self: &FunctionInfo): FuncType {
        use 0x1::result;
        use 0x1::reflect;
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<reflect::resolve<FuncType>> (
            self.module_address, self.module_name, self.function_name
        );
        aborts_if !aborts_of<reflect::resolve<FuncType>> (
            self.module_address, self.module_name, self.function_name
        ) && !result::is_ok<FuncType, reflect::ReflectionError>(
            result_of<reflect::resolve<FuncType>> (
                self.module_address, self.module_name, self.function_name
            )
        );
        ensures !aborts_of<reflect::resolve<FuncType>> (
            self.module_address, self.module_name, self.function_name
        ) && result::is_ok<FuncType, reflect::ReflectionError>(
            result_of<reflect::resolve<FuncType>> (
                self.module_address, self.module_name, self.function_name
            )
        ) ==>
            result
                == result_of<reflect::resolve<FuncType>> (
                    self.module_address, self.module_name, self.function_name
                ).Ok.0;
    }

    spec new_function_info_from_address(
        module_address: address, module_name: String, function_name: String
    ): FunctionInfo {
        use 0x1::string;
        aborts_if !spec_is_identifier(std::string::bytes(module_name));
        aborts_if !spec_is_identifier(std::string::bytes(function_name));
        ensures result
            == FunctionInfo { module_address, module_name, function_name };
        pragma opaque = true;
        ensures [inferred] spec_is_identifier(string::bytes(module_name))
            && spec_is_identifier(string::bytes(function_name)) ==>
            result
                == FunctionInfo {
                    module_address: module_address,
                    module_name: module_name,
                    function_name: function_name
                };
        aborts_if [inferred]!spec_is_identifier(string::bytes(module_name));
        aborts_if [inferred] spec_is_identifier(string::bytes(module_name))
            && !spec_is_identifier(string::bytes(function_name));
    }

    spec check_dispatch_type_compatibility(
        framework_function: &0x1::function_info::FunctionInfo,
        dispatch_target: &0x1::function_info::FunctionInfo
    ): bool {
        pragma opaque = true;
        ensures [inferred] result
            == (
                S1.. |~ result_of<check_dispatch_type_compatibility_impl>(
                    framework_function, dispatch_target
                )
            );
        ensures [inferred]..S1 |~(ensures_of<load_function_impl>(dispatch_target));
        aborts_if [inferred] S1 |~(
            aborts_of<check_dispatch_type_compatibility_impl>(
                framework_function, dispatch_target
            )
        );
        aborts_if [inferred] aborts_of<load_function_impl>(dispatch_target);
    }

    spec load_module_from_function(
        f: &0x1::function_info::FunctionInfo
    ) {
        pragma opaque = true;
        ensures [inferred] ensures_of<load_function_impl>(f);
        aborts_if [inferred] aborts_of<load_function_impl>(f);
    }
}
