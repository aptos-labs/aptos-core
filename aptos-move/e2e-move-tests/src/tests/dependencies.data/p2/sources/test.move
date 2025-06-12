module 0xcafe::m2 {
    use 0xcafe::m1;

    public entry fun noop() {
        // Do nothing.
    }

    public entry fun load_m1() {
        m1::noop();
    }
}
