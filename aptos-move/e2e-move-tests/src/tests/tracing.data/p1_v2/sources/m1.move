module 0xcafe::m1 {
    public fun run() {
        let x = 2;
        let total = 0;

        let i = 0;
        // Do less iterations.
        while (i < 20) {{
            // Change to multiplication.
            total *= x;
            i += 1;
        }};
    }
}
