module 0xcafe::m3 {
    use 0xcafe::m2;

    public entry fun run() {
        m2::run();
    }
}
