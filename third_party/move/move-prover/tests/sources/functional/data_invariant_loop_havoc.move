module 0x42::data_invariant_loop_havoc {
    struct R {
        x: u64,
    }

    spec R {
        invariant x > 0;
    }

    fun bad_straight(r: &mut R) {
        r.x = 0;
    }

    fun bad_loop(r: &mut R) {
        let i = 0;
        while (i < 1) {
            r.x = 0;
            i = i + 1;
        };
    }
}
