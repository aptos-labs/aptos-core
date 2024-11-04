module 0x815::m {

    fun while_1(c: u64) {
        while (c > 0) c = c - 1;
    }

    fun while_2(c: u64): u64 {
        while (c > 0) {
            if (c >= 10) {
                c = c - 10
            }
        };
        c = c + 1;
        c
    }

    fun while_3(c: u64): u64 {
        while (c > 0) {
            while (c > 10) c = c - 10;
            c = c - 1;
        };
        c
    }

    fun loop_1(c: u64): u64 {
        loop {
            c = c + 1;
            if (c % 2 == 0) continue;
            c = c + 3;
            if (c % 2 == 1) break;
        };
        c
    }

    fun nested_loop(x: u64): u64 {
        while (x > 0) {
            while (x > 10) {
                x = x - 1;
                break;
            };
            x = x - 1;
            continue;
        };
        x
    }
}
