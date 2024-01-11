module 0xc0ffee::m {
    inline fun brk() {
        break;
    }

    inline fun brk2(f: | |) {
        f();
    }

    inline fun brk3() {
	while (true) {
            break;
	}
    }

    inline fun brk4() {
	while (true) {
            continue;
	}
    }

    public fun foo(): u64 {
        let i = 0;
        while (i < 10) {
            i = i + 1;
            if (i == 5) {
                brk();
		brk3();
		brk4();
            }
        };
        i
    }

    public fun bar(): u64 {
        let i = 0;
        while (i < 10) {
            i = i + 1;
            if (i == 5) {
                brk2(| | break);
		brk2(| | while (true) { break });
		brk2(| | while (true) { continue });
            }
        };
        i
    }

    fun broken() {
	break;
    }

    fun continued() {
	continue;
    }
}
