module 0x42::loop_invalid {

    fun misplaced_break(x: u64): u64 {
        while (x > 0) {
            break
        };
        break;
        x
    }

    fun misplaced_continue(x: u64): u64 {
        continue;
        while (x > 0) {
            continue
        };
        x
    }
}
