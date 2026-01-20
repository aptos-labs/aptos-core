module 0xcafe::m2 {
    public entry fun entrypoint() {
        let i = 0;
        // Run for a while before m1 is loaded.
        while (i < 10000) {{
            i += 1;
        }};
        0xcafe::m1::run();
    }
}
