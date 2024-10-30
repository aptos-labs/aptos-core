module publisher::m3 {
    use publisher::m2;

    public entry fun run() {
        m2::run();
    }
}
