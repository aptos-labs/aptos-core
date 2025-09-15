module 0xcafe::m3 {
    use 0xcafe::m2;

    public entry fun noop() {
        // Do nothing.
    }

    public entry fun load_m2_m1() {
        m2::load_m1();
    }
}
