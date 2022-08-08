mod test_context;
mod auth_test;

/// Returns the name of the current function. This macro is used to derive the name for the golden
/// file of each test case.
#[macro_export]
macro_rules! current_function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        let mut strip = 3;
        if name.contains("::{{closure}}") {
            strip += 13;
        }
        &name[..name.len() - strip]
    }};
}
