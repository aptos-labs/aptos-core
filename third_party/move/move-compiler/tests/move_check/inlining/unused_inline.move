module 0xc0ffee::m {
    inline fun foo(): u64 {
        let i = 0;
        while (i < 10) {
            i = i + 1;
            if (i == 5) {
		break;
            }
        };
        i
    }

    public fun bar(): u64 {
        let i = 0;
        while (i < 10) {
            i = i + 1;
            if (i == 5) {
                break;
            }
        };
        i
    }
}
