//# publish
module 0xc0ffee::m {
    fun apply_repeatedly(f: |u64|u64 has copy + drop, times: u64): |u64|u64 has copy + drop {
        |x| {
            for (i in 0..times) {
                x = f(x);
            };
            x
        }
    }

    public fun test(): u64 {
        apply_repeatedly(|x| x + 1, 10)(1)
    }
}
