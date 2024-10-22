
module 0xABCD::maps_example {
    use aptos_std::ordered_map;
    use aptos_std::simple_map;

    const OFFSET: u64 = 270001;
    const MOD: u64 = 1000000;

    public entry fun test_add_remove(len: u64, repeats: u64, use_simple_map: bool) {
        // y is same sequence of values as x, just lagging len behind
        // so that map always has len elements.
        let x = 1234;
        let y = 1234;

        let simple_map = simple_map::new();
        let ordered_map = ordered_map::new();

        for(i in 0..len) {
            if (use_simple_map) {
                simple_map.add(x, x);
            } else {
                ordered_map.add(x, x);
            };

            x = x + OFFSET;
            if (x > MOD) { x = x - MOD};
            // doing plus and minus instead of something like:
            // x = (x * 92717) % 262139;
            // because multiplications and divisions become costly.
        };

        for (i in 0..repeats) {
            if (use_simple_map) {
                simple_map.add(x, x);
                simple_map.remove(&y);
            } else {
                ordered_map.add(x, x);
                ordered_map.remove(&y);
            };

            x = x + OFFSET;
            if (x > MOD) { x = x - MOD};
            y = y + OFFSET;
            if (y > MOD) { y = y - MOD};
        };
    }
}
