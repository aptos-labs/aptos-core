module 0x99::cfg_opt_simple {
    fun test1(x: u64, y: u64): u64 {
       'outer: loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                x = x + 1;
                break 'outer
            };
            y = y + 1
        };
        x + y
    }

    fun test2(x: u64, y: u64): u64 {
        'outer: loop{
            loop{
                if (x > 1) break;
                if (y > 2) break;
                break 'outer
            };
            y = y + 1
        };
        x + y
    }
}
