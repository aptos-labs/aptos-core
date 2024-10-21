module 0xc0ffee::m {
    inline fun brk() {
        break;
    }

    inline fun brk2(f: |u64|) {
        f(2);
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
                brk2(|_x: u64| break);
		brk2(|_x: u64| while (true) { break });
		brk2(|_x: u64| while (true) { continue });
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
