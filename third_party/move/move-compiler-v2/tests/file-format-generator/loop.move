module 0x42::loops {

    fun while_loop(x: u64): u64 {
        while (x > 0) {
            x = x - 1;
        };
        x
    }

    fun while_loop_with_break_and_continue(x: u64): u64 {
        while (x > 0) {
            if (x == 42)
                break;
            if (x == 21)
                continue;
            x = x - 1;
        };
        x
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
