spec std::reflect {

    /// The VM resolver reports resolution failure as `ReflectionError` rather
    /// than aborting. Its successful function value is intentionally opaque
    /// to the prover, but the declared normal/abort boundary lets
    /// `resolve` forward that value exactly.
    spec native_resolve<FuncType>(
        addr: address,
        module_name: &0x1::string::String,
        func_name: &0x1::string::String
    ): 0x1::result::Result<FuncType, 0x1::reflect::ReflectionError> {
        pragma opaque;
        pragma aborts_if_is_partial = false;
        aborts_if false;
    }

    spec error_code(self: 0x1::reflect::ReflectionError): u64 {
        pragma opaque = true;
        ensures [inferred](self is InvalidIdentifier) ==> result == 0;
        ensures [inferred](self is FunctionNotFound) ==> result == 1;
        ensures [inferred](self is FunctionNotAccessible) ==> result == 2;
        ensures [inferred](self is FunctionIncompatibleType) ==> result == 3;
        ensures [inferred](self is FunctionNotInstantiated) ==> result == 4;
        aborts_if [inferred] false;
    }

    spec resolve<FuncType>(
        addr: address, module_name: &0x1::string::String, func_name: &0x1::string::String
    ): 0x1::result::Result<FuncType, 0x1::reflect::ReflectionError> {
        use 0x1::features;
        pragma opaque = true;
        pragma aborts_if_is_partial = false;
        aborts_if aborts_of<features::is_function_reflection_enabled>();
        aborts_if !aborts_of<features::is_function_reflection_enabled>()
            && !result_of<features::is_function_reflection_enabled>();
        aborts_if !aborts_of<features::is_function_reflection_enabled>()
            && result_of<features::is_function_reflection_enabled>()
            && aborts_of<native_resolve<FuncType>> (addr, module_name, func_name);
        ensures !aborts_of<features::is_function_reflection_enabled>()
            && result_of<features::is_function_reflection_enabled>()
            && !aborts_of<native_resolve<FuncType>> (addr, module_name, func_name) ==>
            result == result_of<native_resolve<FuncType>> (addr, module_name, func_name);
    }
}
