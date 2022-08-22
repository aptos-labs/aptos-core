/// Module providing debug functionality.
module aptos_std::debug {
    native public fun print<T>(x: &T);

    native public fun print_stack_trace();
}
