module entry_func_failed::func {
    public entry fun init() {
        // invalid function with 0 as denominator
        1/0;
    }
}
