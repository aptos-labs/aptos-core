module 0x42::m {

    inline fun foo(f: |&u64|) {
    }

    fun g() {
        foo(|v| {
            v == &1;
        });
    }


}
