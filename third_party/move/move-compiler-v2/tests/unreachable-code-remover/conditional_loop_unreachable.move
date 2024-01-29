module 0xc0ffee::m {
    fun test(p: bool, q: bool) {
        while (p) {
            if (q) {
                loop {};
                let i = 0;
                i = i + 1;
            } else {
                break;
            };
            let i = 0;
            i = i + 1;
        }
    }

}
