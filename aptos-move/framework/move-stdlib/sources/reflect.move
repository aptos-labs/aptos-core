/// Functionality for reflection in Move.
module std::reflect {
    use std::error;
    use std::features;
    use std::result::Result;
    use std::string::String;

    /// This error indicates that the reflection feature is not enabled.
    const EFEATURE_NOT_ENABLED: u64 = 0;

    /// Resolves a function specified by address and symbolic name, with expected type, into a typed function value.
    ///
    /// Example usage:
    ///
    /// ```
    ///    let fn : |address|u64 has store = reflect::resolve(@somewhere, utf8(b"mod"), utf8(b"fn")).unwrap();
    ///    assert!(fn(my_addr) == some_value)
    /// ```
    ///
    /// See `ReflectionError` for the possible errors which can result. On successful resolution,
    /// a function value is returned which can be safely used in future executions as indicated by the requested
    /// type.
    ///
    /// In order to be accessible, the resolved function must be public. This prevents reflection to
    /// work around the languages modular encapsulation guarantees.
    ///
    /// The resolved function can be generic, in which case the instantiation must be inferrible
    /// from the provided `FuncType`. For example, `public fun foo<T>(T)`, with `FunType = |u64|`,
    /// `T = u64` can be derived. If not all type parameters can be inferred, an error will be
    /// produced.
    public fun resolve<FuncType>(
        addr: address, module_name: &String, func_name: &String
    ): Result<FuncType, ReflectionError> {
        assert!(
            features::is_function_reflection_enabled(),
            error::invalid_state(EFEATURE_NOT_ENABLED)
        );
        native_resolve(addr, module_name, func_name)
    }

    /// Represents errors returned by the reflection API.
    /// TODO: make this public once language version 2.4 is available
    enum ReflectionError has copy, drop, store {
        InvalidIdentifier,
        FunctionNotFound,
        FunctionNotAccessible,
        FunctionIncompatibleType,
        FunctionNotInstantiated
    }

    /// Returns numerical code associated with error.
    public fun error_code(self: ReflectionError): u64 {
        match(self) {
            InvalidIdentifier => 0,
            FunctionNotFound => 1,
            FunctionNotAccessible => 2,
            FunctionIncompatibleType => 3,
            FunctionNotInstantiated => 4
        }
    }

    native fun native_resolve<FuncType>(
        addr: address, module_name: &String, func_name: &String
    ): Result<FuncType, ReflectionError>;
}
