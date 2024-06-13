module 0xcafe::m2 {
    use 0xcafe::m1;

    public entry fun run() {
        m1::run();
    }
}
