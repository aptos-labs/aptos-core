module 0xc0ffee::m {
    fun test() {
        let i = 0;
        loop {
            i = i + 1;
            if (i == 10) {
                break;
                i = i + 1; // unreachable
            } else {
                continue;
                i = i + 1; // unreachable
            };
            i = i + 1; // unreachable
        }
    }

}
