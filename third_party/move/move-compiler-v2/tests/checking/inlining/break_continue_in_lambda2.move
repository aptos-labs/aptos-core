module 0xc0ffee::m {

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

    fun broken() {
	break;
    }

    fun continued() {
	continue;
    }

}
