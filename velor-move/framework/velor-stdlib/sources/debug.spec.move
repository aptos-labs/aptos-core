spec velor_std::debug {
    spec print<T>(x: &T) {
        aborts_if false;
    }

    spec print_stack_trace() {
        aborts_if false;
    }

    spec native_print(x: String) {
        pragma opaque;
        aborts_if false;
    }

    spec native_stack_trace(): String {
        pragma opaque;
        aborts_if false;
    }
}
