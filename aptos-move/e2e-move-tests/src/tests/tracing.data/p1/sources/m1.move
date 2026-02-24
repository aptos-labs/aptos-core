module 0xcafe::m1 {
    public fun run() {
        let x = 2;
        let total = 0;

        let i = 0;
        while (i < 100) {{
            total += x;
            i += 1;
        }};
    }
}
