module 0xbeef::limits {
    public entry fun spin(n: u64) {
        let i = 0;
        while (i < n) {
            i = i + 1;
        };
    }
}
