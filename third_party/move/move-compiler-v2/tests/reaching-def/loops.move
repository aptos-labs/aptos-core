module 0x42::loops {

    // Test reaching defs with a while loop
    // At loop header, x should have defs from both init and loop body
    fun while_loop(): u64 {
        let x = 0;
        let i = 0;
        while (i < 10) {
            x = x + 1;
            i = i + 1;
        };
        x
    }

    // Test loop with break - x has defs from init, loop body, and break path
    fun loop_with_break(cond: bool): u64 {
        let x = 1;
        let i = 0;
        while (i < 10) {
            if (cond) {
                x = 99;
                break
            };
            x = x + 1;
            i = i + 1;
        };
        x  // x can be 1 (loop never entered), 99 (break), or accumulated value
    }

    // Test loop with continue
    fun loop_with_continue(cond: bool): u64 {
        let x = 0;
        let i = 0;
        while (i < 10) {
            i = i + 1;
            if (cond) {
                continue
            };
            x = x + 1;
        };
        x
    }

    // Test nested loops
    fun nested_loops(): u64 {
        let sum = 0;
        let i = 0;
        while (i < 3) {
            let j = 0;
            while (j < 3) {
                sum = sum + 1;
                j = j + 1;
            };
            i = i + 1;
        };
        sum
    }
}
