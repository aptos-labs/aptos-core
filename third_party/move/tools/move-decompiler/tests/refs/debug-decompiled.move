module 0x1::debug {
    native fun native_print(arg0: 0x1::string::String);
    native fun native_stack_trace() : 0x1::string::String;
    public fun print<T0>(arg0: &T0) {
        native_print(0x1::string_utils::debug_string<T0>(arg0));
    }
    
    public fun print_stack_trace() {
        native_print(native_stack_trace());
    }
    
    // decompiled from Move bytecode v6
}
