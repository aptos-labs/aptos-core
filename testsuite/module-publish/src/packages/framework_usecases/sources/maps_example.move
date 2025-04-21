
module 0xABCD::maps_example {
    use aptos_std::big_ordered_map;
    use aptos_std::ordered_map;
    use aptos_std::simple_map;

    const OFFSET: u64 = 270001;
    const MOD: u64 = 1000000;

    struct SimpleMapResource has key {
        value: simple_map::SimpleMap<u64, u64>,
    }

    struct OrderedMapResource has key {
        value: ordered_map::OrderedMap<u64, u64>,
    }

    struct BigOrderedMapResource has key {
        value: big_ordered_map::BigOrderedMap<u64, u64>,
    }

    inline fun do_test_add_remove(len: u64, repeats: u64, add: |u64|, remove: |u64|) {
        // y is same sequence of values as x, just lagging len behind
        // so that map always has len elements.
        let x = 1234;
        let y = 1234;

        for(i in 0..len) {
            add(x);

            x = x + OFFSET;
            if (x > MOD) { x = x - MOD};
            // doing plus and minus instead of something like:
            // x = (x * 92717) % 262139;
            // because multiplications and divisions become costly.
        };

        for (i in 0..repeats) {
            add(x);
            remove(y);

            x = x + OFFSET;
            if (x > MOD) { x = x - MOD};
            y = y + OFFSET;
            if (y > MOD) { y = y - MOD};
        };
    }

    public entry fun test_add_remove_simple_map(sender: &signer, len: u64, repeats: u64) {
        let map = simple_map::new();
        do_test_add_remove(
            len,
            repeats,
            |x| { map.add(x, x); },
            |x| { map.remove(&x); },
        );
        move_to(sender, SimpleMapResource { value: map });
    }

    public entry fun test_add_remove_ordered_map(sender: &signer, len: u64, repeats: u64) {
        let map = ordered_map::new();
        do_test_add_remove(
            len,
            repeats,
            |x| { map.add(x, x); },
            |x| { map.remove(&x); },
        );
        move_to(sender, OrderedMapResource { value: map });
    }

    public entry fun test_add_remove_big_ordered_map(sender: &signer, len: u64, repeats: u64, inner_max_degree: u16, leaf_max_degree: u16) {
        let map = big_ordered_map::new_with_config(inner_max_degree, leaf_max_degree, false);
        do_test_add_remove(
            len,
            repeats,
            |x| { map.add(x, x); },
            |x| { map.remove(&x); },
        );
        move_to(sender, BigOrderedMapResource { value: map });
    }
}
