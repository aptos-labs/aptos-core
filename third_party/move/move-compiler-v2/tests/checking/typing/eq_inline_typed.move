module 0x42::m {

    inline fun foo(f: |&u64|) {
    }

    fun g() {
        foo(|v: &u64| {
            v == &1;
        });
    }


}
