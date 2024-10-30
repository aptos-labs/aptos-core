module publisher::m2 {
    use publisher::m1;

    public entry fun run() {
        m1::run();
    }
}
