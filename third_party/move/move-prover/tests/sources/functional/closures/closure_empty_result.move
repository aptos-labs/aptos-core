module 0x42::test {
    //This exercises the `invoke` arm of `translate_bytecode` with no return vars
    public fun test_unit_closure(f: |u64| ()) {
        f(0);
    }

    //This exercises the `translate_fun` function with no return vars
    public fun wrapper() {
        test_unit_closure(|_x| ())
    }
}
